import { readFileSync } from "node:fs";
import { readManifest } from "./release-manifest.mjs";

const { name, version } = readManifest();
const packages = readFileSync("Cargo.lock", "utf8").split("[[package]]");
if (!packages.some((entry) => entry.includes(`name = "${name}"`) && entry.includes(`version = "${version}"`))) {
  throw new Error("Cargo.toml and Cargo.lock versions disagree. Update the lockfile before releasing.");
}
console.log(`Release version validated: ${version}`);
