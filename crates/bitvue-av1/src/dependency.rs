//! Frame dependency tracking for minimal repro extraction
//!
//! This module provides functionality to analyze frame dependencies
//! and extract minimal reproducible clips from AV1 bitstreams.

use bitvue_core::FrameType;
use crate::obu::{Obu, ObuType};
use std::collections::HashSet;

/// Frame dependency information
#[derive(Debug, Clone)]
pub struct FrameNode {
    /// OBU index in the bitstream
    pub obu_index: usize,
    /// Frame type (Key, Inter, IntraOnly, Switch)
    pub frame_type: FrameType,
    /// Frame size in bytes
    pub size: usize,
}

/// Dependency graph for all frames in a bitstream
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// All frames in the bitstream
    pub frames: Vec<FrameNode>,
    /// Mapping from OBU index to frame index
    obu_to_frame: Vec<Option<usize>>,
}

impl DependencyGraph {
    /// Build dependency graph from OBUs
    pub fn build(obus: &[Obu]) -> Self {
        let mut frames = Vec::new();
        let mut obu_to_frame = vec![None; obus.len()];

        for (obu_idx, obu) in obus.iter().enumerate() {
            match obu.header.obu_type {
                ObuType::Frame | ObuType::FrameHeader => {
                    let frame_type = obu.frame_type.unwrap_or(FrameType::Key);

                    let frame_idx = frames.len();
                    obu_to_frame[obu_idx] = Some(frame_idx);

                    frames.push(FrameNode {
                        obu_index: obu_idx,
                        frame_type,
                        size: obu.total_size as usize,
                    });
                }
                _ => {}
            }
        }

        Self {
            frames,
            obu_to_frame,
        }
    }

    /// Get frame index for an OBU index
    pub fn get_frame_index(&self, obu_index: usize) -> Option<usize> {
        self.obu_to_frame.get(obu_index).copied().flatten()
    }

    /// Find the nearest prior key frame
    pub fn find_nearest_key_frame(&self, frame_index: usize) -> Option<usize> {
        // Search backwards from frame_index
        for i in (0..=frame_index).rev() {
            if let Some(frame) = self.frames.get(i) {
                if frame.frame_type.is_key() {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Get total number of frames
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

/// Extraction request parameters
#[derive(Debug, Clone)]
pub struct ExtractionRequest {
    /// Target frame index to extract
    pub target_frame: usize,
    /// Number of context frames before target
    pub context_before: usize,
    /// Number of context frames after target
    pub context_after: usize,
    /// Include sequence header OBU
    pub include_sequence_header: bool,
}

/// Extraction result with selected OBU indices
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// OBU indices to include in extraction (sorted)
    pub obu_indices: Vec<usize>,
    /// Number of frames included
    pub frame_count: usize,
    /// Estimated output size in bytes
    pub estimated_size: usize,
}

/// Extract required OBUs for minimal repro
///
/// This function determines which OBUs are needed to decode the target frame
/// by:
/// 1. Finding the nearest prior key frame
/// 2. Including all frames from key frame to target
/// 3. Adding context frames before and after
/// 4. Including necessary metadata (sequence header, temporal delimiter)
pub fn extract_required_obus(
    request: &ExtractionRequest,
    graph: &DependencyGraph,
    obus: &[Obu],
) -> ExtractionResult {
    let mut required_obu_indices = HashSet::new();

    // Validate target frame
    if request.target_frame >= graph.frame_count() {
        // Invalid target, return empty result
        return ExtractionResult {
            obu_indices: Vec::new(),
            frame_count: 0,
            estimated_size: 0,
        };
    }

    // Find nearest prior key frame
    let key_frame_idx = graph
        .find_nearest_key_frame(request.target_frame)
        .unwrap_or(0);

    // Include all frames from key frame to target
    for frame_idx in key_frame_idx..=request.target_frame {
        if let Some(frame) = graph.frames.get(frame_idx) {
            required_obu_indices.insert(frame.obu_index);
        }
    }

    // Add context frames before target
    let context_start = request.target_frame.saturating_sub(request.context_before);
    for frame_idx in context_start..request.target_frame {
        if let Some(frame) = graph.frames.get(frame_idx) {
            required_obu_indices.insert(frame.obu_index);
        }
    }

    // Add context frames after target
    let context_end = (request.target_frame + request.context_after).min(graph.frame_count() - 1);
    for frame_idx in request.target_frame + 1..=context_end {
        if let Some(frame) = graph.frames.get(frame_idx) {
            required_obu_indices.insert(frame.obu_index);
        }
    }

    // Include sequence header if requested
    if request.include_sequence_header {
        for (idx, obu) in obus.iter().enumerate() {
            if obu.header.obu_type == ObuType::SequenceHeader {
                required_obu_indices.insert(idx);
                break;
            }
        }
    }

    // Include temporal delimiters and metadata for required frames
    let mut temporal_delimiters = Vec::new();
    for &obu_idx in &required_obu_indices {
        // Include temporal delimiter before this frame
        if obu_idx > 0 {
            if let Some(prev_obu) = obus.get(obu_idx - 1) {
                if prev_obu.header.obu_type == ObuType::TemporalDelimiter {
                    temporal_delimiters.push(obu_idx - 1);
                }
            }
        }
    }

    // Add temporal delimiters to required set
    for idx in temporal_delimiters {
        required_obu_indices.insert(idx);
    }

    // Convert to sorted vector
    let mut obu_indices: Vec<usize> = required_obu_indices.into_iter().collect();
    obu_indices.sort_unstable();

    // Count frames and estimate size
    let mut frame_count = 0;
    let mut estimated_size = 0;

    for &idx in &obu_indices {
        if let Some(obu) = obus.get(idx) {
            estimated_size += obu.total_size as usize;

            if matches!(obu.header.obu_type, ObuType::Frame | ObuType::FrameHeader) {
                frame_count += 1;
            }
        }
    }

    ExtractionResult {
        obu_indices,
        frame_count,
        estimated_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obu::{ObuHeader, ObuType};

    fn create_test_obu(obu_type: ObuType, frame_type: Option<FrameType>, size: usize) -> Obu {
        Obu {
            header: ObuHeader {
                obu_type,
                has_extension: false,
                has_size: true,
                temporal_id: 0,
                spatial_id: 0,
                header_size: 1,
            },
            payload: vec![0; size],
            payload_size: size as u64,
            total_size: (size + 2) as u64, // header + payload
            offset: 0,
            frame_type,
            frame_header: None,
        }
    }

    #[test]
    fn test_dependency_graph_build() {
        let obus = vec![
            create_test_obu(ObuType::SequenceHeader, None, 10),
            create_test_obu(ObuType::Frame, Some(FrameType::Key), 100),
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50),
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50),
        ];

        let graph = DependencyGraph::build(&obus);

        assert_eq!(graph.frame_count(), 3);
        assert_eq!(graph.frames[0].frame_type, FrameType::Key);
        assert_eq!(graph.frames[1].frame_type, FrameType::Inter);
        assert_eq!(graph.frames[2].frame_type, FrameType::Inter);
    }

    #[test]
    fn test_find_nearest_key_frame() {
        let obus = vec![
            create_test_obu(ObuType::Frame, Some(FrameType::Key), 100),
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50),
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50),
            create_test_obu(ObuType::Frame, Some(FrameType::Key), 100),
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50),
        ];

        let graph = DependencyGraph::build(&obus);

        // Frame 2 should find key frame at index 0
        assert_eq!(graph.find_nearest_key_frame(2), Some(0));

        // Frame 4 should find key frame at index 3
        assert_eq!(graph.find_nearest_key_frame(4), Some(3));

        // Frame 0 (key frame) should find itself
        assert_eq!(graph.find_nearest_key_frame(0), Some(0));
    }

