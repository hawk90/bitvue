//! Per-codec frame analysis extraction functions

use bitvue_av1_codec::overlay_extraction::{
    extract_mv_grid, extract_partition_grid, extract_prediction_mode_grid, extract_qp_grid,
    extract_transform_grid,
};
use bitvue_core::StreamId;

use crate::commands::{
    FrameAnalysisData, MVGridData, MbTypeGridData, MotionVectorData, PartitionGridData,
    PredictionModeGridData, QPGridData, RefIdxGridData, TransformGridData,
};

/// Extract frame analysis for AV1 codec
pub(super) fn extract_av1_analysis(
    file_data: &[u8],
    frame_index: usize,
    core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_av1_analysis: === Starting AV1 analysis extraction ===");
    log::info!("extract_av1_analysis: Analysis started");

    let obu_data = {
        let stream_a_arc = core.get_stream(StreamId::A);
        let stream_a_guard = stream_a_arc.read();

        if let Some(unit_model) = &stream_a_guard.units {
            log::info!("extract_av1_analysis: Unit model found");
            unit_model.units.get(frame_index).and_then(|_unit| {
                log::info!("extract_av1_analysis: Parsing IVF frames...");
                if let Ok((_, ivf_frames)) = bitvue_av1_codec::parse_ivf_frames(file_data) {
                    log::info!("extract_av1_analysis: IVF parsing successful");
                    ivf_frames.get(frame_index).map(|f| {
                        log::info!("extract_av1_analysis: Frame data extracted");
                        f.data.clone()
                    })
                } else {
                    log::warn!("extract_av1_analysis: IVF parsing failed");
                    None
                }
            })
        } else {
            log::warn!("extract_av1_analysis: No unit model found");
            None
        }
    };

    let obu_data = obu_data.ok_or("Frame data not available")?;
    log::info!("extract_av1_analysis: OBU data extracted");

    let qp_grid = extract_qp_grid(&obu_data, frame_index, 20)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = extract_mv_grid(&obu_data, frame_index)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid
                .mv_l0
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mv_l1: grid
                .mv_l1
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mode: grid
                .mode
                .map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| {
                    modes.into_iter().map(|m| m as u8).collect()
                }),
        });

    let partition_grid = extract_partition_grid(&obu_data, frame_index)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid
                .blocks
                .into_iter()
                .map(|b| crate::commands::PartitionBlockData {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height,
                    partition: b.partition as u8,
                    depth: b.depth,
                })
                .collect(),
        });

    let prediction_mode_grid = extract_prediction_mode_grid(&obu_data, frame_index)
        .ok()
        .map(|grid| PredictionModeGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            modes: grid
                .modes
                .into_iter()
                .map(|m| m.map(|pm| pm as u8))
                .collect(),
        });

    let transform_grid = extract_transform_grid(&obu_data, frame_index)
        .ok()
        .map(|grid| TransformGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            tx_sizes: grid
                .tx_sizes
                .into_iter()
                .map(|t| t.map(|tx| tx as u8))
                .collect(),
        });

    let width = qp_grid
        .as_ref()
        .map(|g| g.grid_w * g.block_w)
        .or_else(|| mv_grid.as_ref().map(|g| g.coded_width))
        .or_else(|| partition_grid.as_ref().map(|g| g.coded_width))
        .unwrap_or(1920);

    let height = qp_grid
        .as_ref()
        .map(|g| g.grid_h * g.block_h)
        .or_else(|| mv_grid.as_ref().map(|g| g.coded_height))
        .or_else(|| partition_grid.as_ref().map(|g| g.coded_height))
        .unwrap_or(1080);

    log::info!(
        "extract_av1_analysis: Frame dimensions: {}x{}",
        width,
        height
    );
    log::info!("extract_av1_analysis: === AV1 analysis extraction complete ===");

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid,
        transform_grid,
        mb_type_grid: None,
        ref_idx_grid: None,
    })
}

