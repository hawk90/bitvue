// Tooltip module tests
use super::*;

// ============================================================================
// Fixtures
// ============================================================================
fn create_test_config() -> TooltipConfig {
    TooltipConfig {
        hover_delay_ms: 100,
        cursor_offset: (10, 10),
        max_width: 400,
        enable_copy_actions: true,
    }
}

fn create_test_timeline_tooltip() -> TimelineTooltip {
    TimelineTooltip {
        frame_idx: 42,
        frame_type: "I".to_string(),
        pts: Some(12345),
        dts: Some(12300),
        time_seconds: Some(12.345),
        size_bytes: Some(50000),
        size_bits: Some(400000),
        markers: vec!["Keyframe".to_string()],
        decoded: true,
        decode_error: None,
        syntax_path: Some("OBU_FRAME.tile[0]".to_string()),
        bit_offset: Some(1024),
        byte_offset: Some(128),
        copy_actions: vec![],
    }
}

fn create_test_metrics_tooltip() -> MetricsTooltip {
    MetricsTooltip {
        frame_idx: 42,
        time_seconds: Some(1.234),
        series_name: "PSNR_Y".to_string(),
        value: 42.5,
        unit: "dB".to_string(),
        delta: Some(0.5),
        copy_actions: vec![],
    }
}

fn create_test_tree_tooltip() -> TreeTooltip {
    TreeTooltip {
        path: vec!["root".to_string(), "child".to_string(), "leaf".to_string()],
        offset_hex: "0x400".to_string(),
        size_bytes: 32,
        unit_type: "OBU".to_string(),
        flags: vec!["key".to_string()],
        parsed: true,
        diagnostic_summary: None,
        copy_actions: vec![],
    }
}

fn create_test_syntax_tooltip() -> SyntaxTooltip {
    SyntaxTooltip {
        field_name: "frame_type".to_string(),
        field_type: "u8".to_string(),
        decoded_value: "KEY_FRAME".to_string(),
        bit_range: (0, 3),
        raw_bits: "001".to_string(),
        condition: None,
        copy_actions: vec![],
    }
}

fn create_test_hex_bit_tooltip() -> HexBitTooltip {
    HexBitTooltip {
        offset_hex: "0x800".to_string(),
        byte_hex: "41".to_string(),  // ASCII 'A'
        byte_decimal: 65,
        ascii_char: Some('A'),  // ASCII character
        bit_in_byte: Some(3),
        global_bit_offset: Some(2048),
        mapped_field: Some("show_existing_frame".to_string()),
        copy_actions: vec![],
    }
}

fn create_test_player_tooltip() -> PlayerTooltip {
    PlayerTooltip {
        frame_idx: 42,
        pixel_xy: (100, 200),
        luma: Some(128),
        chroma: Some((128, 128)),
        block_id: Some("CTU 42".to_string()),
        qp: Some(25.0),
        mv: Some((1.5, 2.0)),
        partition_info: Some("128x128".to_string()),
        active_overlays: vec!["QP".to_string(), "MV".to_string()],
        syntax_path: Some("OBU_FRAME.tile[0].sb[5]".to_string()),
        bit_offset: Some(1024),
        byte_offset: Some(128),
        copy_actions: vec![],
    }
}

fn create_test_diagnostics_tooltip() -> DiagnosticsTooltip {
    DiagnosticsTooltip {
        severity: "Error".to_string(),
        category: "Bitstream".to_string(),
        offset_hex: Some("0x400".to_string()),
        frame_unit_refs: vec!["Frame 42".to_string()],
        full_message: "Invalid syntax".to_string(),
        root_cause_chain: vec!["Parse error".to_string()],
        copy_actions: vec![],
    }
}

fn create_test_manager() -> TooltipManager {
    let mut manager = TooltipManager::new(create_test_config());
    manager
}

// ============================================================================
// TooltipConfig Tests
// ============================================================================
#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TooltipConfig::default();
        assert_eq!(config.hover_delay_ms, 150);
        assert_eq!(config.cursor_offset, (10, 10));
        assert_eq!(config.max_width, 400);
        assert!(config.enable_copy_actions);
    }

    #[test]
    fn test_config_custom() {
        let config = TooltipConfig {
            hover_delay_ms: 500,
            cursor_offset: (20, 20),
            max_width: 800,
            enable_copy_actions: false,
        };

        assert_eq!(config.hover_delay_ms, 500);
        assert_eq!(config.cursor_offset, (20, 20));
        assert_eq!(config.max_width, 800);
        assert!(!config.enable_copy_actions);
    }
}

// ============================================================================
// TooltipContent Tests
// ============================================================================
#[cfg(test)]
mod content_tests {
    use super::*;

