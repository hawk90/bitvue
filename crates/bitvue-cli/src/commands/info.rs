//! Display stream information about a video file

use anyhow::Result;
use bitvue_formats::container::detect_container_format;
use std::path::PathBuf;

pub fn run(file_path: PathBuf) -> Result<()> {
    println!("Bitvue CLI - Video File Analyzer");
    println!("================================");

    // Validate file exists
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    // Read file data
    let file_data =
        std::fs::read(&file_path).map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

    // Detect format using proper container detection
    let format = detect_container_format(&file_path)
        .map(|f| format!("{:?}", f))
        .unwrap_or_else(|_| "Unknown".to_string());

    println!("File: {}", file_path.display());
    println!("Size: {} bytes", file_data.len());
    println!("Format: {}", format);

    // TODO: Parse video structure using bitvue-core
    // This will be implemented by integrating with existing bitvue-core functionality

    Ok(())
}
