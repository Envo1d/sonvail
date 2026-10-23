<div align="center">
  <img width="120" src="assets/icons/sonvail-256.png" alt="Sonvail logo">
  <h1>Sonvail</h1>
  <p>A Windows desktop music player for your local collection.</p>
  <p>
    <img alt="Status" src="https://img.shields.io/badge/status-in%20development-blueviolet">
    <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-0078d4">
    <a href="LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-green"></a>
    <img alt="Built with Rust and GPUI" src="https://img.shields.io/badge/built%20with-Rust%20%2B%20GPUI-orange">
  </p>
</div>

Sonvail brings your local music, metadata and playback queue into one desktop workspace. Built with **Rust, GPUI and rodio**, it stores library records in SQLite and keeps them synchronized with your music folders through a filesystem watcher.

## Features

- **Local music library:** add multiple folders and discover MP3, WAV, FLAC and OGG files recursively. Overlapping folders do not duplicate tracks.
- **Automatic indexing:** load saved tracks immediately, reconcile folders at startup and update affected folders when files are added, changed, renamed or removed.
- **Rich metadata:** read track titles, multiple artists, albums, album artists, release dates, genres, composers, track/disc numbers, duration and audio properties. Files without readable tags use filename fallbacks.
- **Embedded artwork:** display album covers in Library, Now Playing and the queue, with cached full-size images and small thumbnails.
- **Playback controls:** play, pause, resume, stop, skip, seek by clicking the progress bar, adjust volume and mute. Shuffle and Off/All/One repeat modes are available.
- **Playback queue:** select tracks from Library or the current queue, follow the active track in Now Playing and preview upcoming tracks in Queue and Up Next.
- **Background audio processing:** bounded PCM buffering, file decoding outside the device callback, unreadable-track skipping and audio-output recovery after stream errors.
- **Desktop appearance:** Dark, Light and System themes, translucent Acrylic panels, a custom window titlebar and DPI-aware application icons.
- **System tray:** close the window to keep music playing in the background. Click the Sonvail tray icon to reopen it, or choose **Exit** from its menu to quit.
- **Persistent settings and diagnostics:** automatically save the theme and source folders, retain indexed records for temporarily unavailable folders and write daily diagnostic logs.
- **Signed Windows updates:** check for releases in Settings, download with progress and install with a restart through Velopack. Package updates require an initial Setup installation and configured release signing.

Sonvail is under active development. Search, favorites, persistent playlists and the separate Albums/Artists pages are not connected yet. Gapless playback, crossfade, ReplayGain, media keys and output-device selection are also pending. Audio-device behavior and visual layout require manual testing on Windows.

## Getting Started

