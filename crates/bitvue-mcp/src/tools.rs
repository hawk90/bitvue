//! Implementations of the MCP tools (one function per catalogue entry) over the loaded streams.

use crate::ivf::parse_ivf_file;
use crate::state::{validate_path, AppState};
use anyhow::Result;
use bitvue_engine::{StreamId, UnitModel, UnitNode};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Parse stream ID from string
pub(crate) fn parse_stream_id(stream: Option<&str>) -> StreamId {
    match stream.unwrap_or("A") {
        "B" => StreamId::B,
        _ => StreamId::A,
    }
}

pub(crate) fn load_file(args: Value, state: &AppState) -> Result<String> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;

    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    // SECURITY: Validate path is within allowed directories
    let validated_path = match validate_path(path, &state.allowed_paths) {
        Ok(p) => p,
        Err(e) => {
            return Ok(json!({
                "success": false,
                "error": format!("Access denied: {}", e)
            })
            .to_string());
        }
    };

    // Check if file exists (now safe from path traversal)
    if !validated_path.exists() {
        return Ok(json!({
            "success": false,
            "error": format!("File not found: {}", path)
        })
        .to_string());
    }

    // Get file size using validated path
    let file_size = validated_path.metadata().map(|m| m.len()).unwrap_or(0);

    // Get file extension
    let ext = validated_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    // Parse the file based on extension
    let units_result: Result<Vec<UnitNode>, String> = match ext {
        "ivf" | "av1" => {
            tracing::info!("Parsing IVF file: {}", validated_path.display());
            parse_ivf_file(&validated_path, stream_id)
        }
        _ => {
            // For now, only IVF is supported
            return Ok(json!({
                "success": false,
                "error": format!("Unsupported format: {}. Only IVF is currently supported.", ext)
            })
            .to_string());
        }
    };

    match units_result {
        Ok(units) => {
            // Populate the stream state
            let core = state
                .core
                .lock()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

            let stream = core.get_stream(stream_id);
            let mut stream = stream.write();

            let unit_count = units.len();
            let frame_count = units.iter().filter(|u| u.frame_index.is_some()).count();

            stream.units = Some(UnitModel {
                units,
                unit_count,
                frame_count,
            });

            // Update loaded file state
            let mut loaded = state
                .loaded_file
                .lock()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            *loaded = Some(validated_path.clone());

            Ok(json!({
                "success": true,
                "path": path,
                "stream": if stream_id == StreamId::A { "A" } else { "B" },
                "file_size": file_size,
                "format": ext,
                "frame_count": frame_count,
                "message": format!("Successfully loaded {} frames from {}", frame_count, path)
            })
            .to_string())
        }
        Err(e) => Ok(json!({
            "success": false,
            "error": e
        })
        .to_string()),
    }
}

pub(crate) fn get_stream_model(
    state: &AppState,
    stream_id: StreamId,
) -> Result<(UnitModel, PathBuf)> {
    let core = state
        .core
        .lock()
        .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let stream = core.get_stream(stream_id);
    let stream = stream.read();

    let units = stream
        .units
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No data loaded for stream. Use load_file first."))?;

    let loaded = state
        .loaded_file
        .lock()
        .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let path = loaded
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No file loaded"))?
        .clone();

    Ok((units.clone(), path))
}

