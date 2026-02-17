#![allow(dead_code)]
//! Tests for tooltip module

use bitvue_core::tooltip::{
    CopyAction, HexBitTooltip, MetricsTooltip, PlayerTooltip, SyntaxTooltip, TimelineTooltip,
    TooltipConfig, TooltipContent, TooltipManager, TooltipState,
};

#[test]
fn test_tooltip_config_default() {
    let config = TooltipConfig::default();
    assert_eq!(config.hover_delay_ms, 150);
    assert!(config.enable_copy_actions);
}

#[test]
fn test_timeline_tooltip_format() {
    let tooltip = TimelineTooltip {
        frame_idx: 42,
        frame_type: "Key".to_string(),
        pts: Some(42000),
        dts: Some(42000),
        time_seconds: Some(42.0),
        size_bytes: Some(1024),
        size_bits: Some(8192),
        markers: vec!["Keyframe".to_string(), "Bookmark".to_string()],
        decoded: true,
        decode_error: None,
        syntax_path: None,
        bit_offset: None,
        byte_offset: None,
        copy_actions: vec![],
    };

    let text = tooltip.format();
    assert!(text.contains("Frame: 42"));
    assert!(text.contains("PTS: 42000 ms"));
    assert!(text.contains("Size: 1024 bytes"));
    assert!(text.contains("Decoded: yes"));
}

#[test]
fn test_metrics_tooltip_format() {
    let tooltip = MetricsTooltip {
        frame_idx: 10,
        time_seconds: Some(0.4),
        series_name: "PSNR_Y".to_string(),
        value: 38.5,
        unit: "dB".to_string(),
        delta: Some(-0.5),
        copy_actions: vec![],
    };

    let text = tooltip.format();
    assert!(text.contains("Frame: 10"));
    assert!(text.contains("PSNR_Y: 38.50 dB"));
    assert!(text.contains("Δ: -0.50 dB"));
}

#[test]
fn test_tooltip_manager() {
    let mut manager = TooltipManager::default();
    assert!(manager.current().is_none());

    let content = TooltipContent::Custom("Test tooltip".to_string());
    manager.show(content, (100, 100));

    assert!(manager.current().is_some());
    assert_eq!(manager.current().unwrap().position, (100, 100));

    manager.hide();
    assert!(manager.current().is_none());
}

#[test]
fn test_copy_action() {
    let action = CopyAction::new("Copy Frame", "A:Frame=42");
    assert_eq!(action.label, "Copy Frame");
    assert_eq!(action.content, "A:Frame=42");
}

#[test]
fn test_hex_bit_tooltip_format() {
    let tooltip = HexBitTooltip {
        offset_hex: "0x1234".to_string(),
        byte_hex: "0xAB".to_string(),
        byte_decimal: 171,
        ascii_char: Some('A'),
        bit_in_byte: Some(5),
        global_bit_offset: Some(9237),
        mapped_field: Some("frame_size".to_string()),
        copy_actions: vec![],
    };

    let text = tooltip.format();
    assert!(text.contains("Offset: 0x1234"));
    assert!(text.contains("Byte: 0xAB (171)"));
    assert!(text.contains("ASCII: 'A'"));
    assert!(text.contains("Field: frame_size"));
}

#[test]
fn test_syntax_tooltip_format() {
    let tooltip = SyntaxTooltip {
        field_name: "frame_width".to_string(),
        field_type: "u16".to_string(),
        decoded_value: "1920".to_string(),
        bit_range: (100, 116),
        raw_bits: "0111100000000000".to_string(),
        condition: Some("if(frame_size_override)".to_string()),
        copy_actions: vec![],
    };

    let text = tooltip.format();
    assert!(text.contains("frame_width: u16"));
    assert!(text.contains("Value: 1920"));
    assert!(text.contains("Bits: 100..116 (16 bits)"));
    assert!(text.contains("Condition:"));
}

#[test]
fn test_tooltip_state_format() {
    let content = TooltipContent::Custom("Test content".to_string());
    let state = TooltipState {
        content,
        position: (50, 50),
        visible: true,
    };

    assert_eq!(state.format_text(), "Test content");
    assert_eq!(state.copy_actions().len(), 0);
}