    #[test]
    fn test_content_timeline() {
        let content = TooltipContent::Timeline(create_test_timeline_tooltip());
        if let TooltipContent::Timeline(t) = content {
            assert_eq!(t.frame_idx, 42);
            assert_eq!(t.pts, Some(12345));
            assert_eq!(t.frame_type, "I");
        } else {
            panic!("Expected Timeline content");
        }
    }

    #[test]
    fn test_content_metrics() {
        let content = TooltipContent::Metrics(create_test_metrics_tooltip());
        if let TooltipContent::Metrics(m) = content {
            assert_eq!(m.series_name, "PSNR_Y");
            assert!((m.value - 42.5).abs() < 0.01);
        } else {
            panic!("Expected Metrics content");
        }
    }

    #[test]
    fn test_content_tree() {
        let content = TooltipContent::Tree(create_test_tree_tooltip());
        if let TooltipContent::Tree(t) = content {
            assert_eq!(t.path.len(), 3);
            assert_eq!(t.size_bytes, 32);
        } else {
            panic!("Expected Tree content");
        }
    }

    #[test]
    fn test_content_syntax() {
        let content = TooltipContent::Syntax(create_test_syntax_tooltip());
        if let TooltipContent::Syntax(s) = content {
            assert_eq!(s.field_name, "frame_type");
            assert_eq!(s.bit_range, (0, 3));
        } else {
            panic!("Expected Syntax content");
        }
    }

    #[test]
    fn test_content_hex_bit() {
        let content = TooltipContent::HexBit(create_test_hex_bit_tooltip());
        if let TooltipContent::HexBit(h) = content {
            assert_eq!(h.byte_decimal, 65);  // Updated to match new test data (ASCII 'A')
            assert_eq!(h.bit_in_byte, Some(3));
        } else {
            panic!("Expected HexBit content");
        }
    }

    #[test]
    fn test_content_player() {
        let content = TooltipContent::Player(create_test_player_tooltip());
        if let TooltipContent::Player(p) = content {
            assert_eq!(p.pixel_xy, (100, 200));
            assert_eq!(p.block_id, Some("CTU 42".to_string()));
        } else {
            panic!("Expected Player content");
        }
    }

    #[test]
    fn test_content_diagnostics() {
        let content = TooltipContent::Diagnostics(create_test_diagnostics_tooltip());
        if let TooltipContent::Diagnostics(d) = content {
            assert_eq!(d.severity, "Error");
            assert_eq!(d.category, "Bitstream");
        } else {
            panic!("Expected Diagnostics content");
        }
    }
}

// ============================================================================
// TooltipManager Tests
// ============================================================================
#[cfg(test)]
mod manager_tests {
    use super::*;

    #[test]
    fn test_manager_new_with_config() {
        let config = create_test_config();
        let manager = TooltipManager::new(config);
        assert!(manager.current_tooltip.is_none());
    }

    #[test]
    fn test_manager_default() {
        let manager = TooltipManager::default();
        assert!(manager.current_tooltip.is_none());
        assert_eq!(manager.config.hover_delay_ms, 150);
    }

    #[test]
    fn test_manager_show() {
        let mut manager = create_test_manager();
        let content = TooltipContent::Timeline(create_test_timeline_tooltip());
        manager.show(content, (100, 200));
        assert!(manager.current_tooltip.is_some());
    }

    #[test]
    fn test_manager_hide() {
        let mut manager = create_test_manager();
        let content = TooltipContent::Timeline(create_test_timeline_tooltip());
        manager.show(content, (100, 200));
        manager.hide();
        assert!(manager.current_tooltip.is_none());
    }

    #[test]
    fn test_manager_update_position() {
        let mut manager = create_test_manager();
        let content = TooltipContent::Timeline(create_test_timeline_tooltip());
        manager.show(content, (100, 200));
        manager.update_position((150, 250));
        assert_eq!(manager.current().unwrap().position, (150, 250));
    }

    #[test]
    fn test_manager_current() {
        let mut manager = create_test_manager();
        assert!(manager.current().is_none());
        let content = TooltipContent::Timeline(create_test_timeline_tooltip());
        manager.show(content, (100, 200));
        assert!(manager.current().is_some());
    }
}

// ============================================================================
// TooltipState Tests
// ============================================================================
#[cfg(test)]
mod state_tests {
    use super::*;

    #[test]
    fn test_state_format_text_timeline() {
        let state = TooltipState {
            content: TooltipContent::Timeline(create_test_timeline_tooltip()),
            position: (100, 200),
            visible: true,
        };
        let text = state.format_text();
        assert!(text.contains("Frame: 42"));
        assert!(text.contains("PTS: 12345 ms"));
    }

