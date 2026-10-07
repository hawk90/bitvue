//! JSON-RPC / MCP wire types: request, response, error and tool descriptor.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// MCP Request
#[derive(Debug, Deserialize)]
pub(crate) struct McpRequest {
    #[serde(rename = "jsonrpc")]
    pub(crate) _jsonrpc: String,
    pub(crate) id: Value,
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

/// MCP Response
pub(crate) struct McpResponse {
    pub(crate) jsonrpc: String,
    pub(crate) id: Value,
    pub(crate) result: Option<Value>,
    pub(crate) error: Option<McpError>,
}

impl McpResponse {
    pub(crate) fn to_json(&self) -> Result<String> {
        let mut obj = json!({
            "jsonrpc": self.jsonrpc,
            "id": self.id
        });

        if let Some(result) = &self.result {
            obj["result"] = result.clone();
        }
        if let Some(error) = &self.error {
            obj["error"] = json!({
                "code": error.code,
                "message": error.message,
                "data": error.data
            });
        }

        Ok(serde_json::to_string(&obj)?)
    }
}

/// MCP Error
#[derive(Debug)]
pub(crate) struct McpError {
    pub(crate) code: i32,
    pub(crate) message: String,
    pub(crate) data: Option<Value>,
}

/// MCP Tool definition
#[derive(Debug, Serialize)]
pub(crate) struct Tool {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) input_schema: Value,
}
