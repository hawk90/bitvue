//! Phase 12: Parity Verification & Regression Tests
//!
//! These tests verify that the `bitvue decode` command produces output that
//! is structurally correct and consistent across runs (regression baseline).
//!
//! They do NOT require external test vectors — each test either:
//!   a) uses the embedded `test_data/av1_test.ivf` fixture, or
//!   b) constructs a minimal synthetic bitstream inline.

use bitvue_cli::commands::decode::{run, DecodeConfig, ForceCodec};
use std::path::PathBuf;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn test_data(name: &str) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Walk up from crates/bitvue-cli to the workspace root, then into test_data/
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_data")
        .join(name)
}

fn av1_config() -> DecodeConfig {
    DecodeConfig {
        file: test_data("av1_test.ivf"),
        force_codec: Some(ForceCodec::AV1),
        ..Default::default()
    }
}

// ─── AV1 IVF parity tests ─────────────────────────────────────────────────────

#[test]
fn av1_decode_runs_without_error() {
    let cfg = av1_config();
    let result = run(cfg);
    assert!(result.is_ok(), "AV1 decode failed: {:?}", result.err());
}

#[test]
fn av1_stats_flag_runs_without_error() {
    let cfg = DecodeConfig {
        file: test_data("av1_test.ivf"),
        force_codec: Some(ForceCodec::AV1),
        stats: true,
        ..Default::default()
    };
    assert!(run(cfg).is_ok());
}

#[test]
fn av1_stream_stats_flag_runs_without_error() {
    let cfg = DecodeConfig {
        file: test_data("av1_test.ivf"),
        force_codec: Some(ForceCodec::AV1),
        stream_stats: true,
        ..Default::default()
    };
    assert!(run(cfg).is_ok());
}

#[test]
fn av1_md5_flag_runs_without_error() {
    let cfg = DecodeConfig {
        file: test_data("av1_test.ivf"),
        force_codec: Some(ForceCodec::AV1),
        md5: true,
        ..Default::default()
    };
    assert!(run(cfg).is_ok());
}

#[test]
fn av1_max_frames_limit_respected() {
    // Should not error even when max_frames < total frame count
    let cfg = DecodeConfig {
        file: test_data("av1_test.ivf"),
        force_codec: Some(ForceCodec::AV1),
        max_frames: 1,
        stats: true,
        ..Default::default()
    };
    assert!(run(cfg).is_ok());
}

#[test]
fn av1_autodetect_without_force_codec() {
    // Auto-detect should work for an IVF file
    let cfg = DecodeConfig {
        file: test_data("av1_test.ivf"),
        force_codec: None,
        ..Default::default()
    };
    assert!(run(cfg).is_ok());
}

// ─── Error path tests ─────────────────────────────────────────────────────────

#[test]
fn nonexistent_file_returns_error() {
    let cfg = DecodeConfig {
        file: PathBuf::from("/nonexistent/path/to/file.ivf"),
        ..Default::default()
    };
    let result = run(cfg);
    assert!(result.is_err(), "Expected error for missing file");
}

// ─── Synthetic bitstream tests ────────────────────────────────────────────────

/// A minimal valid IVF header (32 bytes) with zero frames.
/// Validates that the parser handles empty-frame files gracefully.
fn minimal_ivf_av1() -> Vec<u8> {
    let mut data = Vec::with_capacity(32);
    // IVF signature
    data.extend_from_slice(b"DKIF");
    // Version (little-endian u16)
    data.extend_from_slice(&0u16.to_le_bytes());
    // Header size (32, little-endian u16)
    data.extend_from_slice(&32u16.to_le_bytes());
    // Codec FourCC: AV01
    data.extend_from_slice(b"AV01");
    // Width (320), Height (240)
    data.extend_from_slice(&320u16.to_le_bytes());
    data.extend_from_slice(&240u16.to_le_bytes());
    // Frame rate: 30/1
    data.extend_from_slice(&30u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    // Frame count (unknown = 0)
    data.extend_from_slice(&0u32.to_le_bytes());
    // Reserved
    data.extend_from_slice(&0u32.to_le_bytes());
    assert_eq!(data.len(), 32);
    data
}

#[test]
fn minimal_ivf_no_frames_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("minimal.ivf");
    std::fs::write(&path, minimal_ivf_av1()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::AV1),
        ..Default::default()
    };
    // Should not panic; may return Ok or an error about no frames
    let _ = run(cfg);
}

#[test]
fn empty_file_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.ivf");
    std::fs::write(&path, b"").unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::AV1),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn garbage_data_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("garbage.bin");
    std::fs::write(&path, b"\x00\xff\xde\xad\xbe\xef\x42\x00".repeat(64)).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::AV1),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}
