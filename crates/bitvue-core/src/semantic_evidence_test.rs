// Semantic Evidence module tests
use super::*;

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_semantic_record(id: &str, display_idx: u64) -> SemanticEvidenceRecord {
    SemanticEvidenceRecord::new(
        id.to_string(),
        display_idx,
        SemanticEvidence::Av1Cdf(Av1CdfUpdate {
            context_id: 0,
            symbol: 1,
            old_probability: 0.5,
            new_probability: 0.6,
            divergence_from_default: 0.1,
        }),
        "syntax_1".to_string(),
    )
}

fn create_test_index() -> SemanticIndex {
    SemanticIndex::new()
}

// ============================================================================
// Codec Tests
// ============================================================================
#[cfg(test)]
mod codec_tests {
    use super::*;

    #[test]
    fn test_av1_evidence_returns_av1_codec() {
        let evidence = SemanticEvidence::Av1Cdf(Av1CdfUpdate {
            context_id: 0,
            symbol: 1,
            old_probability: 0.5,
            new_probability: 0.6,
            divergence_from_default: 0.1,
        });
        assert_eq!(evidence.codec(), Codec::Av1);
    }

    #[test]
    fn test_h264_evidence_returns_h264_codec() {
        let evidence = SemanticEvidence::H264Poc(H264PocInfo {
            poc_type: 0,
            poc_lsb: 100,
            poc_msb: 0,
            top_field_poc: 100,
            bottom_field_poc: 101,
            wrap_count: 0,
            is_wrap_boundary: false,
        });
        assert_eq!(evidence.codec(), Codec::H264);
    }

    #[test]
    fn test_hevc_evidence_returns_hevc_codec() {
        let evidence = SemanticEvidence::HevcIrap(HevcIrapInfo {
            nal_type: HevcNalType::IdrNLp,
            is_irap: true,
            is_idr: true,
            is_cra: false,
            is_bla: false,
            no_rasl_output_flag: true,
            associated_rasl_count: 0,
        });
        assert_eq!(evidence.codec(), Codec::Hevc);
    }

    #[test]
    fn test_vp9_evidence_returns_vp9_codec() {
        let evidence = SemanticEvidence::Vp9Superframe(Vp9SuperframeInfo {
            is_superframe: true,
            frame_count: 3,
            frame_sizes: vec![1000, 2000, 1500],
            total_size: 4500,
        });
        assert_eq!(evidence.codec(), Codec::Vp9);
    }

    #[test]
    fn test_vvc_evidence_returns_vvc_codec() {
        let evidence = SemanticEvidence::VvcGdr(VvcGdrInfo {
            is_gdr: true,
            recovery_poc_cnt: 10,
            no_output_before_recovery: false,
            gradual_refresh_line: 500,
        });
        assert_eq!(evidence.codec(), Codec::Vvc);
    }

    #[test]
    fn test_custom_evidence_returns_specified_codec() {
        let evidence = SemanticEvidence::Custom {
            codec: Codec::Av1,
            name: "custom_test".to_string(),
            data: HashMap::new(),
        };
        assert_eq!(evidence.codec(), Codec::Av1);
    }
}

// ============================================================================
// Description Tests
// ============================================================================
#[cfg(test)]
mod description_tests {
    use super::*;

    #[test]
    fn test_av1_cdf_description() {
        let evidence = SemanticEvidence::Av1Cdf(Av1CdfUpdate {
            context_id: 42,
            symbol: 1,
            old_probability: 0.5,
            new_probability: 0.7,
            divergence_from_default: 0.2,
        });
        let desc = evidence.description();
        assert!(desc.contains("42"));
        assert!(desc.contains("0.20"));
    }

    #[test]
    fn test_av1_tile_group_description() {
        let evidence = SemanticEvidence::Av1TileGroup(Av1TileGroupInfo {
            tile_start_idx: 0,
            tile_end_idx: 3,
            tile_count: 3,
            is_uniform_spacing: true,
        });
        let desc = evidence.description();
        assert!(desc.contains("0-3"));
    }

    #[test]
    fn test_h264_poc_description() {
        let evidence = SemanticEvidence::H264Poc(H264PocInfo {
            poc_type: 0,
            poc_lsb: 100,
            poc_msb: 0,
            top_field_poc: 100,
            bottom_field_poc: 101,
            wrap_count: 0,
            is_wrap_boundary: false,
        });
        let desc = evidence.description();
        assert!(desc.contains("100"));
    }

    #[test]
    fn test_vp9_superframe_description() {
        let evidence = SemanticEvidence::Vp9Superframe(Vp9SuperframeInfo {
            is_superframe: true,
            frame_count: 2,
            frame_sizes: vec![1000, 2000],
            total_size: 3000,
        });
        let desc = evidence.description();
        assert!(desc.contains("2 frames"));
    }

