//! YUV diff comparison methods for BitvueApp

use crate::bitvue_app::BitvueApp;
use crate::helpers::count_frames;
use crate::yuv_diff::{BitDepth, ChromaSubsampling};

/// YUV diff methods
pub trait BitvueAppYuvDiff {
    fn load_reference_yuv_frame(&mut self, frame_index: usize, width: u32, height: u32) -> bool;
    fn calculate_diff_metrics(&mut self, current_frame: &bitvue_decode::DecodedFrame);
    fn export_diff_metrics_csv(
        &mut self,
        csv_path: &std::path::Path,
        export_all_frames: bool,
    ) -> Result<(), String>;
}

impl BitvueAppYuvDiff for BitvueApp {
    /// Load reference YUV frame for diff comparison
    /// Returns true if loading was successful
    fn load_reference_yuv_frame(&mut self, frame_index: usize, width: u32, height: u32) -> bool {
        let ref_path = match &self.yuv_diff_settings.reference_file {
            Some(path) => path.clone(),
            None => return false,
        };

        // Convert ChromaSubsampling and BitDepth to bitvue_decode types
        let chroma = match self.yuv_diff_settings.subsampling {
            ChromaSubsampling::Yuv420 => bitvue_decode::ChromaSubsampling::Yuv420,
            ChromaSubsampling::Yuv422 => bitvue_decode::ChromaSubsampling::Yuv422,
            ChromaSubsampling::Yuv444 => bitvue_decode::ChromaSubsampling::Yuv444,
        };

        let bit_depth = match self.yuv_diff_settings.bit_depth {
            BitDepth::Bit8 => bitvue_decode::BitDepth::Bit8,
            BitDepth::Bit10 => bitvue_decode::BitDepth::Bit10,
            BitDepth::Bit12 => bitvue_decode::BitDepth::Bit12,
        };

        // Create YUV file params
        let params = bitvue_decode::YuvFileParams {
            width,
            height,
            chroma_subsampling: chroma,
            bit_depth,
            frame_rate: (25, 1), // Default, not critical for diff
        };

        // Try to load the YUV file
        match bitvue_decode::YuvLoader::open(&ref_path, Some(params)) {
            Ok(mut loader) => {
                // Apply frame offset
                let target_frame = if self.yuv_diff_settings.frame_offset >= 0 {
                    frame_index + self.yuv_diff_settings.frame_offset as usize
                } else {
                    frame_index.saturating_sub((-self.yuv_diff_settings.frame_offset) as usize)
                };

                // Read frames until we reach target
                let mut frame = None;
                for _ in 0..=target_frame {
                    match loader.read_frame() {
                        Ok(Some(f)) => frame = Some(f),
                        Ok(None) => {
                            self.set_error(format!("Reference YUV EOF at frame {}", target_frame));
                            return false;
                        }
                        Err(e) => {
                            self.set_error(format!("Failed to read reference YUV: {}", e));
                            return false;
                        }
                    }
                }

                if let Some(f) = frame {
                    // Validate dimensions
                    if f.width != width || f.height != height {
                        self.set_error(format!(
                            "Reference YUV dimensions mismatch: {}x{} vs {}x{}",
                            f.width, f.height, width, height
                        ));
                        return false;
                    }

                    self.yuv_diff_settings.reference_frame = Some(f);
                    self.yuv_diff_settings.reference_dimensions = Some((width, height));
                    true
                } else {
                    false
                }
            }
            Err(e) => {
                self.set_error(format!("Failed to open reference YUV: {}", e));
                false
            }
        }
    }