#[test]
fn test_na_handling() {
    let tooltip = TimelineTooltip {
        frame_idx: 10,
        frame_type: "Inter".to_string(),
        pts: None,
        dts: None,
        time_seconds: None,
        size_bytes: None,
        size_bits: None,
        markers: vec![],
        decoded: false,
        decode_error: None,
        syntax_path: None,
        bit_offset: None,
        byte_offset: None,
        copy_actions: vec![],
    };

    let text = tooltip.format();
    assert!(text.contains("PTS: N/A"));
    assert!(text.contains("Size: N/A"));
}

#[test]
fn test_player_tooltip_pixel_hover() {
    // Test pixel hover (no block data)
    let tooltip = PlayerTooltip {
        frame_idx: 42,
        pixel_xy: (100, 200),
        luma: Some(128),
        chroma: Some((64, 192)),
        block_id: None,
        qp: None,
        mv: None,
        partition_info: None,
        active_overlays: vec![],
        syntax_path: None,
        bit_offset: None,
        byte_offset: None,
        copy_actions: vec![],
    };

    let text = tooltip.format();
    assert!(text.contains("Frame: 42"));
    assert!(text.contains("Pixel: (100, 200)"));
    assert!(text.contains("Y: 128"));
    assert!(text.contains("UV: (64, 192)"));
    assert!(!text.contains("Block:"));
    assert!(!text.contains("QP:"));
    assert!(!text.contains("MV:"));
}

#[test]
fn test_player_tooltip_block_hover() {
    // Test block hover with QP and MV
    let tooltip = PlayerTooltip {
        frame_idx: 42,
        pixel_xy: (100, 200),
        luma: Some(128),
        chroma: Some((64, 192)),
        block_id: Some("B_16x16_42".to_string()),
        qp: Some(28.5),
        mv: Some((3.5, -2.0)),
        partition_info: Some("16x16".to_string()),
        active_overlays: vec!["QP".to_string(), "MV".to_string()],
        syntax_path: None,
        bit_offset: None,
        byte_offset: None,
        copy_actions: vec![],
    };

    let text = tooltip.format();
    assert!(text.contains("Frame: 42"));
    assert!(text.contains("Pixel: (100, 200)"));
    assert!(text.contains("Y: 128"));
    assert!(text.contains("UV: (64, 192)"));
    assert!(text.contains("Block: B_16x16_42"));
    assert!(text.contains("QP: 28.5"));
    assert!(text.contains("MV: (3.5, -2.0)"));
    assert!(text.contains("Partition: 16x16"));
    assert!(text.contains("Overlays: QP, MV"));

    // Verify MV magnitude is included
    let magnitude = (3.5_f32 * 3.5 + 2.0 * 2.0).sqrt();
    assert!(text.contains(&format!("[{:.1}px]", magnitude)));
}

#[test]
fn test_player_tooltip_ws_player_spatial_compliance() {
    // Test WS_PLAYER_SPATIAL compliance:
    // - Pixel hover: x,y + luma/chroma
    // - Block hover: block id, qp, mv, partition type

    let tooltip = PlayerTooltip {
        frame_idx: 10,
        pixel_xy: (50, 75),
        luma: Some(100),
        chroma: Some((50, 150)),
        block_id: Some("B_8x8_5".to_string()),
        qp: Some(30.0),
        mv: Some((1.0, 2.0)),
        partition_info: Some("8x8".to_string()),
        active_overlays: vec![],
        syntax_path: None,
        bit_offset: None,
        byte_offset: None,
        copy_actions: vec![],
    };

    let text = tooltip.format();

    // Pixel hover requirements
    assert!(text.contains("Pixel: (50, 75)"), "Must show pixel x,y");
    assert!(text.contains("Y: 100"), "Must show luma");
    assert!(text.contains("UV: (50, 150)"), "Must show chroma");

    // Block hover requirements
    assert!(text.contains("Block: B_8x8_5"), "Must show block id");
    assert!(text.contains("QP: 30.0"), "Must show QP");
    assert!(text.contains("MV: (1.0, 2.0)"), "Must show MV");
    assert!(text.contains("Partition: 8x8"), "Must show partition type");
}