When a Windows release is available on [GitHub Releases](https://github.com/Envo1d/sonvail/releases), download and install `Sonvail-win-x64-Setup.exe`. Installing through Setup enables package updates from Settings. You can also run the player from source using the development instructions below.

1. Run Sonvail and open **Settings**.
2. Use **Add folders** to select one or more music folders.
3. Open **Library** and let the initial indexing finish. Later filesystem changes are detected automatically.
4. Click a track to start playback and create a queue from the current library snapshot.
5. Open **Now Playing** to follow the current track, browse the queue and use playback controls. Click **…** to reveal additional track details.

Selecting a new track in Library creates a new queue. New files appear in Library automatically; the membership of an already playing queue stays unchanged. Tag changes update matching queued tracks without restarting playback.

## Development

Windows development requires a Rust toolchain supporting **edition 2024**, the MSVC C++ Build Tools and the Windows SDK. GPUI is pinned to **0.2.2**; dependency versions are recorded in `Cargo.lock`. Python and Pillow are only needed when regenerating the icon exports.

```powershell
git clone https://github.com/Envo1d/sonvail.git
cd sonvail
cargo run --locked
```

The Cargo package and application binary are named `audio-player`.

### Validation

```powershell
cargo test --locked
cargo fmt --check
cargo clippy --all-targets --locked
```

Automated tests use temporary files and databases, synthetic audio and a fake output device. They do not launch the GUI or play audio through a real device. Manually verify playback, pause/resume, skipping, stop, seeking, repeat/shuffle, empty queues, device reconnection and window behavior.

### Build a Windows Executable

Build the player for Windows x64:

```powershell
cargo build --release --locked --bin audio-player --target x86_64-pc-windows-msvc
```

The executable is written to `target/x86_64-pc-windows-msvc/release/audio-player.exe`. This produces a standalone executable; use the packaging instructions below to create Setup and update packages.

GPUI's Windows release build needs the Windows SDK shader compiler, `fxc.exe`. If it is not discovered automatically, set `GPUI_FXC_PATH` to the compiler from your installed SDK. For example, this locates it in the standard Windows SDK directory:

```powershell
$compiler = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\fxc.exe' |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $compiler) { throw 'Install the Windows SDK with fxc.exe first.' }
$env:GPUI_FXC_PATH = $compiler.FullName
```

Then repeat the build command.

### Package a Windows Release

In addition to the Rust/MSVC and Windows SDK requirements, packaging needs **Node.js 24**, **.NET 8** and a dedicated Sonvail Ed25519 signing key. Node.js and .NET are packaging tools; users do not need them to run Sonvail.

Keep signing keys outside the repository. For first-time release setup, generate a pair in a private directory. This example uses the current user's profile without embedding a machine-specific path:

```powershell
$signingDirectory = Join-Path $env:USERPROFILE '.sonvail-signing'
New-Item -ItemType Directory -Path $signingDirectory -Force | Out-Null
node scripts/updater-signing.mjs generate (Join-Path $signingDirectory 'updater.pem')
```

The command creates `updater.pem` (private key) and `updater.pem.pub` (public key), and refuses to overwrite existing files. Back up the private key securely and reuse the same pair for subsequent releases so installed versions can verify future updates.

To package with an existing key pair, set the path to the private key and load its public companion before building:

```powershell
$signingKeyPath = Join-Path $env:USERPROFILE '.sonvail-signing\updater.pem'
$env:SONVAIL_UPDATER_PUBLIC_KEY = (Get-Content "$signingKeyPath.pub" -Raw).Trim()
./scripts/build-windows.ps1 -SigningKeyPath $signingKeyPath
```

Adjust `$signingKeyPath` if the key is stored elsewhere. The script compiles the application with the public key, downloads a pinned Velopack packaging tool with checksum verification, creates Setup and a full update package, signs the release feed and validates the output. The private key is used only for signing; it is not embedded in the application.

Artifacts are written to `target/release/bundle/velopack/<version>/`:

- `Sonvail-win-x64-Setup.exe`
- `Sonvail-<version>-win-x64-full.nupkg`
- `releases.win-x64.json`
- `releases.win-x64.json.sig`

The version comes from `Cargo.toml` and must match `Cargo.lock`. Pass `-ReleaseNotesPath <markdown-file>` to include release notes in the update. The script refuses to reuse an existing output directory for the same version.

### Publish Signed Releases

The Windows release workflow runs on pushes to `main` or a manual run on `main`. Configure these repository settings under **Settings → Secrets and variables → Actions**:

| Kind | Name | Value |
| --- | --- | --- |
| Variable | `SONVAIL_UPDATER_PUBLIC_KEY` | Contents of the `.pub` file |
| Secret | `SONVAIL_SIGNING_PRIVATE_KEY` | Full private PEM contents, including line breaks |

Set a new stable `x.y.z` version in `Cargo.toml`, update `Cargo.lock`, and push the committed changes to `main`. The workflow skips versions that are already published or are not higher than the latest stable release. It runs tests, formatting and Clippy, builds the Windows packages, creates a tag on the built commit and uploads all four artifacts to a draft before publishing it as the latest release. Existing tags are never moved.

Keep private keys out of source control, release attachments and issue reports. Installer packaging, the workflow and end-to-end updates still require manual Windows verification.

## Updating Sonvail

1. Open **Settings → Updates** and select **Check for updates**.
2. Review the available version and release notes.
3. Select **Install and restart**. Sonvail downloads the package, verifies its signature-backed size and SHA-256, then stops playback and exits before the updater replaces application files and restarts the player.

Your music, settings, library database and artwork cache stay in their existing locations. Download or verification errors leave the player open and allow a retry. Updates require an initial installation through Sonvail Setup and a build containing the trusted public key; standalone development executables cannot use package updates.

## Local Data

Sonvail keeps your music files in their original folders. Removing a source folder from Settings removes its library records without deleting your music.

For a new Windows installation, application data is stored in:

| Data | Location |
| --- | --- |
| Settings | `%APPDATA%\Envo1d\Sonvail\config\settings.json` |
| Library database | `%LOCALAPPDATA%\Envo1d\Sonvail\data\library.sqlite3` |
| Artwork cache | `%LOCALAPPDATA%\Envo1d\Sonvail\data\artwork` |
| Logs | `%LOCALAPPDATA%\Envo1d\Sonvail\data\logs` |

Logs are named `Sonvail.YYYY-MM-DD.log`. Use `RUST_LOG` to adjust diagnostic verbosity, for example:

```powershell
$env:RUST_LOG = 'audio_player=debug'
cargo run --locked
```

## Feedback

Report bugs and request features through **[GitHub Issues](https://github.com/Envo1d/sonvail/issues)**. Include the application version, Windows version and steps to reproduce the issue. For playback or indexing problems, include the file format and relevant log messages.

## License

Sonvail is distributed under the **[MIT License](LICENSE)**. Copyright (c) 2026 Envo1d.