    /// Calculate PSNR/SSIM metrics between current frame and reference
    fn calculate_diff_metrics(&mut self, current_frame: &bitvue_decode::DecodedFrame) {
        let ref_frame = match &self.yuv_diff_settings.reference_frame {
            Some(f) => f,
            None => return,
        };

        // Use BlockMetricsCalculator for PSNR/SSIM
        let calculator = bitvue_core::BlockMetricsCalculator::new(
            current_frame.width, // Use full frame as one block
            current_frame.bit_depth,
        );

        // Calculate Y-plane metrics
        let psnr_y = calculator.calculate_psnr_block(&current_frame.y_plane, &ref_frame.y_plane);
        let ssim_y = calculator.calculate_ssim_block(&current_frame.y_plane, &ref_frame.y_plane);
        self.yuv_diff_settings.psnr_value = Some(psnr_y);
        self.yuv_diff_settings.ssim_value = Some(ssim_y);

        // Calculate U-plane metrics (if available)
        let (psnr_u, ssim_u) =
            if let (Some(current_u), Some(ref_u)) = (&current_frame.u_plane, &ref_frame.u_plane) {
                let psnr = calculator.calculate_psnr_block(current_u, ref_u);
                let ssim = calculator.calculate_ssim_block(current_u, ref_u);
                (psnr, ssim)
            } else {
                (0.0, 0.0)
            };

        // Calculate V-plane metrics (if available)
        let (psnr_v, ssim_v) =
            if let (Some(current_v), Some(ref_v)) = (&current_frame.v_plane, &ref_frame.v_plane) {
                let psnr = calculator.calculate_psnr_block(current_v, ref_v);
                let ssim = calculator.calculate_ssim_block(current_v, ref_v);
                (psnr, ssim)
            } else {
                (0.0, 0.0)
            };

        // Calculate YUV-PSNR (weighted average: 6:1:1 for Y:U:V per ITU-T)
        let psnr_yuv = if psnr_u > 0.0 && psnr_v > 0.0 {
            (6.0 * psnr_y + psnr_u + psnr_v) / 8.0
        } else {
            psnr_y // Monochrome - only Y-plane
        };
        self.yuv_diff_settings.psnr_yuv_value = Some(psnr_yuv);

        // Calculate YUV-SSIM (simple average)
        let ssim_yuv = if ssim_u > 0.0 && ssim_v > 0.0 {
            (ssim_y + ssim_u + ssim_v) / 3.0
        } else {
            ssim_y // Monochrome - only Y-plane
        };
        self.yuv_diff_settings.ssim_yuv_value = Some(ssim_yuv);

        tracing::debug!(
            "Diff metrics: PSNR-Y={:.2} dB, PSNR-YUV={:.2} dB, SSIM-Y={:.4}, SSIM-YUV={:.4}",
            psnr_y,
            psnr_yuv,
            ssim_y,
            ssim_yuv
        );
    }

