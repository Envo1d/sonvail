import { createHash } from "node:crypto";
import { readFileSync, statSync } from "node:fs";
import { resolve, join } from "node:path";

import { verifyUpdateSignature } from "./verify-update-signature.mjs";

import { readManifest } from "./release-manifest.mjs";

const directory = resolve(process.argv[2]);
const expectedVersion = readManifest().version;
const feedBytes = readFileSync(join(directory, "releases.win-x64.json"));
const signature = readFileSync(join(directory, "releases.win-x64.json.sig"), "utf8");
const publicKey = process.env.SONVAIL_UPDATER_PUBLIC_KEY ?? "";

verifyUpdateSignature(feedBytes, signature, publicKey);

const feed = JSON.parse(feedBytes.toString("utf8"));
const entries = feed.Assets ?? feed.assets;

if (!Array.isArray(entries) || entries.length !== 1) {
  throw new Error("Publish exactly one full package per release.");
}

for (const entry of entries) {
  const asset = Object.fromEntries(Object.entries(entry).map(([key, value]) => [key.toLowerCase(), value]));

  if (asset.packageid !== "Sonvail" || asset.version !== expectedVersion || asset.type !== "Full") {
    throw new Error("Velopack identity, version or package type mismatch.");
  }

  if (asset.filename !== `Sonvail-${expectedVersion}-win-x64-full.nupkg`) {
    throw new Error("Unsafe package filename.");
  }

  const bytes = readFileSync(join(directory, asset.filename));
  const checksum = createHash("sha256").update(bytes).digest("hex");

  if (bytes.length !== asset.size || checksum !== asset.sha256?.toLowerCase()) {
    throw new Error("Package size or SHA-256 does not match feed.");
  }
}

const setup = statSync(join(directory, "Sonvail-win-x64-Setup.exe"));
if (!setup.isFile() || setup.size === 0) {
  throw new Error("Missing Velopack Setup.exe.");
}

console.log(`Velopack artifacts validated: ${expectedVersion}`);
