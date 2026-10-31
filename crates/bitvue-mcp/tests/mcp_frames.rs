//! The frame information the MCP tools report must be the stream's own (#60): frame type, size,
//! offset and PTS equal what `bitvue decode` prints for `test_data/av1_test.ivf` (decoder-derived
//! types, sizes without the 12-byte IVF frame header).

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

/// `(frame_type, size, pts, offset)` of the first frames, as `bitvue decode` lists them.
const FIRST_FRAMES: &[(&str, u64, u64, u64)] = &[
    ("I", 10615, 0, 44),
    ("P", 5476, 1, 10671),
    ("P", 205, 2, 16159),
    ("P", 239, 3, 16376),
];

fn call(requests: &[Value]) -> Vec<Value> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let mut child = Command::new(env!("CARGO_BIN_EXE_bitvue-mcp-server"))
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for r in requests {
        writeln!(stdin, "{r}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| {
            let r: Value = serde_json::from_str(l).unwrap();
            serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
        })
        .collect()
}

fn tool(id: u32, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}})
}

#[test]
fn analyze_frame_reports_the_real_type_size_pts_and_offset() {
    let mut requests = vec![tool(
        0,
        "load_file",
        json!({"path": "test_data/av1_test.ivf"}),
    )];
    for i in 0..FIRST_FRAMES.len() {
        requests.push(tool(
            i as u32 + 1,
            "analyze_frame",
            json!({"frame_index": i}),
        ));
    }
    let responses = call(&requests);
    assert_eq!(responses[0]["frame_count"], 250);
    for (i, &(kind, size, pts, offset)) in FIRST_FRAMES.iter().enumerate() {
        let f = &responses[i + 1];
        assert_eq!(f["frame_type"], kind, "frame {i} type");
        assert_eq!(f["size"], size, "frame {i} size");
        assert_eq!(f["pts"], pts, "frame {i} pts");
        assert_eq!(f["offset"], offset, "frame {i} offset");
    }
}

#[test]
fn gop_structure_counts_one_key_frame_and_the_rest_inter() {
    let responses = call(&[
        tool(0, "load_file", json!({"path": "test_data/av1_test.ivf"})),
        tool(1, "get_gop_structure", json!({"max_frames": 250})),
    ]);
    let counts = &responses[1]["frame_type_counts"];
    assert_eq!(
        (counts["I"].as_u64(), counts["P"].as_u64()),
        (Some(1), Some(249))
    );
    // QP is a base_q_idx, so 0..=255. The key frame's 42 is what the full frame-header parser
    // (the one the sidecar uses) reads; it is pinned here, not checked against another tool.
    let qps: Vec<u64> = responses[1]["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["qp"].as_u64().unwrap())
        .collect();
    assert!(qps.iter().all(|&q| q <= 255));
    assert_eq!(qps[0], 42, "key frame base_q_idx as parsed today");
}