    /// Export diff metrics (PSNR/SSIM) to CSV file
    /// Supports both single-frame and multi-frame export
    fn export_diff_metrics_csv(
        &mut self,
        csv_path: &std::path::Path,
        export_all_frames: bool,
    ) -> Result<(), String> {
        use std::io::Write;

        // Get current frame from stream
        let stream_a = self.core.get_stream(bitvue_core::StreamId::A);
        let state = stream_a.read();

        // Get dimensions from container
        let (width, height) = match &state.container {
            Some(c) => match (c.width, c.height) {
                (Some(w), Some(h)) => (w, h),
                _ => return Err("No frame dimensions available".to_string()),
            },
            None => return Err("No stream loaded".to_string()),
        };

        // Get total frame count
        let frame_count = if export_all_frames {
            if let Some(units) = &state.units {
                count_frames(&units.units)
            } else {
                return Err("No frame data available".to_string());
            }
        } else {
            1 // Just current frame
        };

        // Get current frame index for single-frame export
        let start_frame = if export_all_frames {
            0
        } else {
            let selection = self.core.get_selection();
            let sel_guard = selection.read();
            match &sel_guard.temporal {
                Some(bitvue_core::TemporalSelection::Block { frame_index, .. }) => *frame_index,
                Some(bitvue_core::TemporalSelection::Point { frame_index }) => *frame_index,
                Some(bitvue_core::TemporalSelection::Range { start, .. }) => *start,
                Some(bitvue_core::TemporalSelection::Marker { frame_index }) => *frame_index,
                None => 0,
            }
        };

        // Create CSV file
        let mut file = std::fs::File::create(csv_path)
            .map_err(|e| format!("Failed to create CSV file: {}", e))?;

        // Write CSV header with all plane metrics
        writeln!(
            file,
            "Frame,PSNR-Y(dB),PSNR-U(dB),PSNR-V(dB),PSNR-YUV(dB),SSIM-Y,SSIM-U,SSIM-V,SSIM-YUV"
        )
        .map_err(|e| format!("Failed to write CSV header: {}", e))?;

        let mut exported_count = 0;
        let mut skipped_count = 0;

        // Drop state guard to avoid holding lock during iteration
        drop(state);

        // Iterate through frames
        let end_frame = if export_all_frames {
            frame_count
        } else {
            start_frame + 1
        };

        for frame_idx in start_frame..end_frame {
            // Try to load reference frame
            if !self.load_reference_yuv_frame(frame_idx, width, height) {
                tracing::warn!("Skipping frame {}: failed to load reference", frame_idx);
                skipped_count += 1;
                continue;
            }

            // Get reference frame we just loaded
            let ref_frame = match &self.yuv_diff_settings.reference_frame {
                Some(f) => f,
                None => {
                    skipped_count += 1;
                    continue;
                }
            };

            // Try to get decoded frame from stream cache
            let stream_a = self.core.get_stream(bitvue_core::StreamId::A);
            let mut state = stream_a.write();

            let (decoded_y_plane, decoded_u_plane, decoded_v_plane) =
                if let Some(frames) = &mut state.frames {
                    if let Some(cached_frame) = frames.get(frame_idx) {
                        if cached_frame.decoded {
                            // Clone YUV plane data from cached frame
                            (
                                cached_frame.y_plane.clone(),
                                cached_frame.u_plane.clone(),
                                cached_frame.v_plane.clone(),
                            )
                        } else {
                            (None, None, None)
                        }
                    } else {
                        (None, None, None)
                    }
                } else {
                    (None, None, None)
                };

            // Drop lock before calculation
            drop(state);

            // Calculate metrics for all planes if we have both decoded and reference YUV data
            let (psnr_y, psnr_u, psnr_v, psnr_yuv, ssim_y, ssim_u, ssim_v, ssim_yuv) =
                if let Some(decoded_y) = decoded_y_plane {
                    let calculator =
                        bitvue_core::BlockMetricsCalculator::new(width, ref_frame.bit_depth);

                    // Calculate Y-plane metrics
                    let psnr_y = calculator.calculate_psnr_block(&decoded_y, &ref_frame.y_plane);
                    let ssim_y = calculator.calculate_ssim_block(&decoded_y, &ref_frame.y_plane);

                    // Calculate U-plane metrics (if available)
                    let (psnr_u, ssim_u) = if let (Some(decoded_u), Some(ref_u)) =
                        (&decoded_u_plane, &ref_frame.u_plane)
                    {
                        let psnr = calculator.calculate_psnr_block(decoded_u, ref_u);
                        let ssim = calculator.calculate_ssim_block(decoded_u, ref_u);
                        (psnr, ssim)
                    } else {
                        (0.0, 0.0) // Monochrome or U-plane not available
                    };

                    // Calculate V-plane metrics (if available)
                    let (psnr_v, ssim_v) = if let (Some(decoded_v), Some(ref_v)) =
                        (&decoded_v_plane, &ref_frame.v_plane)
                    {
                        let psnr = calculator.calculate_psnr_block(decoded_v, ref_v);
                        let ssim = calculator.calculate_ssim_block(decoded_v, ref_v);
                        (psnr, ssim)
                    } else {
                        (0.0, 0.0) // Monochrome or V-plane not available
                    };

                    // Calculate YUV-PSNR (weighted average: 6:1:1 for Y:U:V per ITU-T)
                    let psnr_yuv = if psnr_u > 0.0 && psnr_v > 0.0 {
                        (6.0 * psnr_y + psnr_u + psnr_v) / 8.0
                    } else {
                        psnr_y // Monochrome - only Y-plane
                    };

                    // Calculate YUV-SSIM (simple average)
                    let ssim_yuv = if ssim_u > 0.0 && ssim_v > 0.0 {
                        (ssim_y + ssim_u + ssim_v) / 3.0
                    } else {
                        ssim_y // Monochrome - only Y-plane
                    };

                    (
                        psnr_y, psnr_u, psnr_v, psnr_yuv, ssim_y, ssim_u, ssim_v, ssim_yuv,
                    )
                } else {
                    // Frame not in cache - skip it
                    tracing::warn!(
                        "Skipping frame {}: not decoded (navigate to frame first)",
                        frame_idx
                    );
                    skipped_count += 1;
                    continue;
                };

            // Write CSV row with all plane metrics
            writeln!(
                file,
                "{},{:.2},{:.2},{:.2},{:.2},{:.4},{:.4},{:.4},{:.4}",
                frame_idx, psnr_y, psnr_u, psnr_v, psnr_yuv, ssim_y, ssim_u, ssim_v, ssim_yuv
            )
            .map_err(|e| format!("Failed to write CSV row: {}", e))?;

            exported_count += 1;

            // Show progress every 10 frames
            if export_all_frames && frame_idx % 10 == 0 {
                tracing::info!(
                    "Exporting progress: {}/{} frames",
                    frame_idx + 1,
                    frame_count
                );
            }
        }

        tracing::info!(
            "Exported metrics to {:?}: {} frames exported, {} skipped",
            csv_path,
            exported_count,
            skipped_count
        );

        if skipped_count > 0 {
            return Err(format!(
                "Export incomplete: {} frames exported, {} skipped (Note: Full multi-frame export requires decoded frames)",
                exported_count, skipped_count
            ));
        }

        Ok(())
    }
}
