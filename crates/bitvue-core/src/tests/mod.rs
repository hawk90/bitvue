//! Consolidated integration tests for bitvue-core
//!
//! These tests were moved from tests/ to src/tests/ to solve
//! linker OOM issues in CI (signal 7 [Bus error]).
//!
//! In tests/, each file is compiled as a separate binary.
//! In src/tests/, all tests are compiled into the lib.rs binary.

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

// Re-export all test modules
pub mod alignment;
pub mod block_metrics;
pub mod byte_cache;
pub mod bytecache_test;
pub mod cache_debug_overlay;
pub mod cache_provenance;
pub mod cache_validation;
pub mod compare;
pub mod compare_cache;
pub mod compare_evidence;
pub mod compare_h264_quirk_b_frame_pyramid_001;
pub mod compare_h264_quirk_dpb_sliding_001;
pub mod compare_h264_quirk_field_frame_001;
pub mod compare_h264_quirk_mixed_scenarios_001;
pub mod compare_h264_quirk_mmco_timing_001;
pub mod compare_h264_quirk_poc_wrap_001;
pub mod compare_vp9_quirks_altref;
pub mod compare_vp9_quirks_integration;
pub mod compare_vp9_quirks_refframe_refresh;
pub mod compare_vp9_quirks_segmentation;
pub mod compare_vp9_quirks_show_existing;
pub mod compare_vp9_quirks_superframe;
pub mod compatibility_32bit_test;
pub mod container_format_test;
pub mod coordinate_transform;
pub mod core;
pub mod corrupt_data_generation_test;
pub mod diagnostics;
pub mod diagnostics_bands;
pub mod diagnostics_core_test;
pub mod diagnostics_edge_cases_test;
pub mod diagnostics_navigation_test;
pub mod diagnostics_parser_integration_test;
pub mod diagnostics_performance_test;
pub mod diagnostics_realworld_scenario_test;
pub mod diagnostics_stream_state_test;
pub mod diagnostics_ui_integration_test;
pub mod diagnostics_workflow_test;
pub mod diff_heatmap;
pub mod disable_reason;
pub mod discoverability;
pub mod edge_cases_bitreader_test;
pub mod endianness_edge_cases_test;
pub mod evidence;
pub mod export;
pub mod frame_identity_core_vp9_evidence_chain_004;
pub mod frame_identity_core_vp9_evidence_chain_005;
pub mod future_plugin;
pub mod hrd;
pub mod index_dev_hud;
pub mod index_dev_hud_window;
pub mod index_extractor;
pub mod index_extractor_evidence;
pub mod index_session;
pub mod index_session_evidence;
pub mod index_session_window;
pub mod indexing;
pub mod insight_feed;
pub mod lockcheck;
pub mod mcp;
pub mod metadata;
pub mod metrics_distribution;
pub mod mv_overlay;
pub mod occlusion_budget;
pub mod parity_baseline_evaluation;
pub mod parity_harness;
pub mod partition_grid;
pub mod performance;
pub mod picture_stats;
pub mod player_evidence;
pub mod qp_heatmap;
pub mod reference_graph;
pub mod reference_graph_evidence;
pub mod selection;
pub mod semantic_evidence;
pub mod spatial_hierarchy;
pub mod temporal_state;
pub mod timeline;
pub mod timeline_cache;
pub mod timeline_evidence;
pub mod timeline_lanes;
pub mod timeline_window;
pub mod tooltip;
pub mod worker;