    #[test]
    fn test_extract_required_obus() {
        let obus = vec![
            create_test_obu(ObuType::SequenceHeader, None, 20), // 0
            create_test_obu(ObuType::TemporalDelimiter, None, 2), // 1
            create_test_obu(ObuType::Frame, Some(FrameType::Key), 100), // 2
            create_test_obu(ObuType::TemporalDelimiter, None, 2), // 3
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50), // 4
            create_test_obu(ObuType::TemporalDelimiter, None, 2), // 5
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50), // 6
        ];

        let graph = DependencyGraph::build(&obus);

        let request = ExtractionRequest {
            target_frame: 2, // Last inter frame
            context_before: 0,
            context_after: 0,
            include_sequence_header: true,
        };

        let result = extract_required_obus(&request, &graph, &obus);

        // Should include: sequence header (0), key frame (2), and both inter frames (4, 6)
        // Plus temporal delimiters: 1, 3, 5
        assert!(result.obu_indices.contains(&0)); // Sequence header
        assert!(result.obu_indices.contains(&2)); // Key frame
        assert!(result.obu_indices.contains(&4)); // Inter frame 1
        assert!(result.obu_indices.contains(&6)); // Inter frame 2 (target)

        assert_eq!(result.frame_count, 3);
    }

    #[test]
    fn test_extract_with_context() {
        let obus = vec![
            create_test_obu(ObuType::SequenceHeader, None, 20),
            create_test_obu(ObuType::Frame, Some(FrameType::Key), 100), // 1
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50), // 2
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50), // 3
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50), // 4
            create_test_obu(ObuType::Frame, Some(FrameType::Inter), 50), // 5
        ];

        let graph = DependencyGraph::build(&obus);

        let request = ExtractionRequest {
            target_frame: 2, // Frame at OBU index 3
            context_before: 1,
            context_after: 1,
            include_sequence_header: true,
        };

        let result = extract_required_obus(&request, &graph, &obus);

        // Should include frames 1, 2, 3 (context before + target + context after)
        // Plus key frame (0) since it's needed for dependencies
        assert_eq!(result.frame_count, 4);
    }
}
