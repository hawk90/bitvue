//! AV1 tile/superblock quirks that affect timeline visualization without affecting frame
//! identity (S.T0-1.AV1.FrameIdentity.Core.impl.quirks_AV1.001).

use super::{FrameIndexMap, FrameMetadata};

/// AV1-specific quirks for frame identity
///
/// AV1 has several codec-specific features that affect frame identity:
/// 1. Tile-to-superblock mapping: Tiles may affect frame reordering visualization
/// 2. Film grain synthesis: Film grain flags can create "virtual" frames
/// 3. show_existing_frame: References previously decoded frames without new data
///
/// These quirks are documented here but implementation is deferred to AV1 parser integration.
#[derive(Debug, Clone)]
pub struct Av1FrameIdentityQuirks {
    /// Whether this frame uses show_existing_frame (references existing frame)
    pub is_show_existing: bool,

    /// Frame index referenced by show_existing_frame (if applicable)
    pub show_existing_frame_idx: Option<usize>,

    /// Whether film grain synthesis is enabled
    pub has_film_grain: bool,

    /// Number of tiles in this frame (affects spatial parsing)
    pub tile_count: Option<usize>,
}

impl Av1FrameIdentityQuirks {
    /// Create default quirks (no special handling)
    pub fn default_quirks() -> Self {
        Self {
            is_show_existing: false,
            show_existing_frame_idx: None,
            has_film_grain: false,
            tile_count: None,
        }
    }

    /// Create quirks for show_existing_frame
    pub fn show_existing(frame_idx: usize) -> Self {
        Self {
            is_show_existing: true,
            show_existing_frame_idx: Some(frame_idx),
            has_film_grain: false,
            tile_count: None,
        }
    }

    /// Check if this frame needs special identity handling
    pub fn needs_special_handling(&self) -> bool {
        self.is_show_existing || self.has_film_grain
    }

    /// Create quirks for frame with film grain
    pub fn with_film_grain() -> Self {
        Self {
            is_show_existing: false,
            show_existing_frame_idx: None,
            has_film_grain: true,
            tile_count: None,
        }
    }

    /// Create quirks for frame with tile count
    pub fn with_tiles(count: usize) -> Self {
        Self {
            is_show_existing: false,
            show_existing_frame_idx: None,
            has_film_grain: false,
            tile_count: Some(count),
        }
    }

    /// Set film grain flag
    pub fn set_film_grain(&mut self, enabled: bool) {
        self.has_film_grain = enabled;
    }

    /// Set tile count
    pub fn set_tile_count(&mut self, count: usize) {
        self.tile_count = Some(count);
    }
}

// ============================================================================
// Timeline-specific AV1 Quirks Extensions (quirks_AV1.001)
// ============================================================================

/// Timeline metadata with AV1 quirks
///
/// Deliverable: av1_tiles:FrameIdentity:Timeline:AV1:quirks_AV1
///
/// Extends timeline frame metadata with AV1-specific information
/// that affects visualization but not frame identity.
#[derive(Debug, Clone)]
pub struct TimelineFrameWithQuirks {
    /// Display index (primary identity)
    pub display_idx: usize,
    /// Frame size in bytes
    pub size_bytes: u64,
    /// Frame type string
    pub frame_type: String,
    /// AV1-specific quirks
    pub quirks: Av1FrameIdentityQuirks,
}

impl TimelineFrameWithQuirks {
    /// Create new timeline frame with quirks
    pub fn new(
        display_idx: usize,
        size_bytes: u64,
        frame_type: String,
        quirks: Av1FrameIdentityQuirks,
    ) -> Self {
        Self {
            display_idx,
            size_bytes,
            frame_type,
            quirks,
        }
    }

    /// Check if this frame is a "virtual" frame (show_existing_frame)
    ///
    /// show_existing_frame creates a display frame without new coded data.
    /// Per FRAME_IDENTITY_CONTRACT: Still gets unique display_idx.
    pub fn is_virtual_frame(&self) -> bool {
        self.quirks.is_show_existing
    }