pub(crate) fn analyze_frame(args: Value, state: &AppState) -> Result<String> {
    let frame_index = args["frame_index"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid frame_index"))? as usize;

    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let (units, path) = get_stream_model(state, stream_id)?;

    // Find the frame
    let frame = units
        .units
        .iter()
        .find(|u| u.frame_index == Some(frame_index))
        .ok_or_else(|| anyhow::anyhow!("Frame {} not found", frame_index))?;

    Ok(json!({
        "frame_index": frame_index,
        "stream": if stream_id == StreamId::A { "A" } else { "B" },
        "file": path.to_string_lossy(),
        "unit_type": frame.unit_type.to_string(),
        "frame_type": frame.frame_type.clone().unwrap_or_else(|| std::sync::Arc::from("Unknown")).to_string(),
        "offset": frame.offset,
        "size": frame.size,
        "pts": frame.pts,
        "dts": frame.dts,
        "qp_avg": frame.qp_avg,
        "ref_frames": frame.ref_frames.clone(),
        "display_name": frame.display_name.to_string()
    })
    .to_string())
}

pub(crate) fn get_qp_map(args: Value, state: &AppState) -> Result<String> {
    let frame_index = args["frame_index"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid frame_index"))? as usize;

    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let (units, _path) = get_stream_model(state, stream_id)?;

    // Find the frame
    let frame = units
        .units
        .iter()
        .find(|u| u.frame_index == Some(frame_index))
        .ok_or_else(|| anyhow::anyhow!("Frame {} not found", frame_index))?;

    let qp = frame.qp_avg.unwrap_or(0);

    // Collect QP statistics across all frames
    let qp_values: Vec<u8> = units.units.iter().filter_map(|u| u.qp_avg).collect();

    let qp_min = qp_values.iter().copied().min().unwrap_or(0);
    let qp_max = qp_values.iter().copied().max().unwrap_or(0);
    let qp_avg = if qp_values.is_empty() {
        0
    } else {
        qp_values.iter().map(|&x| x as u32).sum::<u32>() / qp_values.len() as u32
    } as u8;

    Ok(json!({
        "frame_index": frame_index,
        "frame_qp": qp,
        "stream_qp_stats": {
            "min": qp_min,
            "max": qp_max,
            "avg": qp_avg
        },
        "note": "QP data is per-frame average. Block-level QP requires bitstream parsing."
    })
    .to_string())
}

pub(crate) fn get_motion_vectors(args: Value, state: &AppState) -> Result<String> {
    let frame_index = args["frame_index"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid frame_index"))? as usize;

    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let (units, _path) = get_stream_model(state, stream_id)?;

    // Find the frame
    let frame = units
        .units
        .iter()
        .find(|u| u.frame_index == Some(frame_index))
        .ok_or_else(|| anyhow::anyhow!("Frame {} not found", frame_index))?;

    let ref_idx = frame.ref_frames.clone().unwrap_or_default();

    Ok(json!({
        "frame_index": frame_index,
        "frame_type": frame.frame_type.clone().unwrap_or_else(|| std::sync::Arc::from("Unknown")).to_string(),
        "ref_frames": ref_idx,
        "note": "Motion vector extraction requires bitstream-level parsing. Currently showing reference frame indices."
    }).to_string())
}

pub(crate) fn compare_streams(args: Value, state: &AppState) -> Result<String> {
    let frame_index = args["frame_index"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid frame_index"))? as usize;

    // Get both streams
    let (units_a, path_a) = match get_stream_model(state, StreamId::A) {
        Ok(u) => u,
        Err(e) => {
            return Ok(json!({
                "message": format!("Stream A error: {}", e),
                "note": "Load a file first using load_file"
            })
            .to_string())
        }
    };
    let (units_b, path_b) = match get_stream_model(state, StreamId::B) {
        Ok(u) => u,
        Err(_) => {
            return Ok(json!({
                "message": "Stream B not loaded. Load a second file with load_file to compare.",
                "stream_a": {
                    "file": path_a.to_string_lossy(),
                    "frame_count": units_a.frame_count
                }
            })
            .to_string())
        }
    };

    let frame_a = units_a
        .units
        .iter()
        .find(|u| u.frame_index == Some(frame_index));

    let frame_b = units_b
        .units
        .iter()
        .find(|u| u.frame_index == Some(frame_index));

    match (frame_a, frame_b) {
        (Some(fa), Some(fb)) => {
            let size_diff = if fa.size > 0 && fb.size > 0 {
                (fa.size as f64 - fb.size as f64) / fb.size as f64 * 100.0
            } else {
                0.0
            };

            let qp_a = fa.qp_avg.unwrap_or(0);
            let qp_b = fb.qp_avg.unwrap_or(0);

            Ok(json!({
                "frame_index": frame_index,
                "stream_a": {
                    "file": path_a.to_string_lossy(),
                    "type": fa.frame_type.clone().unwrap_or_else(|| std::sync::Arc::from("Unknown")).to_string(),
                    "size": fa.size,
                    "qp": qp_a,
                    "offset": fa.offset
                },
                "stream_b": {
                    "file": path_b.to_string_lossy(),
                    "type": fb.frame_type.clone().unwrap_or_else(|| std::sync::Arc::from("Unknown")).to_string(),
                    "size": fb.size,
                    "qp": qp_b,
                    "offset": fb.offset
                },
                "comparison": {
                    "size_diff_percent": size_diff,
                    "qp_diff": qp_a as i16 - qp_b as i16,
                    "size_larger": if size_diff > 0.0 { "A" } else if size_diff < 0.0 { "B" } else { "Equal" },
                    "quality_note": if qp_a < qp_b {
                        "Stream A has lower QP (higher quality)"
                    } else if qp_a > qp_b {
                        "Stream B has lower QP (higher quality)"
                    } else {
                        "Both streams have similar QP"
                    }
                }
            }).to_string())
        }
        (Some(fa), None) => Ok(json!({
            "message": format!("Frame {} exists in Stream A but not in Stream B", frame_index),
            "stream_a": {
                "type": fa.frame_type.clone().unwrap_or_else(|| std::sync::Arc::from("Unknown")).to_string(),
                "size": fa.size
            }
        })
        .to_string()),
        (None, Some(fb)) => Ok(json!({
            "message": format!("Frame {} exists in Stream B but not in Stream A", frame_index),
            "stream_b": {
                "type": fb.frame_type.clone().unwrap_or_else(|| std::sync::Arc::from("Unknown")).to_string(),
                "size": fb.size
            }
        })
        .to_string()),
        (None, None) => Ok(json!({
            "message": format!("Frame {} not found in either stream", frame_index)
        })
        .to_string()),
    }
}

pub(crate) fn get_gop_structure(args: Value, state: &AppState) -> Result<String> {
    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let max_frames = args["max_frames"].as_u64().unwrap_or(100) as usize;

    let (units, path) = get_stream_model(state, stream_id)?;

    let frames: Vec<Value> = units
        .units
        .iter()
        .filter(|u| u.frame_index.is_some())
        .take(max_frames)
        .map(|u| {
            json!({
                "frame_index": u.frame_index,
                "type": u.frame_type.clone().unwrap_or_else(|| std::sync::Arc::from("Unknown")).to_string(),
                "size": u.size,
                "qp": u.qp_avg,
                "pts": u.pts,
                "ref_frames": u.ref_frames.clone()
            })
        })
        .collect();

    let i_count = frames.iter().filter(|f| f["type"] == "I").count();
    let p_count = frames.iter().filter(|f| f["type"] == "P").count();
    let b_count = frames.iter().filter(|f| f["type"] == "B").count();

    Ok(json!({
        "file": path.to_string_lossy(),
        "stream": if stream_id == StreamId::A { "A" } else { "B" },
        "total_frames": units.frame_count,
        "shown_frames": frames.len(),
        "frame_type_counts": {
            "I": i_count,
            "P": p_count,
            "B": b_count
        },
        "frames": frames
    })
    .to_string())
}

pub(crate) fn find_decoding_issues(args: Value, state: &AppState) -> Result<String> {
    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let (units, path) = get_stream_model(state, stream_id)?;

    let mut issues = Vec::new();
    let mut warnings = Vec::new();

    // Check for unusually large or small frames
    let sizes: Vec<usize> = units.units.iter().map(|u| u.size).collect();
    if !sizes.is_empty() {
        let avg_size = sizes.iter().copied().sum::<usize>() / sizes.len();
        let max_size = *sizes.iter().max().unwrap();
        let min_size = *sizes.iter().min().unwrap();

        if max_size > avg_size * 10 {
            warnings.push(format!(
                "Found unusually large frame ({} bytes vs avg {} bytes)",
                max_size, avg_size
            ));
        }

        if min_size < avg_size / 10 && min_size > 0 {
            warnings.push(format!(
                "Found unusually small frame ({} bytes vs avg {} bytes)",
                min_size, avg_size
            ));
        }
    }

    // Check for missing QP data
    let qp_missing = units
        .units
        .iter()
        .filter(|u| u.frame_index.is_some() && u.qp_avg.is_none())
        .count();

    if qp_missing > 0 {
        warnings.push(format!("QP data not available for {} frames", qp_missing));
    }

    // Check file path
    if path.to_string_lossy().contains(".html") {
        issues.push("File appears to be an HTML file, not a valid video file".to_string());
    }

    Ok(json!({
        "file": path.to_string_lossy(),
        "stream": if stream_id == StreamId::A { "A" } else { "B" },
        "frames_checked": units.frame_count,
        "issues": issues,
        "warnings": warnings,
        "status": if issues.is_empty() { "No critical issues found" } else { "Issues detected" }
    })
    .to_string())
}

pub(crate) fn get_stream_info(args: Value, state: &AppState) -> Result<String> {
    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);

    let (units, path) = get_stream_model(state, stream_id)?;

    // Calculate total size
    let total_size: u64 = units.units.iter().map(|u| u.size as u64).sum();

    // Get file extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    Ok(json!({
        "file": path.to_string_lossy(),
        "stream": if stream_id == StreamId::A { "A" } else { "B" },
        "format": ext,
        "frame_count": units.frame_count,
        "unit_count": units.unit_count,
        "total_size": total_size,
        "avg_frame_size": if units.frame_count > 0 {
            total_size / units.frame_count as u64
        } else {
            0
        }
    })
    .to_string())
}