    #[test]
    fn test_vvc_gdr_description() {
        let evidence = SemanticEvidence::VvcGdr(VvcGdrInfo {
            is_gdr: true,
            recovery_poc_cnt: 15,
            no_output_before_recovery: false,
            gradual_refresh_line: 600,
        });
        let desc = evidence.description();
        assert!(desc.contains("15"));
    }

    #[test]
    fn test_custom_description() {
        let evidence = SemanticEvidence::Custom {
            codec: Codec::Av1,
            name: "test_custom".to_string(),
            data: HashMap::new(),
        };
        let desc = evidence.description();
        assert!(desc.contains("test_custom"));
    }
}

// ============================================================================
// SemanticEvidenceRecord Tests
// ============================================================================
#[cfg(test)]
mod record_tests {
    use super::*;

    #[test]
    fn test_new_creates_record() {
        let record = create_test_semantic_record("test_id", 100);
        assert_eq!(record.id, "test_id");
        assert_eq!(record.display_idx, 100);
        assert!(record.metadata.is_empty());
    }

    #[test]
    fn test_add_metadata() {
        let mut record = create_test_semantic_record("test_id", 100);
        record.add_metadata("key1", "value1");
        assert_eq!(record.metadata.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_add_metadata_multiple() {
        let mut record = create_test_semantic_record("test_id", 100);
        record.add_metadata("key1", "value1");
        record.add_metadata("key2", "value2");
        assert_eq!(record.metadata.len(), 2);
    }

    #[test]
    fn test_syntax_link_stored() {
        let record = create_test_semantic_record("test_id", 100);
        assert_eq!(record.syntax_link, "syntax_1");
    }
}

// ============================================================================
// SemanticIndex Tests
// ============================================================================
#[cfg(test)]
mod index_tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_index() {
        let index = create_test_index();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_add_increments_length() {
        let mut index = create_test_index();
        index.add(create_test_semantic_record("id1", 100));
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_find_by_id() {
        let mut index = create_test_index();
        index.add(create_test_semantic_record("test_id", 100));
        let result = index.find_by_id("test_id");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_by_id_not_found() {
        let index = create_test_index();
        let result = index.find_by_id("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_by_display_idx() {
        let mut index = create_test_index();
        index.add(create_test_semantic_record("id1", 100));
        index.add(create_test_semantic_record("id2", 100));
        index.add(create_test_semantic_record("id3", 200));
        let results = index.find_by_display_idx(100);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_find_by_codec() {
        let mut index = create_test_index();
        index.add(create_test_semantic_record("id1", 100));
        let results = index.find_by_codec(Codec::Av1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_by_syntax_link() {
        let mut index = create_test_index();
        index.add(create_test_semantic_record("id1", 100));
        let results = index.find_by_syntax_link("syntax_1");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_by_syntax_link_no_match() {
        let mut index = create_test_index();
        index.add(create_test_semantic_record("id1", 100));
        let results = index.find_by_syntax_link("nonexistent");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_all_returns_records() {
        let mut index = create_test_index();
        index.add(create_test_semantic_record("id1", 100));
        index.add(create_test_semantic_record("id2", 200));
        let all = index.all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_is_empty_true() {
        let index = create_test_index();
        assert!(index.is_empty());
    }

    #[test]
    fn test_is_empty_false() {
        let mut index = create_test_index();
        index.add(create_test_semantic_record("id1", 100));
        assert!(!index.is_empty());
    }
}

// ============================================================================
// Codec-Specific Evidence Tests
// ============================================================================
#[cfg(test)]
mod av1_tests {
    use super::*;

    #[test]
    fn test_av1_cdf_update_fields() {
        let cdf = Av1CdfUpdate {
            context_id: 10,
            symbol: 5,
            old_probability: 0.3,
            new_probability: 0.7,
            divergence_from_default: 0.4,
        };
        assert_eq!(cdf.context_id, 10);
        assert_eq!(cdf.divergence_from_default, 0.4);
    }

    #[test]
    fn test_av1_tile_group_info() {
        let info = Av1TileGroupInfo {
            tile_start_idx: 0,
            tile_end_idx: 4,
            tile_count: 4,
            is_uniform_spacing: true,
        };
        assert_eq!(info.tile_count, 4);
        assert!(info.is_uniform_spacing);
    }

    #[test]
    fn test_av1_superres_disabled() {
        let info = Av1SuperresInfo {
            enabled: false,
            coded_width: 1920,
            upscaled_width: 1920,
            scale_denominator: 8,
        };
        assert!(!info.enabled);
    }

    #[test]
    fn test_av1_film_grain_enabled() {
        let info = Av1FilmGrainInfo {
            enabled: true,
            grain_seed: 12345,
            num_y_points: 8,
            num_cb_points: 8,
            num_cr_points: 4,
            chroma_scaling_from_luma: false,
        };
        assert!(info.enabled);
        assert_eq!(info.grain_seed, 12345);
    }
}

#[cfg(test)]
mod h264_tests {
    use super::*;

    #[test]
    fn test_h264_poc_wrap_boundary() {
        let poc = H264PocInfo {
            poc_type: 0,
            poc_lsb: 0,
            poc_msb: -300,
            top_field_poc: 0,
            bottom_field_poc: 1,
            wrap_count: 1,
            is_wrap_boundary: true,
        };
        assert!(poc.is_wrap_boundary);
        assert_eq!(poc.wrap_count, 1);
    }

    #[test]
    fn test_h264_dpb_sliding() {
        let sliding = H264DpbSliding {
            evicted_frame_num: 50,
            evicted_poc: 150,
            reason: "Sliding window".to_string(),
            dpb_fullness_before: 5,
            dpb_fullness_after: 4,
        };
        assert!(sliding.reason.contains("Sliding"));
    }

    #[test]
    fn test_h264_b_pyramid_level() {
        let pyramid = H264BPyramidInfo {
            pyramid_depth: 2,
            current_level: 1,
            is_reference: true,
            parent_poc: Some(100),
        };
        assert_eq!(pyramid.current_level, 1);
        assert!(pyramid.is_reference);
    }
}

#[cfg(test)]
mod hevc_tests {
    use super::*;

    #[test]
    fn test_hevc_irap_idr() {
        let irap = HevcIrapInfo {
            nal_type: HevcNalType::IdrNLp,
            is_irap: true,
            is_idr: true,
            is_cra: false,
            is_bla: false,
            no_rasl_output_flag: true,
            associated_rasl_count: 0,
        };
        assert!(irap.is_idr);
        assert!(!irap.is_cra);
    }

    #[test]
    fn test_hevc_temporal_info() {
        let temp = HevcTemporalInfo {
            temporal_id: 2,
            max_temporal_layers: 4,
            discardable: false,
            sub_layer_non_ref: false,
        };
        assert_eq!(temp.temporal_id, 2);
        assert_eq!(temp.max_temporal_layers, 4);
    }

    #[test]
    fn test_hevc_ctu_boundary() {
        let ctu = HevcCtuBoundary {
            ctu_addr: 10,
            slice_idx: 2,
            tile_idx: 1,
            is_slice_boundary: true,
            is_tile_boundary: false,
            dependent_slice: false,
        };
        assert!(ctu.is_slice_boundary);
        assert!(!ctu.is_tile_boundary);
    }
}

#[cfg(test)]
mod vp9_tests {
    use super::*;

    #[test]
    fn test_vp9_superframe() {
        let info = Vp9SuperframeInfo {
            is_superframe: true,
            frame_count: 3,
            frame_sizes: vec![1000, 2000, 1500],
            total_size: 4500,
        };
        assert!(info.is_superframe);
        assert_eq!(info.frame_count, 3);
    }

    #[test]
    fn test_vp9_ref_buffer() {
        let info = Vp9RefBufferInfo {
            last_ref_idx: 0,
            golden_ref_idx: 1,
            altref_ref_idx: 2,
            refresh_frame_flags: 0x0F,
            ref_frame_sign_bias: [false, true, false, true],
        };
        assert_eq!(info.altref_ref_idx, 2);
    }

    #[test]
    fn test_vp9_segmentation_enabled() {
        let info = Vp9SegmentationInfo {
            enabled: true,
            update_map: true,
            update_data: true,
            abs_or_delta_update: false,
            segment_feature_active: [[false; 4]; 8],
            segment_feature_data: [[0; 4]; 8],
        };
        assert!(info.enabled);
    }
}

#[cfg(test)]
mod vvc_tests {
    use super::*;

    #[test]
    fn test_vvc_gdr_info() {
        let info = VvcGdrInfo {
            is_gdr: true,
            recovery_poc_cnt: 20,
            no_output_before_recovery: true,
            gradual_refresh_line: 540,
        };
        assert!(info.is_gdr);
        assert!(info.no_output_before_recovery);
    }

    #[test]
    fn test_vvc_rpl_info() {
        let info = VvcRplInfo {
            num_ref_entries: [5, 3],
            ltrp_in_header_flag: true,
            inter_layer_ref_present: false,
            ref_pic_list: vec![],
        };
        assert_eq!(info.num_ref_entries[0], 5);
    }

    #[test]
    fn test_vvc_alf_enabled() {
        let info = VvcAlfInfo {
            alf_enabled: true,
            num_alf_aps_ids_luma: 5,
            alf_chroma_idc: 1,
            alf_cc_cb_enabled: false,
            alf_cc_cr_enabled: false,
        };
        assert!(info.alf_enabled);
    }

    #[test]
    fn test_vvc_subpic_info() {
        let info = VvcSubpicInfo {
            subpic_idx: 0,
            subpic_id: 1,
            ctu_top_left_x: 0,
            ctu_top_left_y: 0,
            width_in_ctus: 10,
            height_in_ctus: 8,
            treated_as_pic: false,
            loop_filter_across_boundary: true,
        };
        assert_eq!(info.width_in_ctus, 10);
    }
}
