//! Analyze a specific frame

use anyhow::{Context, Result};
use bitvue_av1_codec::{parse_ivf_frames, parse_obu_syntax, ObuIterator};
use bitvue_formats::container::{detect_container_format, ContainerFormat};
use bitvue_formats::mp4::{extract_av1_samples, parse_mp4};
use std::path::PathBuf;

pub fn run(
    file_path: PathBuf,
    frame: usize,
    syntax: bool,
    residual: bool,
    coding_flow: bool,
) -> Result<()> {
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    let file_data = std::fs::read(&file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let container = detect_container_format(&file_path).unwrap_or(ContainerFormat::Unknown);

    println!("File:      {}", file_path.display());
    println!("Container: {:?}", container);
    println!("Frame:     {}", frame);

    // Retrieve raw frame data
    let frame_data: Vec<u8> = match container {
        ContainerFormat::IVF => {
            let (_header, frames) = parse_ivf_frames(&file_data)
                .map_err(|e| anyhow::anyhow!("IVF parse error: {}", e))?;
            if frame >= frames.len() {
                anyhow::bail!(
                    "Frame index {} is out of range (file has {} frames)",
                    frame,
                    frames.len()
                );
            }
            frames[frame].data.clone()
        }
        ContainerFormat::MP4 => {
            let samples = extract_av1_samples(&file_data)
                .map_err(|e| anyhow::anyhow!("MP4 AV1 sample extraction error: {}", e))?;
            if frame >= samples.len() {
                // Try to give a better error using MP4Info frame count
                let total = parse_mp4(&file_data)
                    .map(|i| i.sample_count)
                    .unwrap_or(samples.len());
                anyhow::bail!(
                    "Frame index {} is out of range (file has {} frames)",
                    frame,
                    total
                );
            }
            samples[frame].to_vec()
        }
        _ => {
            anyhow::bail!(
                "Frame analysis is not yet supported for container format {:?}",
                container
            );
        }
    };

    println!("Frame size: {} bytes", frame_data.len());

    // Print OBU summary
    println!("\nOBU summary:");
    let mut obu_count = 0usize;
    let mut obu_offset = 0usize;
    for obu in ObuIterator::new(&frame_data) {
        match obu {
            Ok(obu) => {
                let frame_type_str = obu
                    .frame_type
                    .as_ref()
                    .map(|ft| format!(" [type={}]", ft.as_str()))
                    .unwrap_or_default();
                println!(
                    "  OBU[{}] type={:?} size={}{} offset={}",
                    obu_count,
                    obu.header.obu_type,
                    obu.payload.len(),
                    frame_type_str,
                    obu_offset
                );
                obu_offset += obu.payload.len() + 1; // +1 for header byte (approximate)
                obu_count += 1;
            }
            Err(e) => {
                println!("  OBU[{}] parse error: {}", obu_count, e);
                obu_count += 1;
            }
        }
    }
    if obu_count == 0 {
        println!("  (no OBUs found — may not be AV1)");
    }

    // Syntax analysis
    if syntax {
        println!("\nSyntax elements:");
        let mut iter = ObuIterator::new(&frame_data);
        let mut obu_idx = 0usize;
        while let Some(result) = iter.next_obu_with_offset() {
            match result {
                Ok(obu_with_offset) => {
                    let offset = obu_with_offset.offset;
                    let consumed = obu_with_offset.consumed;
                    let obu_type = obu_with_offset.obu.header.obu_type;
                    println!("  OBU[{}] {:?}:", obu_idx, obu_type);
                    let obu_slice = &frame_data[offset..offset + consumed];
                    match parse_obu_syntax(obu_slice, obu_idx, (offset * 8) as u64) {
                        Ok(model) => {
                            let root = model.nodes.get(&model.root_id).cloned();
                            if let Some(root_node) = root {
                                print_syntax_tree(&model, &root_node, 2);
                            }
                        }
                        Err(e) => {
                            println!("    (syntax parse error: {})", e);
                        }
                    }
                    obu_idx += 1;
                }
                Err(e) => {
                    println!("  OBU[{}] parse error: {}", obu_idx, e);
                    obu_idx += 1;
                }
            }
        }
    }

    // Residual note
    if residual {
        println!("\nResidual analysis:");
        println!("  (Residual coefficient extraction requires a full AV1 tile-group decoder.");
        println!(
            "   This is not yet implemented in the CLI. Use the Bitvue GUI for residual views.)"
        );
    }

    // Coding flow note
    if coding_flow {
        println!("\nCoding flow:");
        println!("  (Coding flow analysis requires a full AV1 tile-group decoder.");
        println!(
            "   This is not yet implemented in the CLI. Use the Bitvue GUI for coding flow views.)"
        );
    }

    Ok(())
}

/// Recursively print a syntax tree node with indentation.
fn print_syntax_tree(
    model: &bitvue_core::SyntaxModel,
    node: &bitvue_core::SyntaxNode,
    depth: usize,
) {
    let indent = "  ".repeat(depth);
    match &node.value {
        Some(val) => println!(
            "{}  {}: {} (bits {}-{})",
            indent, node.field_name, val, node.bit_range.start_bit, node.bit_range.end_bit
        ),
        None => println!(
            "{}[{}] (bits {}-{})",
            indent, node.field_name, node.bit_range.start_bit, node.bit_range.end_bit
        ),
    }
    let children = node.children.clone();
    for child_id in &children {
        if let Some(child) = model.nodes.get(child_id).cloned() {
            print_syntax_tree(model, &child, depth + 1);
        }
    }
}
