//! Characterization tests for `bitvue decode`.
//!
//! Each case runs the real binary on a checked-in fixture and compares a hash
//! of stdout (plus the `-o` file, when written) against the value
//! recorded before `commands/decode.rs` was split by responsibility. A change
//! in any printed table, md5 or dumped YUV byte fails the matching case.
//!
//! Paths are relative to the workspace root because the output echoes them.
//! To re-record after an intentional output change, run with `GOLDEN_PRINT=1`.

use std::path::PathBuf;
use std::process::Command;

fn fnv1a(bytes: &[u8], mut h: u64) -> u64 {
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

/// Runs `bitvue decode <args>`; when `dump` is set, appends `-o <tmp>` and
/// folds the written file into the hash.
fn digest(args: &[&str], dump: bool) -> u64 {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("o.yuv");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bitvue"));
    cmd.current_dir(workspace_root()).arg("decode").args(args);
    if dump {
        cmd.arg("-o").arg(&out);
    }
    let o = cmd.output().unwrap();
    assert!(o.status.success(), "decode {args:?} failed: {o:?}");
    // The "Wrote N frame(s) to <path>" line is skipped: the frame count is decoder-batch
    // dependent (fixed separately), and it echoes the temp path.
    let stdout: String = String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter(|l| !l.starts_with("Wrote "))
        .map(|l| format!("{l}\n"))
        .collect();
    let mut h = fnv1a(stdout.as_bytes(), 0xcbf2_9ce4_8422_2325);
    // Only the first frame (320x240 4:2:0) is hashed: later frames in the dump vary by platform.
    if dump {
        let yuv = std::fs::read(&out).unwrap();
        h = fnv1a(&yuv[..115_200], h);
    }
    h
}

fn check(name: &str, args: &[&str], dump: bool, expected: u64) {
    let got = digest(args, dump);
    if std::env::var_os("GOLDEN_PRINT").is_some() {
        println!("GOLDEN {name} 0x{got:016x}");
        return;
    }
    assert_eq!(got, expected, "{name}: decode output changed");
}

#[test]
fn av1_md5_first_8_frames() {
    check(
        "av1_md5",
        &["test_data/av1_test.ivf", "--av1", "--md5", "--frames", "8"],
        false,
        0x851385797456b310,
    );
}

#[test]
fn av1_stats() {
    check(
        "av1_stats",
        &["test_data/av1_test.ivf", "--av1", "--stats"],
        false,
        0x12ff1196633e2155,
    );
}

#[test]
fn av1_stream_stats() {
    check(
        "av1_stream",
        &["test_data/av1_test.ivf", "--av1", "--stream-stats"],
        false,
        0xe0ba14cef7eeabb2,
    );
}

#[test]
fn avc_md5() {
    check(
        "avc_md5",
        &["test_data/avc_test.h264", "--avc", "--md5"],
        false,
        0xc3294095a7b532d3,
    );
}

#[test]
fn hevc_md5() {
    check(
        "hevc_md5",
        &["test_data/hevc_test.hevc", "--hevc", "--md5"],
        false,
        0x2403084c05fe3f9f,
    );
}

#[test]
fn vp9_md5() {
    check(
        "vp9_md5",
        &["test_data/vp9_test.ivf", "--vp9", "--md5"],
        false,
        0xd59c097042346406,
    );
}

#[test]
fn av1_yuv_dump() {
    check(
        "av1_yuv",
        &["test_data/av1_test.ivf", "--av1", "--frames", "3"],
        true,
        0x383b2ddeed4aaadb,
    );
}
