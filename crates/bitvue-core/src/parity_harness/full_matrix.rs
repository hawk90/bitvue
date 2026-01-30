//! Full parity matrix - extracted from VQ Analyzer UI State Space

/// Full parity matrix JSON with all competitor features extracted from VQ Analyzer images
/// Categories: IA, Interaction, Contract, Evidence, Performance, FailureUX, Accessibility
pub const FULL_PARITY_MATRIX_JSON: &str = r#"{
  "meta": {
    "version": "v14",
    "generated_at": "2026-01-15T14:30:00.000000Z",
    "scope": "UI/UX parity vs VQ Analyzer - extracted from 296 reference images",
    "competitors": ["vq_analyzer", "elecard_streameye", "intel_vpa"]
  },
  "items": [
    {
      "id": "IA_MAIN_PANEL_CODING_FLOW",
      "category": "IA",
      "severity": "P0",
      "question": "Does the main panel display coding flow grid with CTB/CU/PU hierarchy?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/02-Main-Panel/MainPanel_hevc.jpg", "03-Modes-VVC/03-Coding-Flow/*"]}],
      "our_refs": ["player/pipeline.rs", "partition_grid.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Block hierarchy visible with proper partitioning"}
    },
    {
      "id": "IA_STREAM_VIEW_TIMELINE",
      "category": "IA",
      "severity": "P0",
      "question": "Is there a timeline view showing frame sizes, QP overlay, and thumbnails?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/03-Stream-View/frame_sizes_view.jpg", "02-UI-Components/03-Stream-View/TopFilmstrip_hevc_*.jpg"]}],
      "our_refs": ["timeline.rs", "timeline_lanes.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Timeline shows frame sizes with QP overlay and filmstrip thumbnails"}
    },
    {
      "id": "IA_SYNTAX_TREE_PER_CODEC",
      "category": "IA",
      "severity": "P0",
      "question": "Does syntax info panel show tree view with codec-specific syntax elements?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/AV1/OBUtab_AV1.jpg", "02-UI-Components/04-Syntax-Info/HEVC/NAL_hevc.jpg", "02-UI-Components/04-Syntax-Info/VVC/vvc-syntax-info.jpg"]}],
      "our_refs": ["types.rs:SyntaxNodeId"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json", "pass_criteria": "Tree view navigable with expand/collapse; selection propagates to hex view"}
    },
    {
      "id": "IA_SELECTION_INFO_BLOCK_DETAILS",
      "category": "IA",
      "severity": "P0",
      "question": "Does selection info panel show per-block details (CTB addr, MV, QP, prediction mode)?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/05-Selection-Info/SelectionInfo_hevc.jpg"]}],
      "our_refs": ["selection.rs", "types.rs:BlockInfo"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json", "pass_criteria": "Block selection shows all fields: width, height, CTB addr, MV L0/L1, QP, pred mode"}
    },
    {
      "id": "IA_HEX_VIEW_RAW_BYTES",
      "category": "IA",
      "severity": "P0",
      "question": "Is there a hex view showing raw bytes with offset, hex, and ASCII columns?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/06-Unit-Info-HEX/HEX_view.jpg", "02-UI-Components/06-Unit-Info-HEX/RawBytes_hevc.jpg"]}],
      "our_refs": ["byte_cache.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Hex view with offset column, hex bytes, ASCII representation, selection highlight"}
    },
    {
      "id": "IA_STATUS_PANEL_ERRORS",
      "category": "IA",
      "severity": "P1",
      "question": "Does status panel show errors, warnings, and stream info?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/07-Status-Panel/errors.jpg", "02-UI-Components/07-Status-Panel/status_bar.jpg"]}],
      "our_refs": ["diagnostics.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Status bar shows error count and stream info"}
    },
    {
      "id": "OVERLAY_QP_HEATMAP",
      "category": "Interaction",
      "severity": "P0",
      "question": "Is there a QP heatmap overlay showing per-CU QP values?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["05-Modes-HEVC/08-Heat-Maps/QPmap_hevc.jpg"]}],
      "our_refs": ["qp_heatmap.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "QP values overlaid on blocks with color gradient"}
    },
    {
      "id": "OVERLAY_MV_VECTORS",
      "category": "Interaction",
      "severity": "P0",
      "question": "Is there a motion vector overlay showing MV direction and magnitude?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["05-Modes-HEVC/08-Heat-Maps/MVheat_hevc.jpg"]}],
      "our_refs": ["mv_overlay.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "MV vectors or MV bits per PU shown with proper L0/L1 distinction"}
    },
    {
      "id": "OVERLAY_PSNR_SSIM_HEATMAP",
      "category": "Interaction",
      "severity": "P1",
      "question": "Is there a PSNR/SSIM heatmap overlay showing per-block quality?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["05-Modes-HEVC/08-Heat-Maps/PSNR_hevc.jpg", "05-Modes-HEVC/08-Heat-Maps/SSIM_hevc.jpg"]}],
      "our_refs": ["block_metrics.rs", "diff_heatmap.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Per-block PSNR/SSIM values shown with color coding"}
    },
    {
      "id": "OVERLAY_PU_TYPES",
      "category": "Interaction",
      "severity": "P1",
      "question": "Is there a PU types overlay showing prediction mode colors (intra/inter/skip/merge)?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["05-Modes-HEVC/08-Heat-Maps/PUtype_hevc.jpg"]}],
      "our_refs": ["partition_grid.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Block types color-coded: intra=red, inter=blue, skip, merge, palette with legend"}
    },
    {
      "id": "OVERLAY_EFFICIENCY_MAP",
      "category": "Interaction",
      "severity": "P2",
      "question": "Is there an efficiency map overlay showing coding efficiency per block?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["07-Modes-AV1/11-Overlays/efficiency_map_AV1.jpg", "06-Modes-VP9/07-Overlays/efficiency_map_VP9.jpg"]}],
      "our_refs": ["block_metrics.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Bool-coding efficiency visualized per block"}
    },
    {
      "id": "OVERLAY_REFERENCE_INDICES",
      "category": "Interaction",
      "severity": "P1",
      "question": "Is there a reference indices overlay showing which ref frame each block uses?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["05-Modes-HEVC/08-Heat-Maps/PU_reference_indices_hevc.jpg"]}],
      "our_refs": ["reference_graph.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Reference frame indices shown per block"}
    },
    {
      "id": "METRICS_PSNR_SSIM_GRAPH",
      "category": "IA",
      "severity": "P0",
      "question": "Is there a PSNR/SSIM time series graph with Y/U/V channels?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/03-Stream-View/metrics.JPG"]}],
      "our_refs": ["metrics_distribution.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Time series plot with dual Y-axis (PSNR dB / SSIM 0-1) and Y/U/V legend"}
    },
    {
      "id": "METRICS_HRD_BUFFER_GRAPH",
      "category": "IA",
      "severity": "P1",
      "question": "Is there an HRD/CPB buffer fullness graph over time?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/03-Stream-View/HRDbuffer_avc.jpg"]}],
      "our_refs": ["hrd.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Buffer fullness plot with arrival time markers and bitrate legend"}
    },
    {
      "id": "GLOBAL_DUAL_VIEW_COMPARE",
      "category": "Contract",
      "severity": "P0",
      "question": "Is there dual view for side-by-side stream comparison with sync?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["10-Global-Functions/01-Dual-View/dual_view.jpg", "10-Global-Functions/01-Dual-View/dual_view_3.jpg"]}],
      "our_refs": ["compare.rs", "alignment.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Two panels side-by-side with independent zoom/pan and sync toggle"}
    },
    {
      "id": "GLOBAL_YUV_DEBUG",
      "category": "Interaction",
      "severity": "P1",
      "question": "Is there YUV debug mode for pixel-level inspection?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["10-Global-Functions/02-Debug-YUV/YUV_hevc.jpg", "10-Global-Functions/02-Debug-YUV/YUVdetail_hevc.jpg"]}],
      "our_refs": ["player/mod.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Pixel values shown on hover with Y/U/V breakdown"}
    },
    {
      "id": "GLOBAL_FIND_FIRST_DIFF",
      "category": "Contract",
      "severity": "P1",
      "question": "Is there find first difference feature for comparing YUV with reference?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["10-Global-Functions/03-Find-First-Difference/find_first_diff.jpg"]}],
      "our_refs": ["compare.rs", "diff_heatmap.rs"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json", "pass_criteria": "Frames with differences marked in timeline; first diff frame navigable"}
    },
    {
      "id": "GLOBAL_REFERENCE_POOL",
      "category": "IA",
      "severity": "P0",
      "question": "Is there reference pool / DPB state visualization?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/AV1/refs_AV1.jpg", "02-UI-Components/04-Syntax-Info/HEVC/RefLists_hevc.jpg"]}],
      "our_refs": ["reference_graph.rs", "temporal_state.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Reference pool table with frame, ref slot, refresh status; loop filter deltas"}
    },
    {
      "id": "CONTRACT_TRISYNC_PROPAGATION",
      "category": "Contract",
      "severity": "P0",
      "question": "Does selection propagate across all panels (Tree ⇄ Syntax ⇄ Hex ⇄ Player)?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/*"]}],
      "our_refs": ["selection.rs", "event.rs"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json:selection_propagation_probe", "pass_criteria": "Click in any panel updates all others with anti-loop token"}
    },
    {
      "id": "CONTRACT_DISPLAY_DECODE_ORDER",
      "category": "Contract",
      "severity": "P0",
      "question": "Is display order vs decode order separation enforced everywhere?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["order toggles in StreamView"]}],
      "our_refs": ["command.rs:OrderType", "frame_identity.rs"],
      "verification": {"method": "evidence_bundle_diff", "oracle": "evidence_bundle_diff_contracts.json", "pass_criteria": "All evidence includes explicit order_type; no mixing"}
    },
    {
      "id": "EVIDENCE_BUNDLE_EXPORT",
      "category": "Evidence",
      "severity": "P0",
      "question": "Is there one-click evidence bundle export with screenshots + state + order type?",
      "competitor_refs": [{"target_id": "elecard_streameye", "evidence": ["report/export sections"]}],
      "our_refs": ["export.rs:EvidenceBundleManifest"],
      "verification": {"method": "evidence_bundle_diff", "oracle": "evidence_bundle_diff_contracts.json", "pass_criteria": "Bundle is complete, reproducible, machine-diffable"}
    },
    {
      "id": "INTERACTION_CONTEXT_MENUS",
      "category": "Interaction",
      "severity": "P1",
      "question": "Do context menus appear with proper guards and disabled reason tooltips?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["popup/context-menu states"]}],
      "our_refs": ["export.rs:build_context_menu"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json", "pass_criteria": "Menus open; disabled items show reason tooltip"}
    },
    {
      "id": "CODEC_AV1_OBU_TREE",
      "category": "IA",
      "severity": "P0",
      "question": "Does AV1 mode show OBU tree with frame params, ref pool, segmentation?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/AV1/*", "07-Modes-AV1/*"]}],
      "our_refs": ["types.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "OBU tree with sequence/frame/tile hierarchy; reference pool table"}
    },
    {
      "id": "CODEC_HEVC_NAL_CTU",
      "category": "IA",
      "severity": "P0",
      "question": "Does HEVC mode show NAL tree with VPS/SPS/PPS and CTU/CU hierarchy?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/HEVC/*", "05-Modes-HEVC/*"]}],
      "our_refs": ["types.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "NAL tree; CTU→CU→PU hierarchy; SAO/deblocking views"}
    },
    {
      "id": "CODEC_VVC_DUAL_TREE",
      "category": "IA",
      "severity": "P0",
      "question": "Does VVC mode show dual tree, ALF, and advanced prediction modes?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/VVC/*", "03-Modes-VVC/*"]}],
      "our_refs": ["types.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Dual tree view; ALF/LMCS tabs; affine/geo/MIP prediction details"}
    },
    {
      "id": "CODEC_VP9_SUPERFRAME",
      "category": "IA",
      "severity": "P1",
      "question": "Does VP9 mode show superblock partitions and probabilities?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/04-Syntax-Info/VP9/*", "06-Modes-VP9/*"]}],
      "our_refs": ["types.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "VP9 frame tree; probability tables; SB partitions"}
    },
    {
      "id": "PERF_FRAME_TIME_BUDGET",
      "category": "Performance",
      "severity": "P0",
      "question": "Does UI maintain 60fps (16.6ms frame budget) during navigation?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["smooth navigation observed"]}],
      "our_refs": ["performance.rs", "perf_budget_and_instrumentation.json"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Frame time within 16.6ms budget; degrade steps trigger at thresholds"}
    },
    {
      "id": "PERF_HIT_TEST_LATENCY",
      "category": "Performance",
      "severity": "P1",
      "question": "Is hit-test latency under 1.5ms budget?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["responsive click handling"]}],
      "our_refs": ["performance.rs"],
      "verification": {"method": "semantic_probe", "oracle": "semantic_probe_contracts.json:hit_test_probe", "pass_criteria": "Hit-test returns within 1.5ms; transform consistency verified"}
    },
    {
      "id": "FAILURE_GRACEFUL_DEGRADE",
      "category": "FailureUX",
      "severity": "P1",
      "question": "Does UI degrade gracefully under load with visible indicators?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["loading indicators"]}],
      "our_refs": ["diagnostics.rs", "tooltip.rs"],
      "verification": {"method": "render_snapshot", "oracle": "render_snapshot_contracts.json", "pass_criteria": "Loading states shown; degrade steps visible; no blank panels"}
    },
    {
      "id": "FAILURE_ERROR_RECOVERY",
      "category": "FailureUX",
      "severity": "P0",
      "question": "Does UI recover from parse errors without crashing?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["02-UI-Components/07-Status-Panel/errors.jpg"]}],
      "our_refs": ["error.rs", "diagnostics.rs"],
      "verification": {"method": "manual_review", "oracle": "ui_acceptance_oracle.json", "pass_criteria": "Parse errors shown in status; UI remains responsive"}
    },
    {
      "id": "ACCESSIBILITY_KEYBOARD_NAV",
      "category": "Accessibility",
      "severity": "P2",
      "question": "Is keyboard navigation available for all major actions?",
      "competitor_refs": [{"target_id": "vq_analyzer", "evidence": ["keyboard shortcuts in help"]}],
      "our_refs": ["command.rs"],
      "verification": {"method": "manual_review", "oracle": "ui_acceptance_oracle.json", "pass_criteria": "Tab navigation works; shortcuts for common actions"}
    }
  ]
}"#;

/// Parse and return the full parity matrix
pub fn get_full_parity_matrix() -> Result<(super::ParityMatrix, super::SchemaValidationResult), crate::BitvueError> {
    super::parse_and_validate_parity_matrix(FULL_PARITY_MATRIX_JSON)
}

/// Get the number of parity items in the full matrix
pub fn get_full_parity_item_count() -> usize {
    31 // 31 parity items extracted from VQ Analyzer
}
