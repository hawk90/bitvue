//! Characterization tests for the `bitvue-mcp-server` stdio JSON-RPC protocol.
//!
//! A fixed script of requests (every tool, error paths, a path-traversal attempt, a
//! malformed line) is piped into the real binary; each response line is hashed and
//! compared with the value recorded before `main.rs` was split by responsibility.
//!
//! Only stdout lines that start with `{` are responses: the server currently also
//! writes tracing logs to stdout (recorded separately), so they are ignored here.
//!
//! Platform differences are normalised before hashing (see `normalise`): path separators,
//! the Windows `\\?\` verbatim prefix, the workspace root and OS error wording.
//! To re-record after an intentional output change, run with `GOLDEN_PRINT=1`.

use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const REQUESTS: &[&str] = &[
    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#,
    r#"{"jsonrpc":"2.0","id":4,"method":"bogus/method"}"#,
    r#"{"jsonrpc":"2.0","id":5,"method":"tools/call"}"#,
    r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"nope","arguments":{}}}"#,
    r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"load_file","arguments":{"path":".."}}}"#,
    r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"get_stream_info","arguments":{}}}"#,
    r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"load_file","arguments":{"path":"test_data/av1_test.ivf"}}}"#,
    r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"get_stream_info","arguments":{}}}"#,
    r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"analyze_frame","arguments":{"frame_index":0}}}"#,
    r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"analyze_frame","arguments":{"frame_index":5}}}"#,
    r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"get_qp_map","arguments":{"frame_index":1}}}"#,
    r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"get_motion_vectors","arguments":{"frame_index":1}}}"#,
    r#"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"get_gop_structure","arguments":{"max_frames":12}}}"#,
    r#"{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{"name":"find_decoding_issues","arguments":{}}}"#,
    r#"{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"search_syntax","arguments":{"frame_type":"I","limit":5}}}"#,
    r#"{"jsonrpc":"2.0","id":18,"method":"tools/call","params":{"name":"search_syntax","arguments":{"min_qp":1,"limit":3}}}"#,
    r#"{"jsonrpc":"2.0","id":19,"method":"tools/call","params":{"name":"compare_streams","arguments":{"frame_index":2}}}"#,
    r#"{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"load_file","arguments":{"path":"test_data/vp9_test.ivf","stream":"B"}}}"#,
    r#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"compare_streams","arguments":{"frame_index":2}}}"#,
    r#"{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"name":"list_files","arguments":{}}}"#,
    r#"{"jsonrpc":"2.0","id":23,"method":"tools/call","params":{"name":"analyze_frame","arguments":{"frame_index":9999}}}"#,
    r#"{"jsonrpc":"2.0","id":24,"method":"tools/call","params":{"name":"load_file","arguments":{"path":"test_data/does_not_exist.ivf"}}}"#,
    r#"this is not json"#,
    r#"{"jsonrpc":"2.0","id":25,"method":"ping"}"#,
];

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
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

/// Runs the server on [`REQUESTS`]; returns the masked response lines.
fn responses() -> Vec<String> {
    let root = workspace_root();
    let mut child = Command::new(env!("CARGO_BIN_EXE_bitvue-mcp-server"))
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for r in REQUESTS {
        writeln!(stdin, "{r}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let root_str = root.to_str().unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| normalise(serde_json::from_str(l).unwrap(), root_str).to_string())
        .collect()
}

/// Makes a response identical across platforms: tool results are JSON embedded in a string, so
/// those are parsed and normalised recursively too.
fn normalise(v: Value, root: &str) -> Value {
    match v {
        Value::String(s) => match serde_json::from_str::<Value>(&s) {
            Ok(inner @ (Value::Object(_) | Value::Array(_))) => {
                Value::String(normalise(inner, root).to_string())
            }
            _ => Value::String(normalise_str(&s, root)),
        },
        Value::Array(a) => Value::Array(a.into_iter().map(|x| normalise(x, root)).collect()),
        Value::Object(m) => Value::Object(
            m.into_iter()
                .map(|(k, x)| (k, normalise(x, root)))
                .collect(),
        ),
        other => other,
    }
}

fn normalise_str(s: &str, root: &str) -> String {
    // Windows `canonicalize` yields `\\?\C:\...`; drop the prefix, unify separators, mask the root.
    let s = s.replace(r"\\?\", "").replace('\\', "/");
    let mut s = s.replace(&root.replace('\\', "/"), "<root>");
    // The wording and number of OS errors differ per platform ("... (os error 2)").
    const MARK: &str = "Invalid path: ";
    if let Some(i) = s.find(MARK) {
        let start = i + MARK.len();
        if let Some(rel) = s[start..].find(" (os error ") {
            if let Some(close) = s[start + rel..].find(')') {
                s = format!("{}<os error>{}", &s[..start], &s[start + rel + close + 1..]);
            }
        }
    }
    s
}

/// Expected FNV-1a digest of each response line, in request order.
const GOLDEN: &[u64] = &[
    0xee9bb3178913c63a,
    0x9ea413d059f81faa,
    0xfd8c9726f2363f67,
    0x2358c1d17aa39777,
    0xf71c6d4d09b4369e,
    0xef51ad3c0ec28fdc,
    0x82c643705344ab2e,
    0xc7c095a768f103c5,
    0xd481c5805a314214,
    0x45bf0f55e7ac8f9a,
    0x8bb878b0e1b76dd1,
    0xbca78348053073fa,
    0x98b02d5abb2a79ca,
    0x959f599f9a9dc85c,
    0x20e26348571ee076,
    0x4211182c68819fc9,
    0x0727fa91ade69efa,
    0x5048d383297f0ab4,
    0xcde79be53bfaea89,
    0xbcef9b07c475f015,
    0xe7b36dbe912a54b7,
    0x894e1984783efab9,
    0x6d5f6ac86a21a147,
    0x74e53073c0a27d06,
    0x293b76e9c5e461b5,
];

#[test]
fn every_request_gets_the_recorded_response() {
    let got = responses();
    if std::env::var_os("GOLDEN_PRINT").is_some() {
        for g in &got {
            println!("GOLDEN 0x{:016x}", fnv1a(g.as_bytes()));
        }
        return;
    }
    // The malformed line gets no response, so there is one fewer response than requests.
    assert_eq!(got.len(), GOLDEN.len(), "response count changed");
    let bad: Vec<String> = got
        .iter()
        .zip(GOLDEN)
        .enumerate()
        .filter(|(_, (line, want))| fnv1a(line.as_bytes()) != **want)
        .map(|(i, (line, _))| format!("#{i}: {line}"))
        .collect();
    assert!(
        bad.is_empty(),
        "{} response(s) changed:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
