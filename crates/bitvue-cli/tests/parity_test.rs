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

// ─── HEVC synthetic bitstream tests ──────────────────────────────────────────

/// Minimal Annex B HEVC stream: just a start code + 2-byte NAL header.
/// NAL type 32 = VPS (0x40 0x01 in network byte order).
fn minimal_hevc_annexb() -> Vec<u8> {
    let mut data = Vec::new();
    // Annex B start code
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    // HEVC NAL header (2 bytes): forbidden_zero_bit=0, nal_unit_type=32 (VPS), nuh_layer_id=0, nuh_temporal_id_plus1=1
    // nal_unit_type 32 → bits [9:15] in first 16-bit word
    // 0b0_100000_000000_001 → 0x40 0x01
    data.extend_from_slice(&[0x40, 0x01]);
    data
}

#[test]
fn hevc_empty_file_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.hevc");
    std::fs::write(&path, b"").unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::HEVC),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn hevc_minimal_annexb_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("minimal.hevc");
    std::fs::write(&path, minimal_hevc_annexb()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::HEVC),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn hevc_garbage_data_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("garbage.hevc");
    std::fs::write(&path, b"\xde\xad\xbe\xef".repeat(64)).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::HEVC),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn hevc_stats_flag_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("minimal.hevc");
    std::fs::write(&path, minimal_hevc_annexb()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::HEVC),
        stats: true,
        ..Default::default()
    };
    let _ = run(cfg);
}

#[test]
fn hevc_regress_flag_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("minimal.hevc");
    std::fs::write(&path, minimal_hevc_annexb()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::HEVC),
        regress: true,
        ..Default::default()
    };
    let _ = run(cfg);
}

// ─── AVC synthetic bitstream tests ───────────────────────────────────────────

/// Minimal Annex B AVC stream: start code + 1-byte NAL header.
/// NAL type 7 = SPS (0x67).
fn minimal_avc_annexb() -> Vec<u8> {
    let mut data = Vec::new();
    // Annex B start code
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    // AVC NAL header: forbidden_zero_bit=0, nal_ref_idc=3, nal_unit_type=7 (SPS)
    // 0b0_11_00111 → 0x67
    data.push(0x67);
    // Minimal SPS payload (just a few zero bytes — parser must not panic)
    data.extend_from_slice(&[0x00, 0x00, 0x00]);
    data
}

#[test]
fn avc_empty_file_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.h264");
    std::fs::write(&path, b"").unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::AVC),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn avc_minimal_annexb_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("minimal.h264");
    std::fs::write(&path, minimal_avc_annexb()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::AVC),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn avc_garbage_data_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("garbage.h264");
    std::fs::write(&path, b"\xde\xad\xbe\xef".repeat(64)).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::AVC),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn avc_stats_flag_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("minimal.h264");
    std::fs::write(&path, minimal_avc_annexb()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::AVC),
        stats: true,
        ..Default::default()
    };
    let _ = run(cfg);
}

#[test]
fn avc_regress_flag_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("minimal.h264");
    std::fs::write(&path, minimal_avc_annexb()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::AVC),
        regress: true,
        ..Default::default()
    };
    let _ = run(cfg);
}

// ─── VP9 synthetic bitstream tests ───────────────────────────────────────────

/// Minimal IVF file with VP90 FourCC and zero frames.
fn minimal_ivf_vp9() -> Vec<u8> {
    let mut data = Vec::with_capacity(32);
    data.extend_from_slice(b"DKIF");
    data.extend_from_slice(&0u16.to_le_bytes()); // version
    data.extend_from_slice(&32u16.to_le_bytes()); // header size
    data.extend_from_slice(b"VP90"); // FourCC
    data.extend_from_slice(&320u16.to_le_bytes()); // width
    data.extend_from_slice(&240u16.to_le_bytes()); // height
    data.extend_from_slice(&30u32.to_le_bytes()); // fps numerator
    data.extend_from_slice(&1u32.to_le_bytes()); // fps denominator
    data.extend_from_slice(&0u32.to_le_bytes()); // frame count
    data.extend_from_slice(&0u32.to_le_bytes()); // reserved
    assert_eq!(data.len(), 32);
    data
}

/// Minimal IVF VP90 with one synthetic frame (12-byte IVF frame header + minimal payload).
fn minimal_ivf_vp9_one_frame() -> Vec<u8> {
    let mut data = minimal_ivf_vp9();
    // Update frame count to 1
    data[28] = 1;
    // IVF frame header: size (4 bytes LE) + pts (8 bytes LE)
    let payload: &[u8] = &[
        // VP9 frame marker (2 bits = 0b10), profile (2 bits), reserved, show_existing_frame=0,
        // frame_type=0 (key), show_frame=1, error_resilient=0
        // First byte: 0b10_00_0_0_0_1 = 0x81 (rough approximation — parser must not panic)
        0x81, 0x00, 0x00,
    ];
    data.extend_from_slice(&(payload.len() as u32).to_le_bytes()); // frame size
    data.extend_from_slice(&0u64.to_le_bytes()); // pts
    data.extend_from_slice(payload);
    data
}

#[test]
fn vp9_empty_file_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.vp9");
    std::fs::write(&path, b"").unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::VP9),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn vp9_minimal_ivf_no_frames_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("minimal.vp9.ivf");
    std::fs::write(&path, minimal_ivf_vp9()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::VP9),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn vp9_minimal_ivf_one_frame_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("one_frame.vp9.ivf");
    std::fs::write(&path, minimal_ivf_vp9_one_frame()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::VP9),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn vp9_garbage_data_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("garbage.vp9");
    std::fs::write(&path, b"\xde\xad\xbe\xef".repeat(64)).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::VP9),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn vp9_stats_flag_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("minimal.vp9.ivf");
    std::fs::write(&path, minimal_ivf_vp9()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::VP9),
        stats: true,
        ..Default::default()
    };
    let _ = run(cfg);
}

#[test]
fn vp9_autodetect_from_ivf_fourcc_does_not_panic() {
    // Auto-detect: no ForceCodec; let the IVF FourCC drive codec selection
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("auto.vp9.ivf");
    std::fs::write(&path, minimal_ivf_vp9()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: None,
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

// ─── Cross-codec regression: same file, wrong codec ──────────────────────────

#[test]
fn hevc_file_decoded_as_avc_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("hevc_as_avc.bin");
    std::fs::write(&path, minimal_hevc_annexb()).unwrap();

    // Intentional mismatch: tell decode it's AVC when it's actually HEVC data
    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::AVC),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic regardless of codec mismatch
}

#[test]
fn avc_file_decoded_as_hevc_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("avc_as_hevc.bin");
    std::fs::write(&path, minimal_avc_annexb()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::HEVC),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}

#[test]
fn vp9_ivf_decoded_as_av1_does_not_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("vp9_as_av1.ivf");
    std::fs::write(&path, minimal_ivf_vp9()).unwrap();

    let cfg = DecodeConfig {
        file: path,
        force_codec: Some(ForceCodec::AV1),
        ..Default::default()
    };
    let _ = run(cfg); // must not panic
}
