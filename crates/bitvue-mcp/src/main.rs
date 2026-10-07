//! bitvue MCP Server
//!
//! Model Context Protocol server for bitvue video analyzer.
//! Exposes video analysis capabilities to AI assistants like Claude.

mod catalog;
mod dispatch;
mod ivf;
mod protocol;
mod state;
mod tools;

use anyhow::Result;
use std::io::{self, BufRead, BufReader, Write};

use dispatch::handle_request;
use protocol::McpRequest;
use state::AppState;

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
    // Initialize logging
    // stdout is the JSON-RPC channel (MCP stdio transport); logs must go to stderr.
    tracing_subscriber::fmt()
        .with_env_filter("bitvue_mcp=debug,info")
        .with_writer(io::stderr)
        .init();

    tracing::info!("bitvue MCP Server starting...");

    let state = AppState::new();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    // Read JSON-RPC requests line by line from stdin
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Parse request
        let request: McpRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to parse request: {}", e);
                continue;
            }
        };

        tracing::debug!("Received request: {}", request.method);

        // Handle request
        let response = handle_request(request, &state);

        // Write response
        let response_json = response.to_json()?;
        writeln!(writer, "{}", response_json)?;
        writer.flush()?;
    }

    Ok(())
}
