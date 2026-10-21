// Ed25519 / prehashed minisign envelopes, verified by minisign-verify in Rust.
// Private keys belong outside the repository; only the public envelope is embedded.
import { createHash, createPrivateKey, createPublicKey, generateKeyPairSync, sign } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { verifyUpdateSignature } from "./verify-update-signature.mjs";

const encode = (bytes) => Buffer.from(bytes).toString("base64");

function keyEnvelope(publicKey) {
  const raw = publicKey.export({ format: "der", type: "spki" }).subarray(-32);
  const id = createHash("sha256").update(raw).digest().subarray(0, 8);
  return { raw, id, encoded: encode(`untrusted comment: Sonvail updater public key\n${encode(Buffer.concat([Buffer.from("Ed"), id, raw]))}\n`) };
}

export function signFeed(bytes, pem, filename) {
  const privateKey = createPrivateKey(pem);
  if (privateKey.asymmetricKeyType !== "ed25519") throw new Error("Expected an Ed25519 signing key.");
  const key = keyEnvelope(createPublicKey(privateKey));
  const rawSignature = sign(null, createHash("blake2b512").update(bytes).digest(), privateKey);
  const trusted = `timestamp:${Math.floor(Date.now() / 1000)} file:${filename}`;
  const globalSignature = sign(null, Buffer.concat([rawSignature, Buffer.from(trusted)]), privateKey);
  const signature = encode(`untrusted comment: Sonvail update feed\n${encode(Buffer.concat([Buffer.from("ED"), key.id, rawSignature]))}\ntrusted comment: ${trusted}\n${encode(globalSignature)}\n`);
  return { signature, publicKey: key.encoded };
}

function main() {
  const [mode, path] = process.argv.slice(2);
  if (!path) throw new Error("Usage: node scripts/updater-signing.mjs generate <private-key-path> | sign <feed-path>");
  if (mode === "generate") {
    const { publicKey, privateKey } = generateKeyPairSync("ed25519");
    const encoded = keyEnvelope(publicKey).encoded;
    writeFileSync(path, privateKey.export({ format: "pem", type: "pkcs8" }), { flag: "wx", mode: 0o600 });
    writeFileSync(`${path}.pub`, `${encoded}\n`, { flag: "wx" });
    console.log(`Private key written to ${path}. Store it securely as SONVAIL_SIGNING_PRIVATE_KEY.\nPublic key: ${path}.pub (SONVAIL_UPDATER_PUBLIC_KEY).`);
  } else if (mode === "sign") {
    const pem = process.env.SONVAIL_SIGNING_PRIVATE_KEY ?? readFileSync(process.env.SONVAIL_SIGNING_PRIVATE_KEY_PATH, "utf8");
    const bytes = readFileSync(path);
    const { signature } = signFeed(bytes, pem, basename(path));
    // Refuse mismatched signing/embedded keys before writing a publishable feed.
    verifyUpdateSignature(bytes, signature, process.env.SONVAIL_UPDATER_PUBLIC_KEY ?? "");
    writeFileSync(`${path}.sig`, `${signature}\n`);
  } else {
    throw new Error("Unknown signing command.");
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
