import { readFileSync } from "node:fs";

export function readManifest() {
  const packageSection = readFileSync("Cargo.toml", "utf8")
    .match(/^\[package\]\s*\r?\n([\s\S]*?)(?=^\[|$(?![\s\S]))/m)?.[1];
  const version = packageSection?.match(/^version\s*=\s*"(\d+\.\d+\.\d+)"\s*$/m)?.[1];
  const name = packageSection?.match(/^name\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (name !== "audio-player" || !version || version.split(".").some((part) => part.length > 1 && part[0] === "0")) {
    throw new Error("Expected audio-player with a stable x.y.z version in Cargo.toml.");
  }
  return { name, version };
}
