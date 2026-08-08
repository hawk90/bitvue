//! Wire protocol between `bitvue-desktop` (Electron main process) and the `bitvue-engine`
//! sidecar process. Spec: `docs/DEVELOPMENT_PHASES.md` § "bitvue-protocol wire schema v0".
//!
//! Transport is a single stdio duplex channel. `stdout`/`stdin` carry only framed protocol
//! bytes; `stderr` is reserved for logs/panic output so a sidecar crash stays diagnosable.
//! This crate intentionally does not depend on `bitvue-core` — the wire contract must stay
//! stable independent of internal engine error/type changes. Mapping engine errors onto
//! [`WireErrorCode`] is the sidecar binary's job, not this crate's.

use serde::{Deserialize, Serialize};

/// Frame header size in bytes: 1 (kind) + 4 (correlation_id, little-endian) + 4 (payload_len, little-endian).
pub const FRAME_HEADER_LEN: usize = 9;

/// Current protocol version, exchanged during the `hello` handshake.
pub const PROTOCOL_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    /// Small structured JSON (`Request`/`Response`).
    Control = 0,
    /// Raw bytes — decoded frame planes, QP/MV typed arrays, hex ranges, etc. No wrapper encoding.
    Data = 1,
    /// Sidecar-initiated push (progress, warnings). `correlation_id` is 0 unless tied to a subscription.
    Event = 2,
}

impl FrameKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Control),
            1 => Some(Self::Data),
            2 => Some(Self::Event),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameHeader {
    pub kind: FrameKind,
    pub correlation_id: u32,
    pub payload_len: u32,
}

impl FrameHeader {
    pub fn encode(&self) -> [u8; FRAME_HEADER_LEN] {
        let mut buf = [0u8; FRAME_HEADER_LEN];
        buf[0] = self.kind as u8;
        buf[1..5].copy_from_slice(&self.correlation_id.to_le_bytes());
        buf[5..9].copy_from_slice(&self.payload_len.to_le_bytes());
        buf
    }

    pub fn decode(buf: &[u8; FRAME_HEADER_LEN]) -> Result<Self, ProtocolError> {
        let kind = FrameKind::from_u8(buf[0]).ok_or(ProtocolError::UnknownFrameKind(buf[0]))?;
        let correlation_id = u32::from_le_bytes(buf[1..5].try_into().unwrap());
        let payload_len = u32::from_le_bytes(buf[5..9].try_into().unwrap());
        Ok(Self {
            kind,
            correlation_id,
            payload_len,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("unknown frame kind byte: {0}")]
    UnknownFrameKind(u8),
}

/// Control-plane request (main -> sidecar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: u32,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Control-plane response (sidecar -> main). `result` carries small structured data, or
/// metadata for a `Data` frame that immediately follows with the same `correlation_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: u32,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WireError>,
}

impl Response {
    pub fn success(id: u32, result: serde_json::Value) -> Self {
        Self {
            id,
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(id: u32, error: WireError) -> Self {
        Self {
            id,
            ok: false,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireError {
    pub code: WireErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

/// Stable wire-level error taxonomy. Deliberately a curated mirror of
/// `bitvue_core::error::BitvueError`'s variants, not a direct `Serialize` derive on that type —
/// keeps the cross-process contract stable if internal engine error fields change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WireErrorCode {
    Io,
    Parse,
    InvalidObuType,
    UnexpectedEof,
    UnsupportedCodec,
    Decode,
    InsufficientData,
    InvalidData,
    InvalidFile,
    InvalidRange,
    FileModified,
    FrameNotFound,
    NotFound,
    Serialization,
    /// Raised by the protocol layer itself in response to `cancel_request` — not a `BitvueError` variant.
    Cancelled,
    /// Fallback for engine errors that don't map cleanly to a known code.
    Internal,
}

/// Params for the `cancel_request` control method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelParams {
    pub target_id: u32,
}

/// Params for the `hello` handshake (main -> sidecar, first message on startup).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloParams {
    pub client_version: String,
}

/// Result of the `hello` handshake (sidecar -> main).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResult {
    pub protocol_version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_header_roundtrip() {
        let header = FrameHeader {
            kind: FrameKind::Data,
            correlation_id: 42,
            payload_len: 12_582_912,
        };
        let encoded = header.encode();
        let decoded = FrameHeader::decode(&encoded).unwrap();
        assert_eq!(decoded.kind, FrameKind::Data);
        assert_eq!(decoded.correlation_id, 42);
        assert_eq!(decoded.payload_len, 12_582_912);
    }

    #[test]
    fn unknown_frame_kind_rejected() {
        let mut buf = [0u8; FRAME_HEADER_LEN];
        buf[0] = 255;
        assert!(matches!(
            FrameHeader::decode(&buf),
            Err(ProtocolError::UnknownFrameKind(255))
        ));
    }

    #[test]
    fn response_envelope_serializes_as_documented() {
        let ok = Response::success(1, serde_json::json!({"frame_index": 183}));
        let json = serde_json::to_value(&ok).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["result"]["frame_index"], 183);
        assert!(json.get("error").is_none());

        let err = Response::failure(
            2,
            WireError {
                code: WireErrorCode::Parse,
                message: "bad OBU".into(),
                offset: Some(4096),
            },
        );
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["ok"], false);
        assert_eq!(json["error"]["code"], "PARSE");
        assert!(json.get("result").is_none());
    }
}