/// Extract frame analysis for H.264/AVC codec
pub(super) fn extract_avc_analysis(
    file_data: &[u8],
    frame_index: usize,
    _core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_avc_analysis: Extracting AVC analysis");

    let nal_units = bitvue_avc::parse_nal_units(file_data)
        .map_err(|e| format!("Failed to parse NAL units: {}", e))?;

    let sps = nal_units
        .iter()
        .find_map(|nal| {
            if nal.header.nal_unit_type == bitvue_avc::NalUnitType::Sps {
                bitvue_avc::sps::parse_sps(&nal.payload).ok()
            } else {
                None
            }
        })
        .ok_or("No SPS found in stream")?;

    let qp_grid = bitvue_avc::extract_qp_grid(&nal_units, &sps, 26)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = bitvue_avc::extract_mv_grid(&nal_units, &sps)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid
                .mv_l0
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mv_l1: grid
                .mv_l1
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mode: grid
                .mode
                .map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| {
                    modes.into_iter().map(|m| m as u8).collect()
                }),
        });

    let partition_grid = bitvue_avc::extract_partition_grid(&nal_units, &sps)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid
                .blocks
                .into_iter()
                .map(|b| crate::commands::PartitionBlockData {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height,
                    partition: b.partition as u8,
                    depth: b.depth,
                })
                .collect(),
        });

    let width = sps.display_width();
    let height = sps.display_height();

    log::info!(
        "extract_avc_analysis: Returning analysis for frame {} ({}x{})",
        frame_index,
        width,
        height
    );

    let prediction_mode_grid = bitvue_avc::extract_prediction_mode_grid(&nal_units, &sps)
        .ok()
        .map(|(coded_width, coded_height, block_w, block_h, modes)| {
            let grid_w = coded_width / block_w;
            let grid_h = coded_height / block_h;
            PredictionModeGridData {
                coded_width,
                coded_height,
                block_w,
                block_h,
                grid_w,
                grid_h,
                modes,
            }
        });

    let mb_type_grid = bitvue_avc::extract_mb_type_grid(&nal_units, &sps).ok().map(
        |(coded_width, coded_height, block_w, block_h, mb_types)| {
            let grid_w = coded_width / block_w;
            let grid_h = coded_height / block_h;
            MbTypeGridData {
                coded_width,
                coded_height,
                block_w,
                block_h,
                grid_w,
                grid_h,
                mb_types,
            }
        },
    );

    let ref_idx_grid = bitvue_avc::extract_ref_idx_grid(&nal_units, &sps).ok().map(
        |(coded_width, coded_height, block_w, block_h, ref_idx_l0, ref_idx_l1)| {
            let grid_w = coded_width / block_w;
            let grid_h = coded_height / block_h;
            RefIdxGridData {
                coded_width,
                coded_height,
                block_w,
                block_h,
                grid_w,
                grid_h,
                ref_idx_l0,
                ref_idx_l1,
            }
        },
    );

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid,
        transform_grid: None,
        mb_type_grid,
        ref_idx_grid,
    })
}

/// Extract frame analysis for HEVC/H.265 codec
pub(super) fn extract_hevc_analysis(
    file_data: &[u8],
    frame_index: usize,
    _core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_hevc_analysis: Extracting HEVC analysis");

    let nal_units = bitvue_hevc::parse_nal_units(file_data)
        .map_err(|e| format!("Failed to parse NAL units: {}", e))?;

    let sps = nal_units
        .iter()
        .find_map(|nal| {
            if nal.header.nal_unit_type == bitvue_hevc::NalUnitType::SpsNut {
                bitvue_hevc::sps::parse_sps(&nal.payload).ok()
            } else {
                None
            }
        })
        .ok_or("No SPS found in stream")?;

    let qp_grid = bitvue_hevc::extract_qp_grid(&nal_units, &sps, 26)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = bitvue_hevc::extract_mv_grid(&nal_units, &sps)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid
                .mv_l0
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mv_l1: grid
                .mv_l1
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mode: grid
                .mode
                .map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| {
                    modes.into_iter().map(|m| m as u8).collect()
                }),
        });

    let partition_grid = bitvue_hevc::extract_partition_grid(&nal_units, &sps)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid
                .blocks
                .into_iter()
                .map(|b| crate::commands::PartitionBlockData {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height,
                    partition: b.partition as u8,
                    depth: b.depth,
                })
                .collect(),
        });

    let prediction_mode_grid = bitvue_hevc::extract_prediction_mode_grid(&nal_units, &sps)
        .ok()
        .map(|(coded_width, coded_height, block_w, block_h, modes)| {
            let grid_w = coded_width / block_w;
            let grid_h = coded_height / block_h;
            PredictionModeGridData {
                coded_width,
                coded_height,
                block_w,
                block_h,
                grid_w,
                grid_h,
                modes,
            }
        });

    let width = sps.pic_width_in_luma_samples;
    let height = sps.pic_height_in_luma_samples;

    log::info!(
        "extract_hevc_analysis: Returning analysis for frame {} ({}x{})",
        frame_index,
        width,
        height
    );

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid,
        transform_grid: None,
        mb_type_grid: None,
        ref_idx_grid: None,
    })
}

/// Extract frame analysis for VP9 codec
pub(super) fn extract_vp9_analysis(
    file_data: &[u8],
    frame_index: usize,
    _core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_vp9_analysis: Extracting VP9 analysis");

    let stream = bitvue_vp9::parse_vp9(file_data)
        .map_err(|e| format!("Failed to parse VP9 stream: {}", e))?;

    let frame_header = stream
        .frames
        .get(frame_index)
        .ok_or("Frame index out of bounds")?;

    let frame_payload = stream.frame_payloads.get(frame_index).map(|v| v.as_slice());

    let qp_grid = bitvue_vp9::extract_qp_grid_with_data(frame_header, frame_payload)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = bitvue_vp9::extract_mv_grid(frame_header)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid
                .mv_l0
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mv_l1: grid
                .mv_l1
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mode: grid
                .mode
                .map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| {
                    modes.into_iter().map(|m| m as u8).collect()
                }),
        });

    let partition_grid = bitvue_vp9::extract_partition_grid(frame_header)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid
                .blocks
                .into_iter()
                .map(|b| crate::commands::PartitionBlockData {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height,
                    partition: b.partition as u8,
                    depth: b.depth,
                })
                .collect(),
        });

    let width = frame_header.width;
    let height = frame_header.height;

    log::info!(
        "extract_vp9_analysis: Returning analysis for frame {} ({}x{})",
        frame_index,
        width,
        height
    );

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid: None,
        transform_grid: None,
        mb_type_grid: None,
        ref_idx_grid: None,
    })
}