    #[test]
    fn test_state_format_text_metrics() {
        let state = TooltipState {
            content: TooltipContent::Metrics(create_test_metrics_tooltip()),
            position: (100, 200),
            visible: true,
        };
        let text = state.format_text();
        assert!(text.contains("Frame: 42"));
        assert!(text.contains("PSNR_Y: 42.50 dB"));
    }

    #[test]
    fn test_state_format_text_custom() {
        let state = TooltipState {
            content: TooltipContent::Custom("Custom tooltip text".to_string()),
            position: (100, 200),
            visible: true,
        };
        let text = state.format_text();
        assert_eq!(text, "Custom tooltip text");
    }

    #[test]
    fn test_state_copy_actions() {
        let tooltip = create_test_timeline_tooltip();
        let state = TooltipState {
            content: TooltipContent::Timeline(tooltip),
            position: (100, 200),
            visible: true,
        };
        let actions = state.copy_actions();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_state_copy_actions_custom() {
        let state = TooltipState {
            content: TooltipContent::Custom("Test".to_string()),
            position: (100, 200),
            visible: true,
        };
        let actions = state.copy_actions();
        assert!(actions.is_empty());
    }
}

// ============================================================================
// CopyAction Tests
// ============================================================================
#[cfg(test)]
mod copy_action_tests {
    use super::*;

    #[test]
    fn test_copy_action_new() {
        let action = CopyAction::new("Copy Frame", "Frame 42");
        assert_eq!(action.label, "Copy Frame");
        assert_eq!(action.content, "Frame 42");
    }
}

// ============================================================================
// Format Tests
// ============================================================================
#[cfg(test)]
mod format_tests {
    use super::*;

    #[test]
    fn test_timeline_tooltip_format() {
        let tooltip = create_test_timeline_tooltip();
        let text = tooltip.format();
        assert!(text.contains("Frame: 42 (I)"));
        assert!(text.contains("PTS: 12345 ms"));
        assert!(text.contains("DTS: 12300 ms"));
        assert!(text.contains("Time: 12.345 s"));
        assert!(text.contains("Size: 50000 bytes"));
    }

    #[test]
    fn test_metrics_tooltip_format() {
        let tooltip = create_test_metrics_tooltip();
        let text = tooltip.format();
        assert!(text.contains("Frame: 42"));
        assert!(text.contains("Time: 1.234 s"));
        assert!(text.contains("PSNR_Y: 42.50 dB"));
        assert!(text.contains("Δ: +0.50 dB"));
    }

    #[test]
    fn test_tree_tooltip_format() {
        let tooltip = create_test_tree_tooltip();
        let text = tooltip.format();
        assert!(text.contains("Path: root > child > leaf"));
        assert!(text.contains("Offset: 0x400 (32 bytes)"));
        assert!(text.contains("Type: OBU"));
        assert!(text.contains("Flags: key"));
        assert!(text.contains("Parsed: yes"));
    }

    #[test]
    fn test_syntax_tooltip_format() {
        let tooltip = create_test_syntax_tooltip();
        let text = tooltip.format();
        assert!(text.contains("frame_type: u8"));
        assert!(text.contains("Value: KEY_FRAME"));
        assert!(text.contains("Bits: 0..3 (3 bits)"));
        assert!(text.contains("Raw: 001"));
    }

    #[test]
    fn test_hex_bit_tooltip_format() {
        let tooltip = create_test_hex_bit_tooltip();
        let text = tooltip.format();
        assert!(text.contains("Offset: 0x800"));
        assert!(text.contains("Byte: 41 (65)"));
        assert!(text.contains("ASCII: 'A'"));
        assert!(text.contains("Bit: 3 in byte"));
        assert!(text.contains("Global bit: 2048"));
    }

    #[test]
    fn test_player_tooltip_format() {
        let tooltip = create_test_player_tooltip();
        let text = tooltip.format();
        assert!(text.contains("Frame: 42"));
        assert!(text.contains("Pixel: (100, 200)"));
        assert!(text.contains("Y: 128"));
        assert!(text.contains("UV: (128, 128)"));
        assert!(text.contains("Block: CTU 42"));
        assert!(text.contains("QP: 25.0"));
        assert!(text.contains("MV: (1.5, 2.0) [2.5px]"));
    }

    #[test]
    fn test_diagnostics_tooltip_format() {
        let tooltip = create_test_diagnostics_tooltip();
        let text = tooltip.format();
        assert!(text.contains("Severity: Error"));
        assert!(text.contains("Category: Bitstream"));
        assert!(text.contains("Offset: 0x400"));
        assert!(text.contains("Refs: Frame 42"));
        assert!(text.contains("\nInvalid syntax"));
        assert!(text.contains("Root cause:"));
        assert!(text.contains("→ Parse error"));
    }
}
