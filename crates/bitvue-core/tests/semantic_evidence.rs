#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for semantic evidence

use bitvue_core::semantic_evidence::{
    Av1CdfUpdate, Av1SuperresInfo, Codec, H264PocInfo, SemanticEvidence, SemanticEvidenceRecord,
    SemanticIndex,
};

#[test]
fn test_av1_cdf_evidence() {
    let cdf = Av1CdfUpdate {
        context_id: 42,
        symbol: 3,
        old_probability: 0.75,
        new_probability: 0.72,
        divergence_from_default: 0.23,
    };

    let evidence = SemanticEvidence::Av1Cdf(cdf);
    assert_eq!(evidence.codec(), Codec::Av1);
    assert!(evidence.description().contains("CDF update"));
}

#[test]
fn test_h264_poc_evidence() {
    let poc = H264PocInfo {
        poc_type: 0,
        poc_lsb: 20,
        poc_msb: 64,
        top_field_poc: 84,
        bottom_field_poc: 85,
        wrap_count: 2,
        is_wrap_boundary: false,
    };

    let evidence = SemanticEvidence::H264Poc(poc);
    assert_eq!(evidence.codec(), Codec::H264);
    assert!(evidence.description().contains("POC"));
}

#[test]
fn test_semantic_index() {
    let mut index = SemanticIndex::new();

    let record = SemanticEvidenceRecord::new(
        "sem_001".to_string(),
        42,
        SemanticEvidence::Av1Superres(Av1SuperresInfo {
            enabled: true,
            coded_width: 1280,
            upscaled_width: 1920,
            scale_denominator: 12,
        }),
        "syn_001".to_string(),
    );

    index.add(record);
    assert_eq!(index.len(), 1);

    let found = index.find_by_display_idx(42);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "sem_001");

    let by_codec = index.find_by_codec(Codec::Av1);
    assert_eq!(by_codec.len(), 1);
}
