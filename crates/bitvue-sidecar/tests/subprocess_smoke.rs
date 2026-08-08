//! Real subprocess smoke test: builds and runs the actual compiled `bitvue-sidecar` binary and
//! drives it over real OS pipes (not `Cursor`, unlike the unit tests in `src/main.rs`) —
//! `hello` → `open_stream` → `get_hex_range`, verifying the data-plane bytes come back
//! byte-exact from the real process boundary.
//!
//! Lives under `tests/` (an integration test target) rather than `src/main.rs`'s `#[cfg(test)]`
//! module because `env!("CARGO_BIN_EXE_bitvue-sidecar")` is only populated by Cargo for
//! integration test / example / bench targets of a package that has a `[[bin]]` — not for the
//! bin target's own unit tests.

use std::io::{Read, Write};
use std::process::{Command, Stdio};

use bitvue_protocol::{FrameHeader, FrameKind, Request, Response, FRAME_HEADER_LEN};

fn write_request<W: Write>(writer: &mut W, id: u32, method: &str, params: serde_json::Value) {
    let request = Request {
        id,
        method: method.to_string(),
        params,
    };
    let body = serde_json::to_vec(&request).unwrap();
    let header = FrameHeader {
        kind: FrameKind::Control,
        correlation_id: id,
        payload_len: body.len() as u32,
    };
    writer.write_all(&header.encode()).unwrap();
    writer.write_all(&body).unwrap();
    writer.flush().unwrap();
}

fn read_frame_blocking<R: Read>(reader: &mut R) -> (FrameHeader, Vec<u8>) {
    let mut header_buf = [0u8; FRAME_HEADER_LEN];
    reader
        .read_exact(&mut header_buf)
        .expect("expected a response frame header, got EOF/short read");
    let header = FrameHeader::decode(&header_buf).unwrap();
    let mut payload = vec![0u8; header.payload_len as usize];
    reader.read_exact(&mut payload).unwrap();
    (header, payload)
}

#[test]
fn subprocess_smoke_hello_open_stream_get_hex_range() {
    let known_bytes: Vec<u8> = (0u8..=255).collect();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(&known_bytes).unwrap();

    let bin = env!("CARGO_BIN_EXE_bitvue-sidecar");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn bitvue-sidecar binary");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(
        &mut stdin,
        1,
        "hello",
        serde_json::json!({"client_version": "0.1.0-smoke"}),
    );
    let (header, body) = read_frame_blocking(&mut stdout);
    assert_eq!(header.kind, FrameKind::Control);
    let response: Response = serde_json::from_slice(&body).unwrap();
    assert!(response.ok, "hello failed: {response:?}");

    write_request(
        &mut stdin,
        2,
        "open_stream",
        serde_json::json!({"stream": "A", "path": file.path().to_str().unwrap()}),
    );
    let (header, body) = read_frame_blocking(&mut stdout);
    assert_eq!(header.kind, FrameKind::Control);
    let response: Response = serde_json::from_slice(&body).unwrap();
    assert!(response.ok, "open_stream failed: {response:?}");
    let events = response.result.unwrap()["events"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(events[0]["type"], "ModelUpdated");

    let offset: u64 = 5;
    let len: usize = 20;
    write_request(
        &mut stdin,
        3,
        "get_hex_range",
        serde_json::json!({"stream": "A", "offset": offset, "len": len}),
    );
    let (ctrl_header, ctrl_body) = read_frame_blocking(&mut stdout);
    assert_eq!(ctrl_header.kind, FrameKind::Control);
    assert_eq!(ctrl_header.correlation_id, 3);
    let ctrl_response: Response = serde_json::from_slice(&ctrl_body).unwrap();
    assert!(ctrl_response.ok, "get_hex_range failed: {ctrl_response:?}");
    assert_eq!(
        ctrl_response.result.unwrap(),
        serde_json::json!({"offset": offset, "len": len})
    );

    let (data_header, data_body) = read_frame_blocking(&mut stdout);
    assert_eq!(data_header.kind, FrameKind::Data);
    assert_eq!(data_header.correlation_id, 3);
    assert_eq!(data_body.len(), len);
    assert_eq!(
        data_body,
        known_bytes[offset as usize..offset as usize + len]
    );

    drop(stdin); // close stdin -> sidecar's read loop sees EOF and exits cleanly
    let status = child.wait().expect("sidecar process failed to exit");
    assert!(status.success(), "sidecar exited with {status:?}");
}