    /// Get referenced frame index for virtual frames
    pub fn referenced_frame(&self) -> Option<usize> {
        self.quirks.show_existing_frame_idx
    }

    /// Check if frame has film grain synthesis
    ///
    /// Film grain is applied during display, not decode.
    /// Does not affect frame identity or timeline position.
    pub fn has_film_grain(&self) -> bool {
        self.quirks.has_film_grain
    }

    /// Get tile count (for spatial parsing hints)
    pub fn tile_count(&self) -> Option<usize> {
        self.quirks.tile_count
    }

    /// Get visualization hint for this frame
    ///
    /// Suggests how timeline should display this frame:
    /// - Virtual frames: use lighter/dashed rendering
    /// - Film grain frames: add grain indicator
    /// - Multi-tile frames: add tile count badge
    pub fn viz_hint(&self) -> FrameVizHint {
        if self.is_virtual_frame() {
            FrameVizHint::Virtual
        } else if self.has_film_grain() {
            FrameVizHint::FilmGrain
        } else if let Some(count) = self.tile_count() {
            if count > 1 {
                FrameVizHint::MultiTile(count)
            } else {
                FrameVizHint::Normal
            }
        } else {
            FrameVizHint::Normal
        }
    }
}

/// Visualization hint for timeline rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameVizHint {
    /// Normal frame
    Normal,
    /// Virtual frame (show_existing_frame)
    Virtual,
    /// Frame with film grain synthesis
    FilmGrain,
    /// Frame with multiple tiles
    MultiTile(usize),
}

/// AV1 quirks timeline helper
///
/// Manages AV1 quirks for timeline display without affecting frame identity.
#[derive(Debug, Clone)]
pub struct Av1QuirksTimeline {
    /// Frame metadata with quirks (indexed by display_idx)
    frames: Vec<TimelineFrameWithQuirks>,
    /// Frame index map (identity layer)
    index_map: FrameIndexMap,
}

impl Av1QuirksTimeline {
    /// Create new AV1 quirks timeline
    pub fn new(
        frames_meta: Vec<FrameMetadata>,
        frames_quirks: Vec<(u64, String, Av1FrameIdentityQuirks)>,
    ) -> Self {
        let index_map = FrameIndexMap::new(&frames_meta);

        // Build timeline frames with quirks in display order
        let mut frames = Vec::new();
        for display_idx in 0..index_map.frame_count() {
            if let Some(decode_idx) = index_map.display_to_decode_idx(display_idx) {
                let (size, frame_type, quirks) =
                    frames_quirks.get(decode_idx).cloned().unwrap_or_else(|| {
                        (
                            0,
                            "UNKNOWN".to_string(),
                            Av1FrameIdentityQuirks::default_quirks(),
                        )
                    });

                frames.push(TimelineFrameWithQuirks::new(
                    display_idx,
                    size,
                    frame_type,
                    quirks,
                ));
            }
        }

        Self { frames, index_map }
    }

    /// Get frame with quirks by display_idx
    pub fn get_frame(&self, display_idx: usize) -> Option<&TimelineFrameWithQuirks> {
        self.frames.get(display_idx)
    }

    /// Get all virtual frames (show_existing_frame)
    pub fn virtual_frames(&self) -> Vec<usize> {
        self.frames
            .iter()
            .filter_map(|f| {
                if f.is_virtual_frame() {
                    Some(f.display_idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all frames with film grain
    pub fn film_grain_frames(&self) -> Vec<usize> {
        self.frames
            .iter()
            .filter_map(|f| {
                if f.has_film_grain() {
                    Some(f.display_idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all multi-tile frames
    pub fn multi_tile_frames(&self) -> Vec<(usize, usize)> {
        self.frames
            .iter()
            .filter_map(|f| {
                if let Some(count) = f.tile_count() {
                    if count > 1 {
                        return Some((f.display_idx, count));
                    }
                }
                None
            })
            .collect()
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get index map
    pub fn index_map(&self) -> &FrameIndexMap {
        &self.index_map
    }
}