pub(crate) fn search_syntax(args: Value, state: &AppState) -> Result<String> {
    let frame_type_filter = args["frame_type"].as_str();
    let min_qp = args["min_qp"].as_u64().map(|x| x as u8);
    let max_qp = args["max_qp"].as_u64().map(|x| x as u8);
    let stream_str = args["stream"].as_str();
    let stream_id = parse_stream_id(stream_str);
    let limit = args["limit"].as_u64().unwrap_or(50) as usize;

    let (units, _path) = get_stream_model(state, stream_id)?;

    let mut results: Vec<Value> = Vec::new();

    for unit in units.units.iter() {
        if results.len() >= limit {
            break;
        }

        let frame_idx = unit.frame_index;
        let ftype = unit.frame_type.as_deref();

        // Apply filters
        if let Some(filter) = frame_type_filter {
            if filter != "all" {
                if let Some(ft) = ftype {
                    if ft != filter {
                        continue;
                    }
                } else {
                    continue;
                }
            }
        }

        if let Some(qp) = unit.qp_avg {
            if let Some(min) = min_qp {
                if qp < min {
                    continue;
                }
            }
            if let Some(max) = max_qp {
                if qp > max {
                    continue;
                }
            }
        }

        results.push(json!({
            "frame_index": frame_idx,
            "type": ftype,
            "size": unit.size,
            "qp": unit.qp_avg,
            "offset": unit.offset
        }));
    }

    Ok(json!({
        "query": {
            "frame_type": frame_type_filter,
            "min_qp": min_qp,
            "max_qp": max_qp
        },
        "results_count": results.len(),
        "results": results
    })
    .to_string())
}

pub(crate) fn list_files(state: &AppState) -> Result<String> {
    let loaded = state
        .loaded_file
        .lock()
        .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let core = state
        .core
        .lock()
        .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let mut files = Vec::new();

    // Check Stream A
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();

    if let Some(units) = &stream_a.units {
        files.push(json!({
            "stream": "A",
            "frame_count": units.frame_count,
            "loaded": true
        }));
    }

    // Check Stream B
    let stream_b = core.get_stream(StreamId::B);
    let stream_b = stream_b.read();

    if let Some(units) = &stream_b.units {
        files.push(json!({
            "stream": "B",
            "frame_count": units.frame_count,
            "loaded": true
        }));
    }

    Ok(json!({
        "loaded_file": loaded.as_ref().map(|p| p.to_string_lossy().to_string()),
        "streams": files
    })
    .to_string())
}
