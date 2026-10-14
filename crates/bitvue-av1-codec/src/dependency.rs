//! Frame dependency tracking for minimal repro extraction
//!
//! This module provides functionality to analyze frame dependencies
//! and extract minimal reproducible clips from AV1 bitstreams.

use crate::obu::{Obu, ObuType};
use bitvue_engine::FrameType;
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
}

impl DependencyGraph {
    /// Build dependency graph from OBUs
    pub fn build(obus: &[Obu]) -> Self {
        let mut frames = Vec::new();

        for (obu_idx, obu) in obus.iter().enumerate() {
            match obu.header.obu_type {
                ObuType::Frame | ObuType::FrameHeader => {
                    let frame_type = obu.frame_type.unwrap_or(FrameType::Key);

                    frames.push(FrameNode {
                        obu_index: obu_idx,
                        frame_type,
                        size: obu.total_size as usize,
                    });
                }
                _ => {}
            }
        }

        Self { frames }
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
    // Pre-allocate with known capacity to avoid reallocation
    let mut obu_indices = Vec::with_capacity(required_obu_indices.len());
    obu_indices.extend(required_obu_indices);
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
    use std::sync::Arc;

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
            payload: Arc::from(vec![0; size]),
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

    use ObuType::{Frame, FrameHeader, Padding, SequenceHeader, TemporalDelimiter, TileGroup};

    fn obu(obu_type: ObuType, frame_type: Option<FrameType>, size: usize) -> Obu {
        create_test_obu(obu_type, frame_type, size)
    }

    fn key(size: usize) -> Obu {
        obu(Frame, Some(FrameType::Key), size)
    }

    fn inter(size: usize) -> Obu {
        obu(Frame, Some(FrameType::Inter), size)
    }

    fn request(target: usize, before: usize, after: usize, seq: bool) -> ExtractionRequest {
        ExtractionRequest {
            target_frame: target,
            context_before: before,
            context_after: after,
            include_sequence_header: seq,
        }
    }

    fn extract(obus: &[Obu], request: &ExtractionRequest) -> ExtractionResult {
        extract_required_obus(request, &DependencyGraph::build(obus), obus)
    }

    #[test]
    fn build_records_only_frame_and_frame_header_obus_with_their_stream_position_and_size() {
        let obus = vec![
            obu(SequenceHeader, None, 10),
            obu(TemporalDelimiter, None, 0),
            key(100),
            obu(Padding, None, 3),
            obu(FrameHeader, Some(FrameType::Inter), 7),
            obu(TileGroup, None, 40),
        ];
        let graph = DependencyGraph::build(&obus);
        assert_eq!(graph.frame_count(), 2);
        assert_eq!(
            (
                graph.frames[0].obu_index,
                graph.frames[0].frame_type,
                graph.frames[0].size
            ),
            (2, FrameType::Key, 102)
        );
        assert_eq!(
            (
                graph.frames[1].obu_index,
                graph.frames[1].frame_type,
                graph.frames[1].size
            ),
            (4, FrameType::Inter, 9)
        );
    }

    #[test]
    fn build_treats_a_frame_without_a_parsed_type_as_a_key_frame() {
        let graph = DependencyGraph::build(&[obu(Frame, None, 5)]);
        assert_eq!(graph.frames[0].frame_type, FrameType::Key);
    }

    #[test]
    fn build_of_nothing_or_of_non_frame_obus_is_an_empty_graph() {
        assert_eq!(DependencyGraph::build(&[]).frame_count(), 0);
        let graph = DependencyGraph::build(&[obu(SequenceHeader, None, 1), obu(Padding, None, 1)]);
        assert_eq!(graph.frame_count(), 0);
    }

    #[test]
    fn nearest_key_frame_is_none_without_a_key_frame_or_without_frames() {
        assert_eq!(DependencyGraph::build(&[]).find_nearest_key_frame(0), None);
        let graph = DependencyGraph::build(&[inter(1), inter(1)]);
        assert_eq!(graph.find_nearest_key_frame(1), None);
    }

    #[test]
    fn nearest_key_frame_only_looks_backwards_and_only_counts_key_frames() {
        let graph = DependencyGraph::build(&[
            obu(Frame, Some(FrameType::IntraOnly), 1),
            key(1),
            obu(Frame, Some(FrameType::IntraOnly), 1),
            inter(1),
            key(1),
        ]);
        // IntraOnly is not a key frame; a later key frame is not "prior".
        assert_eq!(graph.find_nearest_key_frame(3), Some(1));
        assert_eq!(graph.find_nearest_key_frame(2), Some(1));
        assert_eq!(graph.find_nearest_key_frame(0), None);
        assert_eq!(graph.find_nearest_key_frame(4), Some(4));
    }

    #[test]
    fn nearest_key_frame_past_the_last_frame_searches_from_the_end() {
        let graph = DependencyGraph::build(&[key(1), inter(1)]);
        assert_eq!(graph.find_nearest_key_frame(5), Some(0));
    }

    #[test]
    fn a_target_past_the_last_frame_extracts_nothing() {
        let obus = vec![key(10), inter(10)];
        for target in [2, 3, 1000] {
            let result = extract(&obus, &request(target, 1, 1, true));
            assert!(result.obu_indices.is_empty(), "target {target}");
            assert_eq!((result.frame_count, result.estimated_size), (0, 0));
        }
        // The last frame itself is valid.
        assert_eq!(extract(&obus, &request(1, 0, 0, false)).frame_count, 2);
    }

    #[test]
    fn extracting_from_a_stream_without_frames_is_empty_and_does_not_underflow() {
        let obus = vec![obu(SequenceHeader, None, 4)];
        let result = extract(&obus, &request(0, 0, 0, true));
        assert!(result.obu_indices.is_empty());
        assert_eq!(extract(&[], &request(0, 5, 5, true)).obu_indices.len(), 0);
    }

    #[test]
    fn without_a_key_frame_the_extraction_starts_at_the_first_frame() {
        let obus = vec![inter(1), inter(1), inter(1)];
        let result = extract(&obus, &request(2, 0, 0, false));
        assert_eq!(result.obu_indices, vec![0, 1, 2]);
    }

    #[test]
    fn extraction_runs_from_the_nearest_key_frame_to_the_target() {
        let obus = vec![key(1), inter(1), key(1), inter(1), inter(1), inter(1)];
        let result = extract(&obus, &request(4, 0, 0, false));
        assert_eq!(result.obu_indices, vec![2, 3, 4]);
        // The target being a key frame itself needs nothing before it.
        assert_eq!(
            extract(&obus, &request(2, 0, 0, false)).obu_indices,
            vec![2]
        );
    }

    #[test]
    fn context_before_reaches_behind_the_key_frame_and_is_limited_by_the_stream_start() {
        let obus = vec![inter(1), inter(1), inter(1), key(1), inter(1)];
        // Key frame at 3, target 4: the dependency chain is {3, 4}; 2 frames of context add 2 and 1... but
        // context_before counts frames before the target, so 4-2 = 2..4 adds frames 2 and 3.
        assert_eq!(
            extract(&obus, &request(4, 2, 0, false)).obu_indices,
            vec![2, 3, 4]
        );
        // More context than frames exist: clamps at 0 without underflow.
        assert_eq!(
            extract(&obus, &request(4, usize::MAX, 0, false)).obu_indices,
            vec![0, 1, 2, 3, 4]
        );
        assert_eq!(
            extract(&obus, &request(0, 9, 0, false)).obu_indices,
            vec![0]
        );
    }

    #[test]
    fn context_after_is_limited_by_the_stream_end() {
        let obus = vec![key(1), inter(1), inter(1), inter(1)];
        assert_eq!(
            extract(&obus, &request(1, 0, 1, false)).obu_indices,
            vec![0, 1, 2]
        );
        assert_eq!(
            extract(&obus, &request(1, 0, 2, false)).obu_indices,
            vec![0, 1, 2, 3]
        );
        assert_eq!(
            extract(&obus, &request(1, 0, 50, false)).obu_indices,
            vec![0, 1, 2, 3]
        );
        // Target is the last frame: nothing after it, and no off-by-one on the range end.
        assert_eq!(
            extract(&obus, &request(3, 0, 5, false)).obu_indices,
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    fn the_sequence_header_is_included_only_when_asked_and_only_the_first_one() {
        let obus = vec![
            obu(SequenceHeader, None, 10),
            key(100),
            obu(SequenceHeader, None, 20),
            inter(50),
        ];
        let with = extract(&obus, &request(0, 0, 0, true));
        assert_eq!(with.obu_indices, vec![0, 1]);
        assert_eq!(with.estimated_size, 12 + 102);
        let without = extract(&obus, &request(0, 0, 0, false));
        assert_eq!(without.obu_indices, vec![1]);
        // Asked for but absent from the stream: nothing to add, no error.
        let no_header = vec![key(1)];
        assert_eq!(
            extract(&no_header, &request(0, 0, 0, true)).obu_indices,
            vec![0]
        );
    }

    #[test]
    fn a_temporal_delimiter_is_kept_only_when_it_directly_precedes_a_required_obu() {
        let obus = vec![
            obu(TemporalDelimiter, None, 0), // 0: before the key frame
            key(10),                         // 1
            obu(TemporalDelimiter, None, 0), // 2: before the skipped...
            obu(Padding, None, 1),           // 3: ...padding, so not directly before the frame
            inter(10),                       // 4
            obu(TemporalDelimiter, None, 0), // 5: before the target
            inter(10),                       // 6
            obu(TemporalDelimiter, None, 0), // 7: after the target, not wanted
        ];
        let result = extract(&obus, &request(2, 0, 0, false));
        assert_eq!(result.obu_indices, vec![0, 1, 4, 5, 6]);
        assert_eq!(result.frame_count, 3);
        assert_eq!(result.estimated_size, 2 + 12 + 12 + 2 + 12);
    }

    #[test]
    fn a_required_obu_at_index_zero_has_no_delimiter_to_look_for() {
        let obus = vec![key(1), obu(TemporalDelimiter, None, 0), inter(1)];
        assert_eq!(
            extract(&obus, &request(0, 0, 0, false)).obu_indices,
            vec![0]
        );
    }

    #[test]
    fn the_indices_are_sorted_unique_and_the_totals_match_the_selected_obus() {
        let obus = vec![
            obu(SequenceHeader, None, 8),
            obu(TemporalDelimiter, None, 0),
            key(30),
            obu(TemporalDelimiter, None, 0),
            obu(FrameHeader, Some(FrameType::Inter), 5),
            obu(TileGroup, None, 20),
            obu(TemporalDelimiter, None, 0),
            inter(15),
        ];
        // Overlapping ranges (key->target, context_before, context_after) must not duplicate entries.
        let result = extract(&obus, &request(1, 5, 5, true));
        assert_eq!(result.obu_indices, vec![0, 1, 2, 3, 4, 6, 7]);
        // The tile group (5) is not a frame OBU, so it is not part of the selection.
        let total: u64 = result.obu_indices.iter().map(|&i| obus[i].total_size).sum();
        assert_eq!(result.estimated_size as u64, total);
        assert_eq!(result.frame_count, 3); // Frame, FrameHeader, Frame
    }

    #[test]
    fn a_graph_that_does_not_match_the_obus_is_tolerated() {
        // Graph built from a longer stream than the one passed: indices beyond it are skipped.
        let long = vec![key(1), inter(1), inter(1), inter(1)];
        let graph = DependencyGraph::build(&long);
        let short = &long[..2];
        let result = extract_required_obus(&request(3, 0, 0, true), &graph, short);
        assert_eq!(result.obu_indices, vec![0, 1, 2, 3]);
        assert_eq!(result.frame_count, 2);
        assert_eq!(result.estimated_size, 6);
    }
}
