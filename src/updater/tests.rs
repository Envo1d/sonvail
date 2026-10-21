use std::fs;

use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use velopack::VelopackAsset;

use super::source;

fn asset(bytes: &[u8]) -> VelopackAsset {
    VelopackAsset {
        PackageId: "Sonvail".into(),
        Version: "0.0.2".into(),
        Type: "Full".into(),
        FileName: "Sonvail-0.0.2-win-x64-full.nupkg".into(),
        SHA256: format!("{:x}", Sha256::digest(bytes)),
        Size: bytes.len() as u64,
        ..Default::default()
    }
}

#[test]
fn verifies_golden_signature_and_rejects_tampered_feed_or_key() {
    // Nexa's public golden fixture tests only the common cryptographic format.
    // This test key is never a trust anchor for Sonvail's production feed.
    let json = String::from_utf8(
        STANDARD
            .decode(include_str!("fixtures/feed.b64").trim())
            .unwrap(),
    )
    .unwrap();
    let signature = include_str!("fixtures/feed.sig");
    let key = include_str!("fixtures/test.pub");
    assert!(source::verify_signature(&json, signature, key).is_ok());
    assert!(source::verify_signature(&(json.clone() + " "), signature, key).is_err());
    assert!(source::verify_signature(&json, "invalid", key).is_err());
    assert!(source::verify_signature(&json, signature, "").is_err());
    assert!(source::verify_feed(&json, "invalid").is_err());
}

#[test]
fn rejects_foreign_unsafe_and_unverifiable_packages() {
    let valid = asset(b"package");
    assert!(source::validate_asset(&valid).is_ok());
    for name in [
        "../Sonvail-full.nupkg",
        "C:\\Sonvail-full.nupkg",
        "Sonvail-0.0.2-win-x64-delta.nupkg",
        "Other-0.0.2-win-x64-full.nupkg",
    ] {
        let mut invalid = valid.clone();
        invalid.FileName = name.into();
        assert!(source::validate_asset(&invalid).is_err());
    }
    for version in [
        "../other",
        "...",
        "1.2",
        "01.2.3",
        "1.2.3-beta",
        "1.2.999999999999999999999999",
    ] {
        let mut invalid = valid.clone();
        invalid.Version = version.into();
        invalid.FileName = format!("Sonvail-{version}-win-x64-full.nupkg");
        assert!(source::validate_asset(&invalid).is_err());
    }
    for mutate in [
        |a: &mut VelopackAsset| a.SHA256.clear(),
        |a: &mut VelopackAsset| a.PackageId = "Nexa".into(),
        |a: &mut VelopackAsset| a.Type = "Delta".into(),
        |a: &mut VelopackAsset| a.Size = 0,
    ] {
        let mut invalid = valid.clone();
        mutate(&mut invalid);
        assert!(source::validate_asset(&invalid).is_err());
    }
}

#[test]
fn verifies_cached_bytes_and_rejects_same_size_tampering() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("package.nupkg");
    let release = asset(b"original");
    fs::write(&path, b"original").unwrap();
    assert!(source::verify_package(&path, &release).is_ok());
    fs::write(&path, b"modified").unwrap();
    assert!(source::verify_package(&path, &release).is_err());
    fs::write(&path, b"short").unwrap();
    assert!(source::verify_package(&path, &release).is_err());
}
