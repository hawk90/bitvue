//! Validate bitstream syntax

use anyhow::{Context, Result};
use bitvue_av1_codec::{parse_all_obus_resilient, parse_ivf_frames, parse_ivf_header, ObuType};
use bitvue_engine::StreamId;
use bitvue_formats::container::{detect_container_format, ContainerFormat};
use std::path::PathBuf;

pub fn run(file_path: PathBuf, strict: bool) -> Result<()> {
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    let file_data = std::fs::read(&file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let container = detect_container_format(&file_path).unwrap_or(ContainerFormat::Unknown);

    println!("Validating: {}", file_path.display());
    println!("Container:  {:?}", container);

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    match container {
        ContainerFormat::IVF => validate_ivf(&file_data, &mut errors, &mut warnings),
        ContainerFormat::MP4 => validate_mp4(&file_data, &mut errors, &mut warnings),
        ContainerFormat::Unknown => {
            warnings
                .push("Could not detect container format — performing raw OBU scan".to_string());
            validate_raw_obus(&file_data, &mut errors, &mut warnings);
        }
        _ => {
            warnings.push(format!(
                "Validation not fully implemented for {:?}; checking file accessibility only",
                container
            ));
        }
    }

    // Report
    if errors.is_empty() && warnings.is_empty() {
        println!("Status: OK — no errors or warnings found");
    } else {
        for w in &warnings {
            println!("WARNING: {}", w);
        }
        for e in &errors {
            println!("ERROR:   {}", e);
        }

        if !errors.is_empty() {
            println!(
                "\nValidation FAILED ({} error(s), {} warning(s))",
                errors.len(),
                warnings.len()
            );
            if strict {
                anyhow::bail!("Validation failed with {} error(s)", errors.len());
            }
        } else {
            println!("\nValidation PASSED with {} warning(s)", warnings.len());
        }
    }

    Ok(())
}

fn validate_ivf(data: &[u8], errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    // Check IVF header
    match parse_ivf_header(data) {
        Err(e) => {
            errors.push(format!("IVF header parse error: {}", e));
            return;
        }
        Ok(header) => {
            if header.version != 0 {
                warnings.push(format!(
                    "IVF header version {} (expected 0)",
                    header.version
                ));
            }
            if header.header_size != 32 {
                warnings.push(format!(
                    "IVF header_size {} (expected 32)",
                    header.header_size
                ));
            }
            let fourcc = std::str::from_utf8(&header.fourcc).unwrap_or("????");
            if !matches!(&header.fourcc, b"AV01" | b"VP90" | b"VP80") {
                warnings.push(format!("Unknown FourCC: {} — may not be decodable", fourcc));
            }
        }
    }

    // Parse frames
    match parse_ivf_frames(data) {
        Err(e) => {
            errors.push(format!("IVF frame parsing error: {}", e));
        }
        Ok((header, frames)) => {
            if header.frame_count as usize != frames.len() {
                warnings.push(format!(
                    "IVF header declares {} frames but {} were parsed",
                    header.frame_count,
                    frames.len()
                ));
            }

            // For AV1, validate OBU structure in each frame
            if &header.fourcc == b"AV01" {
                let mut seq_header_seen = false;
                for (idx, frame) in frames.iter().enumerate() {
                    let (parsed, diagnostics) = parse_all_obus_resilient(&frame.data, StreamId::A);
                    let failed = diagnostics.len();
                    if failed > 0 {
                        errors.push(format!("Frame {}: {} OBU(s) failed to parse", idx, failed));
                    }
                    for obu in &parsed {
                        if obu.header.obu_type == ObuType::SequenceHeader {
                            seq_header_seen = true;
                        }
                    }
                }
                if !seq_header_seen {
                    warnings.push("No AV1 sequence header OBU found in bitstream".to_string());
                }
            }
        }
    }
}

fn validate_mp4(data: &[u8], errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    use bitvue_formats::mp4::parse_mp4;
    match parse_mp4(data) {
        Err(e) => {
            errors.push(format!("MP4 parse error: {}", e));
        }
        Ok(info) => {
            if info.codec.is_none() {
                warnings.push("No codec information found in MP4 file".to_string());
            }
            if info.sample_count == 0 {
                warnings.push("MP4 file contains no samples".to_string());
            }
            if info.timescale == 0 {
                warnings.push("MP4 timescale is zero — timestamps will be invalid".to_string());
            }
            if info.sample_offsets.len() != info.sample_sizes.len() && info.sample_offsets.len() > 1
            {
                warnings.push(format!(
                    "Offset count ({}) does not match size count ({})",
                    info.sample_offsets.len(),
                    info.sample_sizes.len()
                ));
            }
        }
    }
}

fn validate_raw_obus(data: &[u8], errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let (parsed, diagnostics) = parse_all_obus_resilient(data, StreamId::A);
    let failed = diagnostics.len();

    if parsed.is_empty() {
        errors.push("No valid OBUs found in file".to_string());
        return;
    }
    if failed > 0 {
        errors.push(format!("{} OBU(s) failed to parse", failed));
    }
    let has_seq = parsed
        .iter()
        .any(|o| o.header.obu_type == ObuType::SequenceHeader);
    if !has_seq {
        warnings.push("No sequence header OBU found".to_string());
    }
    println!("OBUs parsed: {} ok, {} diagnostics", parsed.len(), failed);
}
