//! Batch process multiple files

use std::path::PathBuf;
use anyhow::Result;

pub fn run(directory: PathBuf, pattern: &str, output: PathBuf) -> Result<()> {
    println!("Batch command: {} with pattern '{}' -> {}", directory.display(), pattern, output.display());
    // TODO: Implement batch processing
    Ok(())
}
