// Cross-language compatibility: the fixture manifest.json/manifest.sig were
// produced by the Go `mtb-manifest sign` tool using the development key pair.
// This test proves the Rust verifier accepts the exact Go-produced bytes.

use mutsukitube_plugin_updater::{
    DEVELOPMENT_PLUGIN_PUBLIC_KEY, artifact_download_url, select_artifacts, verify_manifest,
};
use std::path::Path;

const MANIFEST: &[u8] = include_bytes!("fixtures/manifest.json");

fn read_signature() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/manifest.sig"),
    )
    .unwrap()
    .trim()
    .to_string()
}

#[test]
fn go_signed_manifest_verifies_in_rust() {
    let signature = read_signature();
    let manifest = verify_manifest(MANIFEST, &signature, DEVELOPMENT_PLUGIN_PUBLIC_KEY).unwrap();
    assert_eq!(manifest.version, "v0.1.0");
    assert_eq!(manifest.schema, "mutsukitube-manifest-1");
    assert!(manifest.artifacts.len() >= 6);
}

#[test]
fn selects_windows_amd64_artifacts_from_go_manifest() {
    let signature = read_signature();
    let manifest = verify_manifest(MANIFEST, &signature, DEVELOPMENT_PLUGIN_PUBLIC_KEY).unwrap();
    let artifacts = select_artifacts(&manifest, "windows", "amd64");
    assert!(artifacts.len() >= 6);
    for artifact in &artifacts {
        assert!(artifact.url.starts_with("https://"));
        assert_eq!(
            artifact_download_url(&manifest, artifact).unwrap(),
            artifact.url
        );
    }
}

#[test]
fn tampered_go_manifest_is_rejected() {
    let signature = read_signature();
    let mut tampered = MANIFEST.to_vec();
    let index = tampered.len() / 2;
    tampered[index] ^= 1;
    assert!(verify_manifest(&tampered, &signature, DEVELOPMENT_PLUGIN_PUBLIC_KEY).is_err());
}

#[test]
fn wrong_public_key_rejects_go_manifest() {
    let signature = read_signature();
    let wrong_key = "0000000000000000000000000000000000000000000000000000000000000000";
    assert!(verify_manifest(MANIFEST, &signature, wrong_key).is_err());
}
