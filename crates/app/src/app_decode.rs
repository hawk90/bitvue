//! Decode coordinator methods for BitvueApp

use crate::bitvue_app::BitvueApp;
use eframe::egui;

/// Decode-related methods
pub trait BitvueAppDecode {
    fn poll_decode_results(&mut self, ctx: &egui::Context);
}

impl BitvueAppDecode for BitvueApp {
    /// Poll for async decode results (non-blocking)
    fn poll_decode_results(&mut self, ctx: &egui::Context) {
        use bitvue_core::stream_state::FrameModel;

        let results = self.decoder.poll_results();

        if !results.is_empty() {
            tracing::info!(
                "🎥 poll_decode_results: Processing {} decode result(s)",
                results.len()
            );
        }

        for result in results {
            match result.cached_frame {
                Ok(cached) => {
                    tracing::info!(
                        "🎥 ✅ Decode complete: stream {:?}, frame {}, {}x{}, decoded={}",
                        result.stream_id,
                        result.frame_index,
                        cached.width,
                        cached.height,
                        cached.decoded
                    );

                    // Get stream state and store cached frame
                    let stream = self.core.get_stream(result.stream_id);
                    {
                        let mut state = stream.write();
                        if state.frames.is_none() {
                            state.frames = Some(FrameModel::new());
                        }
                        if let Some(frames) = &mut state.frames {
                            frames.insert_lru(cached.clone());
                        }
                    }

                    // Update player display
                    let color_image = egui::ColorImage::from_rgb(
                        [cached.width as usize, cached.height as usize],
                        &cached.rgb_data,
                    );
                    self.workspaces.player.set_frame(ctx, color_image);

                    // Update YUV viewer
                    if let (Some(y), Some(u), Some(v)) =
                        (&cached.y_plane, &cached.u_plane, &cached.v_plane)
                    {
                        self.panels.yuv_viewer_mut().set_yuv_data(
                            ctx,
                            y,
                            u,
                            v,
                            cached.width,
                            cached.height,
                        );
                    }

                    // Clear pending decode state
                    self.decoder
                        .clear_pending_if_matches(result.stream_id, result.frame_index);
                }
                Err(e) => {
                    tracing::error!(
                        "Async decode failed: stream {:?}, frame {}: {}",
                        result.stream_id,
                        result.frame_index,
                        e
                    );

                    // Clear pending decode state even on error
                    self.decoder
                        .clear_pending_if_matches(result.stream_id, result.frame_index);
                }
            }
        }

        // Request repaint if there's pending work
        if self.decoder.has_pending_work() {
            ctx.request_repaint();
        }
    }
}
