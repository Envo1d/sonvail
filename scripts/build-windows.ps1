param(
    [string]$SigningKeyPath,
    [string]$ReleaseNotesPath
)

$ErrorActionPreference = "Stop"
$repositoryPath = Split-Path -Parent $PSScriptRoot
$previousLocation = Get-Location
$previousKeyPath = $env:SONVAIL_SIGNING_PRIVATE_KEY_PATH

try {
    Set-Location -LiteralPath $repositoryPath
    node scripts/validate-release.mjs
    if ($LASTEXITCODE -ne 0) { throw "Release version validation failed" }
    if (-not $env:SONVAIL_UPDATER_PUBLIC_KEY) { throw "Set SONVAIL_UPDATER_PUBLIC_KEY before building." }
    if (-not $env:SONVAIL_SIGNING_PRIVATE_KEY) {
        if (-not $SigningKeyPath -or -not (Test-Path -LiteralPath $SigningKeyPath -PathType Leaf)) {
            throw "Supply -SigningKeyPath or SONVAIL_SIGNING_PRIVATE_KEY."
        }
        $env:SONVAIL_SIGNING_PRIVATE_KEY_PATH = (Resolve-Path -LiteralPath $SigningKeyPath).Path
    }
    # Always compile with the selected public key; never package an unrelated old binary.
    cargo build --release --locked --bin audio-player --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "Windows application build failed" }

    $versionLine = Select-String -LiteralPath Cargo.toml -Pattern '^version = "(\d+\.\d+\.\d+)"$' | Select-Object -First 1
    $releaseVersion = $versionLine.Matches[0].Groups[1].Value
    # Unique staging/output directories prevent stale files from entering a release.
    $stagingPath = Join-Path $repositoryPath ("target\velopack-stage\" + [guid]::NewGuid())
    $outputPath = Join-Path $repositoryPath "target\release\bundle\velopack\$releaseVersion"
    if (Test-Path -LiteralPath $outputPath) { throw "Release output already exists. Move it aside before repackaging: $outputPath" }
    New-Item -ItemType Directory -Path $stagingPath -Force | Out-Null
    Copy-Item -LiteralPath "target\x86_64-pc-windows-msvc\release\audio-player.exe" -Destination (Join-Path $stagingPath "audio-player.exe")

    $packArguments = @(
        "pack", "--packId", "Sonvail", "--packVersion", $releaseVersion,
        "--packDir", $stagingPath, "--mainExe", "audio-player.exe",
        "--packTitle", "Sonvail", "--packAuthors", "Envo1d",
        "--runtime", "win-x64", "--channel", "win-x64", "--delta", "None",
        "--icon", (Join-Path $repositoryPath "assets\icons\sonvail.ico"),
        "--outputDir", $outputPath
    )
    if ($ReleaseNotesPath) {
        $packArguments += @("--releaseNotes", (Resolve-Path -LiteralPath $ReleaseNotesPath).Path)
    }
    & "$PSScriptRoot\velopack.ps1" @packArguments
    node scripts/updater-signing.mjs sign (Join-Path $outputPath "releases.win-x64.json")
    if ($LASTEXITCODE -ne 0) { throw "Update feed signing failed" }
    node scripts/validate-velopack.mjs $outputPath
    if ($LASTEXITCODE -ne 0) { throw "Velopack artifact validation failed" }
    Write-Host "Sonvail setup and signed update feed: $outputPath"
} finally {
    $env:SONVAIL_SIGNING_PRIVATE_KEY_PATH = $previousKeyPath
    Set-Location -LiteralPath $previousLocation.Path
}
