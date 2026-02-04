//! Batch process multiple files

use anyhow::Result;
use std::path::PathBuf;

pub fn run(directory: PathBuf, pattern: &str, output: PathBuf) -> Result<()> {
    println!(
        "Batch command: {} with pattern '{}' -> {}",
        directory.display(),
        pattern,
        output.display()
    );
    // TODO: Implement batch processing
    Ok(())
}
