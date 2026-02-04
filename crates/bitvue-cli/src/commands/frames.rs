//! List all frames in the video

use anyhow::Result;
use std::path::PathBuf;

pub fn run(file_path: PathBuf, limit: usize, format: &str) -> Result<()> {
    println!(
        "Frames command: {} (limit: {}, format: {})",
        file_path.display(),
        limit,
        format
    );
    // TODO: Implement frame listing
    Ok(())
}
