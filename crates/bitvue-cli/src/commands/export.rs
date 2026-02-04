//! Export analysis results to file

use anyhow::Result;
use std::path::PathBuf;

pub fn run(file_path: PathBuf, output: PathBuf, format: &str) -> Result<()> {
    println!(
        "Export command: {} -> {} ({})",
        file_path.display(),
        output.display(),
        format
    );
    // TODO: Implement export
    Ok(())
}
