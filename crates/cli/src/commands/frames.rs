//! List all frames in the video

use std::path::PathBuf;
use anyhow::Result;

pub fn run(file_path: PathBuf, limit: usize, format: &str) -> Result<()> {
    println!("Frames command: {} (limit: {}, format: {})", file_path.display(), limit, format);
    // TODO: Implement frame listing
    Ok(())
}
