//! Batch process multiple files

use anyhow::{Context, Result};
use bitvue_av1_codec::{parse_ivf_frames, ObuIterator, ObuType};
use bitvue_formats::container::{detect_container_format, ContainerFormat};
use bitvue_formats::mp4::parse_mp4;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Per-file summary written to the output directory
#[derive(Debug, Serialize)]
struct FileSummary {
    file: String,
    container: String,
    frame_count: usize,
    key_frames: usize,
    total_bytes: u64,
    error: Option<String>,
}

pub fn run(directory: PathBuf, pattern: &str, output: PathBuf) -> Result<()> {
    if !directory.exists() {
        anyhow::bail!("Directory not found: {}", directory.display());
    }
    if !directory.is_dir() {
        anyhow::bail!("Path is not a directory: {}", directory.display());
    }

    // Collect matching files
    let matched = collect_matching_files(&directory, pattern)?;
    if matched.is_empty() {
        println!(
            "No files matched pattern '{}' in {}",
            pattern,
            directory.display()
        );
        return Ok(());
    }

    println!(
        "Processing {} file(s) matching '{}' in {}",
        matched.len(),
        pattern,
        directory.display()
    );

    // Create output directory
    std::fs::create_dir_all(&output)
        .with_context(|| format!("Failed to create output directory: {}", output.display()))?;

    let mut summaries: Vec<FileSummary> = Vec::new();
    let mut success_count = 0usize;
    let mut error_count = 0usize;

    for file_path in &matched {
        print!("  {} ... ", file_path.display());
        match process_file(file_path, &output) {
            Ok(summary) => {
                println!(
                    "OK ({} frames, {} key frames)",
                    summary.frame_count, summary.key_frames
                );
                success_count += 1;
                summaries.push(summary);
            }
            Err(e) => {
                println!("ERROR: {}", e);
                error_count += 1;
                summaries.push(FileSummary {
                    file: file_path.display().to_string(),
                    container: "unknown".to_string(),
                    frame_count: 0,
                    key_frames: 0,
                    total_bytes: 0,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Write consolidated summary JSON
    let summary_path = output.join("batch_summary.json");
    let json = serde_json::to_string_pretty(&summaries).context("JSON serialization failed")?;
    std::fs::write(&summary_path, &json)
        .with_context(|| format!("Failed to write summary: {}", summary_path.display()))?;

    println!();
    println!(
        "Batch complete: {} succeeded, {} failed",
        success_count, error_count
    );
    println!("Summary written to: {}", summary_path.display());

    Ok(())
}

/// Process a single file and write its JSON export to the output directory.
/// Returns a `FileSummary` on success.
fn process_file(file_path: &Path, output_dir: &Path) -> Result<FileSummary> {
    let file_data = std::fs::read(file_path)
        .with_context(|| format!("Failed to read: {}", file_path.display()))?;

    let container = detect_container_format(file_path).unwrap_or(ContainerFormat::Unknown);

    let (frame_count, key_frames, total_bytes) = match container {
        ContainerFormat::IVF => analyze_ivf(&file_data)?,
        ContainerFormat::MP4 => analyze_mp4(&file_data)?,
        _ => {
            anyhow::bail!("Unsupported container format: {:?}", container);
        }
    };

    // Write per-file JSON with basic stats
    let stem = file_path.file_stem().unwrap_or_default().to_string_lossy();
    let out_path = output_dir.join(format!("{}.json", stem));

    let summary = FileSummary {
        file: file_path.display().to_string(),
        container: format!("{:?}", container),
        frame_count,
        key_frames,
        total_bytes,
        error: None,
    };
    let json = serde_json::to_string_pretty(&summary).context("JSON serialization failed")?;
    std::fs::write(&out_path, &json)
        .with_context(|| format!("Failed to write: {}", out_path.display()))?;

    Ok(summary)
}

fn analyze_ivf(data: &[u8]) -> Result<(usize, usize, u64)> {
    let (_header, frames) =
        parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF parse error: {}", e))?;

    let mut key_frames = 0usize;
    let mut total_bytes = 0u64;

    for frame in &frames {
        total_bytes += frame.size as u64;
        let frame_type = resolve_av1_frame_type(&frame.data);
        if frame_type == "I" {
            key_frames += 1;
        }
    }

    Ok((frames.len(), key_frames, total_bytes))
}

fn analyze_mp4(data: &[u8]) -> Result<(usize, usize, u64)> {
    let info = parse_mp4(data).map_err(|e| anyhow::anyhow!("MP4 parse error: {}", e))?;
    let key_frames = info.key_frames.len();
    let total_bytes: u64 = info.sample_sizes.iter().map(|&s| s as u64).sum();
    Ok((info.sample_count, key_frames, total_bytes))
}

fn resolve_av1_frame_type(frame_data: &[u8]) -> String {
    for obu in ObuIterator::new(frame_data).flatten() {
        if matches!(obu.header.obu_type, ObuType::Frame | ObuType::FrameHeader) {
            if let Some(ft) = obu.frame_type {
                return ft.as_str().to_string();
            }
        }
    }
    "?".to_string()
}

/// Collect files from a directory that match a simple glob pattern.
///
/// Supports `*` as a wildcard matching any sequence of non-separator characters,
/// and no nested directory traversal — only top-level files in `dir` are matched.
fn collect_matching_files(dir: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("Cannot read directory: {}", dir.display()))?;

    let mut matched = Vec::new();
    for entry in entries {
        let entry = entry.with_context(|| format!("Directory entry error in {}", dir.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if glob_match(pattern, &name) {
            matched.push(path);
        }
    }

    matched.sort();
    Ok(matched)
}

/// Minimal glob matcher: supports `*` (matches any run of chars except `/`).
/// Does not support `?`, `[...]`, or `**`.
fn glob_match(pattern: &str, name: &str) -> bool {
    let pat: Vec<&str> = pattern.split('*').collect();

    if pat.len() == 1 {
        // No wildcards — exact match
        return name == pattern;
    }

    // Must start with the prefix and end with the suffix
    let prefix = pat.first().copied().unwrap_or("");
    let suffix = pat.last().copied().unwrap_or("");

    if !name.starts_with(prefix) {
        return false;
    }
    if !name.ends_with(suffix) {
        return false;
    }

    // Check that middle segments appear in order
    let mut pos = prefix.len();
    let end = name.len().saturating_sub(suffix.len());
    let middle = &pat[1..pat.len() - 1];
    for seg in middle {
        if seg.is_empty() {
            continue;
        }
        match name[pos..end].find(seg) {
            Some(found) => pos += found + seg.len(),
            None => return false,
        }
    }

    // Ensure total matched region is within the non-suffix part
    pos <= end
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn test_glob_exact() {
        assert!(glob_match("foo.ivf", "foo.ivf"));
        assert!(!glob_match("foo.ivf", "bar.ivf"));
    }

    #[test]
    fn test_glob_star() {
        assert!(glob_match("*.ivf", "foo.ivf"));
        assert!(glob_match("*.ivf", "bar_test.ivf"));
        assert!(!glob_match("*.ivf", "foo.mp4"));
    }

    #[test]
    fn test_glob_prefix_star() {
        assert!(glob_match("test_*", "test_001.ivf"));
        assert!(!glob_match("test_*", "other_001.ivf"));
    }

    #[test]
    fn test_glob_both_wildcards() {
        assert!(glob_match("*.mp*", "video.mp4"));
        assert!(glob_match("*.mp*", "video.mkv.mp4"));
    }
}
