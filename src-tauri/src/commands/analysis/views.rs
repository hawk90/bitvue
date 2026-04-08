//! Advanced analysis view commands: coding flow, residuals, deblocking

use bitvue_av1_codec::advanced_features::{
    extract_cdef_data, extract_film_grain_data, extract_loop_restoration_data,
    extract_super_resolution_data,
};
use bitvue_core::StreamId;
use serde::{Deserialize, Serialize};

use crate::commands::AppState;
use super::{validate_frame_index, load_file_data_and_codec, extract_analysis_by_codec, extract_stream_dimensions, detect_codec_from_path};

// =============================================================================
// Coding Flow
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingFlowData {
    pub frame_index: usize,
    pub stages: Vec<CodingStage>,
    pub current_stage: String,
    pub codec_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingStage {
    pub id: String,
    pub label: String,
    pub completed: bool,
    pub data_size: Option<usize>,
}

#[tauri::command]
pub async fn get_coding_flow_analysis(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<CodingFlowData, String> {
    log::info!("get_coding_flow_analysis: Frame {}", frame_index);

    validate_frame_index(&state, frame_index)?;

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a = core.get_stream(StreamId::A);
    let stream_a = stream_a.read();
    let file_path = stream_a.file_path.as_ref().ok_or("No file loaded")?.clone();
    let codec = detect_codec_from_path(file_path.to_str().ok_or("Invalid path")?);
    let units = stream_a.units.as_ref().ok_or("No units loaded")?;
    let frame_unit = units.units.get(frame_index).ok_or("Frame not found")?;
    let raw_bytes = frame_unit.size;

    let stages = vec![
        CodingStage {
            id: "input".to_string(),
            label: "Input (compressed)".to_string(),
            completed: true,
            data_size: if raw_bytes > 0 { Some(raw_bytes) } else { None },
        },
        CodingStage {
            id: "prediction".to_string(),
            label: "Prediction residual".to_string(),
            completed: false,
            data_size: None,
        },
        CodingStage {
            id: "transform".to_string(),
            label: "Transform coefficients".to_string(),
            completed: false,
            data_size: None,
        },
        CodingStage {
            id: "quantization".to_string(),
            label: "Quantization".to_string(),
            completed: false,
            data_size: None,
        },
        CodingStage {
            id: "entropy".to_string(),
            label: "Entropy coding".to_string(),
            completed: true,
            data_size: if raw_bytes > 0 { Some(raw_bytes) } else { None },
        },
    ];

    let codec_features = match codec.as_str() {
        "av1" => vec!["Directional Intra Pred".to_string(), "Compound Prediction".to_string()],
        "hevc" => vec!["35 Intra Modes".to_string(), "Advanced Motion Vector Pred".to_string()],
        "vvc" => vec!["67 Intra Modes".to_string(), "GPM/Combine Pred".to_string()],
        "avc" | "h264" => vec!["CAVLC/CABAC".to_string(), "IPCM Prediction".to_string()],
        _ => vec!["Standard Features".to_string()],
    };

    Ok(CodingFlowData {
        frame_index,
        stages,
        current_stage: "input".to_string(),
        codec_features,
    })
}

// =============================================================================
// Residuals
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidualAnalysisData {
    pub frame_index: usize,
    pub width: u32,
    pub height: u32,
    pub coefficient_stats: CoefficientStats,
    pub block_residuals: Vec<BlockResidualData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoefficientStats {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub variance: f32,
    pub energy: f64,
    pub zero_count: usize,
    pub non_zero_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockResidualData {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub energy: f32,
    pub max_coeff: f32,
    pub non_zeros: usize,
}

/// Get residual analysis for a frame
///
/// Returns a QP-based approximation of residual energy per block.
/// Energy is approximated as `(51 - qp) / 51 * 100`, where higher QP
/// means more quantization and lower residual energy. This is an approximation
/// — actual CAVLC/CABAC coefficient decoding is not yet implemented.
#[tauri::command]
pub async fn get_residual_analysis(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<ResidualAnalysisData, String> {
    log::info!("get_residual_analysis: Frame {}", frame_index);

    validate_frame_index(&state, frame_index)?;

    let (file_data, codec) = load_file_data_and_codec(&state).await?;

    let analysis = {
        let core = state.core.lock().map_err(|e| e.to_string())?;
        extract_analysis_by_codec(&file_data, frame_index, &core, &codec)
    };

    let analysis = analysis.map_err(|e| format!("QP extraction failed: {}", e))?;

    let qp_grid = analysis.qp_grid
        .ok_or("No QP grid available for this frame")?;

    let width = analysis.width;
    let height = analysis.height;
    let block_w = qp_grid.block_w;
    let block_h = qp_grid.block_h;
    let grid_w = qp_grid.grid_w as usize;
    let grid_h = qp_grid.grid_h as usize;
    let missing = -1i16;

    let mut block_residuals: Vec<BlockResidualData> = Vec::with_capacity(grid_w * grid_h);
    for by in 0..grid_h {
        for bx in 0..grid_w {
            let idx = by * grid_w + bx;
            let qp = if idx < qp_grid.qp.len() { qp_grid.qp[idx] } else { missing };
            let effective_qp = if qp == missing { 26i16 } else { qp.max(0).min(51) };

            let energy = (51 - effective_qp) as f32 / 51.0 * 100.0;
            let max_coeff = energy * 2.55;
            let blocks_area = (block_w * block_h) as f32;
            let non_zeros = (energy / 100.0 * blocks_area / 4.0) as usize;

            block_residuals.push(BlockResidualData {
                x: bx as u32 * block_w,
                y: by as u32 * block_h,
                width: block_w,
                height: block_h,
                energy,
                max_coeff,
                non_zeros,
            });
        }
    }

    let n = block_residuals.len();
    if n == 0 {
        return Err("No blocks in QP grid".to_string());
    }

    let energies: Vec<f32> = block_residuals.iter().map(|b| b.energy).collect();
    let min_e = energies.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_e = energies.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mean_e = energies.iter().sum::<f32>() / n as f32;
    let variance_e = (energies.iter().map(|&e| (e - mean_e).powi(2)).sum::<f32>() / n as f32).sqrt();
    let energy_sum: f64 = energies.iter().map(|&e| (e * e) as f64).sum();
    let total_non_zeros: usize = block_residuals.iter().map(|b| b.non_zeros).sum();
    let total_coeffs = n * (block_w * block_h) as usize;

    Ok(ResidualAnalysisData {
        frame_index,
        width,
        height,
        coefficient_stats: CoefficientStats {
            min: min_e,
            max: max_e,
            mean: mean_e,
            variance: variance_e,
            energy: energy_sum,
            zero_count: total_coeffs.saturating_sub(total_non_zeros),
            non_zero_count: total_non_zeros,
        },
        block_residuals,
    })
}

// =============================================================================
// Deblocking
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeblockingAnalysisData {
    pub frame_index: usize,
    pub width: u32,
    pub height: u32,
    pub boundaries: Vec<BoundaryEdgeData>,
    pub params: DeblockingParams,
    pub stats: DeblockingStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryEdgeData {
    pub x: u32,
    pub y: u32,
    pub length: u32,
    pub orientation: String,
    pub strength: f32,
    pub filtered: bool,
    pub bs: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeblockingParams {
    pub beta_offset: i8,
    pub tc_offset: i8,
    pub filter_strength: u8,
    pub chroma_edge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeblockingStats {
    pub total_boundaries: usize,
    pub filtered_boundaries: usize,
    pub strong_boundaries: usize,
    pub weak_boundaries: usize,
}

#[tauri::command]
pub async fn get_deblocking_analysis(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<DeblockingAnalysisData, String> {
    log::info!("get_deblocking_analysis: Frame {}", frame_index);

    validate_frame_index(&state, frame_index)?;

    // Extract frame type before await (guards must be dropped)
    let is_intra = {
        let core = state.core.lock().map_err(|e| e.to_string())?;
        let stream_a = core.get_stream(StreamId::A);
        let stream_a = stream_a.read();
        let units = stream_a.units.as_ref().ok_or("No units loaded")?;
        let frame_data = units.units.get(frame_index).ok_or("Frame not found")?;
        frame_data.frame_type.as_deref() == Some("I")
    };
    let base_bs: u8 = if is_intra { 2 } else { 1 };

    let (file_data, codec) = load_file_data_and_codec(&state).await?;
    let (width, height) = extract_stream_dimensions(&file_data, &codec);

    let block_size = 8u32;
    let mut boundaries = Vec::new();

    for y in (0..height).step_by(block_size as usize).take(30) {
        for x in (block_size..width).step_by(block_size as usize).take(40) {
            let bs = if (x % 16 == 0) || is_intra { base_bs } else { 0 };
            if bs > 0 {
                boundaries.push(BoundaryEdgeData {
                    x, y, length: block_size,
                    orientation: "vertical".to_string(),
                    strength: bs as f32, filtered: true, bs,
                });
            }
        }
    }

    for y in (block_size..height).step_by(block_size as usize).take(30) {
        for x in (0..width).step_by(block_size as usize).take(40) {
            let bs = if (y % 16 == 0) || is_intra { base_bs } else { 0 };
            if bs > 0 {
                boundaries.push(BoundaryEdgeData {
                    x, y, length: block_size,
                    orientation: "horizontal".to_string(),
                    strength: bs as f32, filtered: true, bs,
                });
            }
        }
    }

    let total_count = boundaries.len();
    let filtered_count = boundaries.iter().filter(|b| b.filtered).count();
    let strong_count = boundaries.iter().filter(|b| b.bs >= 3).count();
    let weak_count = boundaries.iter().filter(|b| b.bs > 0 && b.bs < 3).count();

    Ok(DeblockingAnalysisData {
        frame_index,
        width,
        height,
        boundaries,
        params: DeblockingParams {
            beta_offset: 0,
            tc_offset: 0,
            filter_strength: 1,
            chroma_edge: true,
        },
        stats: DeblockingStats {
            total_boundaries: total_count,
            filtered_boundaries: filtered_count,
            strong_boundaries: strong_count,
            weak_boundaries: weak_count,
        },
    })
}

/// AV1 features response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Av1FeaturesData {
    pub frame_index: usize,
    pub cdef: Option<CdefDataResponse>,
    pub loop_restoration: Option<LoopRestorationResponse>,
    pub film_grain: Option<FilmGrainResponse>,
    pub super_resolution: Option<SuperResolutionResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdefDataResponse {
    pub width: u32,
    pub height: u32,
    pub block_size: u32,
    pub blocks: Vec<CdefBlockResponse>,
    pub damping: u8,
    pub y_primary_strength: u8,
    pub y_secondary_strength: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdefBlockResponse {
    pub x: u32,
    pub y: u32,
    pub size: u32,
    pub direction: u8,
    pub strength: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopRestorationResponse {
    pub width: u32,
    pub height: u32,
    pub unit_size: u32,
    pub y_type: u8,
    pub units: Vec<RestorationUnitResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorationUnitResponse {
    pub x: u32,
    pub y: u32,
    pub size: u32,
    pub restoration_type: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilmGrainResponse {
    pub enabled: bool,
    pub seed: u64,
    pub scaling_shift: u8,
    pub ar_coeff_lag: u8,
    pub chroma_scaling_from_luma: bool,
    pub overlap: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperResolutionResponse {
    pub enabled: bool,
    pub scale_denominator: u8,
    pub upscaled_width: u32,
    pub upscaled_height: u32,
}

/// Get AV1 advanced features (CDEF, Loop Restoration, Film Grain, Super Resolution)
#[tauri::command]
pub async fn get_av1_features(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<Av1FeaturesData, String> {
    validate_frame_index(&state, frame_index)?;

    let (file_data, codec) = load_file_data_and_codec(&state).await?;

    if !matches!(codec.to_lowercase().as_str(), "av1") {
        return Ok(Av1FeaturesData { frame_index, cdef: None, loop_restoration: None, film_grain: None, super_resolution: None });
    }

    // Parse IVF and get frame header
    // Parse IVF frames then extract OBUs to find the frame header OBU
    let frame_header = bitvue_av1_codec::parse_ivf_frames(&file_data)
        .ok()
        .and_then(|(_, frames)| frames.into_iter().nth(frame_index))
        .and_then(|f| {
            bitvue_av1_codec::parse_all_obus(&f.data).ok().and_then(|obus| {
                obus.into_iter()
                    .find(|o| matches!(o.obu_type, bitvue_av1_codec::ObuType::FrameHeader | bitvue_av1_codec::ObuType::Frame))
                    .and_then(|o| bitvue_av1_codec::parse_frame_header_basic(&o.payload).ok())
            })
        });

    let Some(fh) = frame_header else {
        return Ok(Av1FeaturesData { frame_index, cdef: None, loop_restoration: None, film_grain: None, super_resolution: None });
    };

    let cdef = extract_cdef_data(&fh).map(|d| CdefDataResponse {
        width: d.width,
        height: d.height,
        block_size: 8,
        blocks: d.block_strengths.iter().map(|b| CdefBlockResponse {
            x: b.x, y: b.y, size: b.size, direction: b.direction, strength: b.strength,
        }).collect(),
        damping: d.damping,
        y_primary_strength: d.y_primary_strength,
        y_secondary_strength: d.y_secondary_strength,
    });

    let loop_restoration = extract_loop_restoration_data(&fh).map(|d| LoopRestorationResponse {
        width: d.width,
        height: d.height,
        unit_size: d.unit_size,
        y_type: d.y_restoration_type as u8,
        units: d.units.iter().map(|u| RestorationUnitResponse {
            x: u.x, y: u.y, size: u.size, restoration_type: u.restoration_type as u8,
        }).collect(),
    });

    let film_grain = extract_film_grain_data(&fh).map(|d| FilmGrainResponse {
        enabled: d.enabled,
        seed: d.seed,
        scaling_shift: d.scaling_shift,
        ar_coeff_lag: d.ar_coeff_lag,
        chroma_scaling_from_luma: d.chroma_scaling_from_luma,
        overlap: d.overlap,
    });

    let super_resolution = extract_super_resolution_data(&fh).map(|d| SuperResolutionResponse {
        enabled: d.enabled,
        scale_denominator: d.scale_denominator,
        upscaled_width: d.upscaled_width,
        upscaled_height: d.upscaled_height,
    });

    Ok(Av1FeaturesData { frame_index, cdef, loop_restoration, film_grain, super_resolution })
}