/// Extract frame analysis for VVC/H.266 codec
pub(super) fn extract_vvc_analysis(
    file_data: &[u8],
    frame_index: usize,
    _core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_vvc_analysis: Extracting VVC analysis");

    let nal_units = bitvue_vvc::parse_nal_units(file_data)
        .map_err(|e| format!("Failed to parse NAL units: {}", e))?;

    let sps = nal_units
        .iter()
        .find_map(|nal| {
            if nal.header.nal_unit_type == bitvue_vvc::NalUnitType::SpsNut {
                bitvue_vvc::sps::parse_sps(&nal.payload).ok()
            } else {
                None
            }
        })
        .ok_or("No SPS found in stream")?;

    let qp_grid = bitvue_vvc::extract_qp_grid(&nal_units, &sps, 26)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = bitvue_vvc::extract_mv_grid(&nal_units, &sps)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid
                .mv_l0
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mv_l1: grid
                .mv_l1
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mode: grid
                .mode
                .map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| {
                    modes.into_iter().map(|m| m as u8).collect()
                }),
        });

    let partition_grid = bitvue_vvc::extract_partition_grid(&nal_units, &sps)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid
                .blocks
                .into_iter()
                .map(|b| crate::commands::PartitionBlockData {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height,
                    partition: b.partition as u8,
                    depth: b.depth,
                })
                .collect(),
        });

    let width = sps.sps_pic_width_max_in_luma_samples;
    let height = sps.sps_pic_height_max_in_luma_samples;

    log::info!(
        "extract_vvc_analysis: Returning analysis for frame {} ({}x{})",
        frame_index,
        width,
        height
    );

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid: None,
        transform_grid: None,
        mb_type_grid: None,
        ref_idx_grid: None,
    })
}

/// Extract frame analysis for AV3 codec
pub(super) fn extract_av3_analysis(
    file_data: &[u8],
    frame_index: usize,
    _core: &bitvue_core::Core,
) -> Result<FrameAnalysisData, String> {
    log::info!("extract_av3_analysis: Extracting AV3 analysis");

    let stream = bitvue_av3_codec::parse_av3(file_data)
        .map_err(|e| format!("Failed to parse AV3 stream: {}", e))?;

    let frame_header = stream
        .frame_headers
        .get(frame_index)
        .ok_or("Frame index out of bounds")?;

    let qp_grid = bitvue_av3_codec::extract_qp_grid(frame_header)
        .ok()
        .map(|grid| QPGridData {
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            block_w: grid.block_w,
            block_h: grid.block_h,
            qp: grid.qp,
            qp_min: grid.qp_min,
            qp_max: grid.qp_max,
        });

    let mv_grid = bitvue_av3_codec::extract_mv_grid(frame_header)
        .ok()
        .map(|grid| MVGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            block_w: grid.block_w,
            block_h: grid.block_h,
            grid_w: grid.grid_w,
            grid_h: grid.grid_h,
            mv_l0: grid
                .mv_l0
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mv_l1: grid
                .mv_l1
                .into_iter()
                .map(|mv| MotionVectorData {
                    dx_qpel: mv.dx_qpel,
                    dy_qpel: mv.dy_qpel,
                })
                .collect(),
            mode: grid
                .mode
                .map(|modes: Vec<bitvue_core::mv_overlay::BlockMode>| {
                    modes.into_iter().map(|m| m as u8).collect()
                }),
        });

    let partition_grid = bitvue_av3_codec::extract_partition_grid(frame_header)
        .ok()
        .map(|grid| PartitionGridData {
            coded_width: grid.coded_width,
            coded_height: grid.coded_height,
            sb_size: grid.sb_size,
            blocks: grid
                .blocks
                .into_iter()
                .map(|b| crate::commands::PartitionBlockData {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height,
                    partition: b.partition as u8,
                    depth: b.depth,
                })
                .collect(),
        });

    let width = frame_header.width;
    let height = frame_header.height;

    log::info!(
        "extract_av3_analysis: Returning analysis for frame {} ({}x{})",
        frame_index,
        width,
        height
    );

    Ok(FrameAnalysisData {
        frame_index,
        width,
        height,
        qp_grid,
        mv_grid,
        partition_grid,
        prediction_mode_grid: None,
        transform_grid: None,
        mb_type_grid: None,
        ref_idx_grid: None,
    })
}
