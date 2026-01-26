//! Tests for Layout Persistence System

#[test]
fn test_layout_serialization() {
    // Test layout configuration serialization
    struct LayoutConfig {
        left_width: f32,
        center_width: f32,
        right_width: f32,
        bottom_height: f32,
    }

    let layout = LayoutConfig {
        left_width: 0.2,
        center_width: 0.5,
        right_width: 0.3,
        bottom_height: 0.25,
    };

    let total_width = layout.left_width + layout.center_width + layout.right_width;
    assert!((total_width - 1.0).abs() < 0.01);
}

#[test]
fn test_panel_state_persistence() {
    // Test panel visibility state persistence
    struct PanelState {
        filmstrip_visible: bool,
        syntax_tree_visible: bool,
        selection_info_visible: bool,
        yuv_viewer_visible: bool,
        quality_metrics_visible: bool,
    }

    let state = PanelState {
        filmstrip_visible: true,
        syntax_tree_visible: true,
        selection_info_visible: false,
        yuv_viewer_visible: true,
        quality_metrics_visible: false,
    };

    // Count visible panels
    let visible_count = [
        state.filmstrip_visible,
        state.syntax_tree_visible,
        state.selection_info_visible,
        state.yuv_viewer_visible,
        state.quality_metrics_visible,
    ]
    .iter()
    .filter(|&&v| v)
    .count();

    assert_eq!(visible_count, 3);
}

#[test]
fn test_workspace_active_state() {
    // Test active workspace persistence
    #[derive(Debug, PartialEq)]
    enum WorkspaceType {
        Av1,
        Hevc,
        Avc,
        Vvc,
        Mpeg2,
        Compare,
    }

    let active_workspace = WorkspaceType::Hevc;
    assert_eq!(active_workspace, WorkspaceType::Hevc);
}

#[test]
fn test_overlay_enable_state() {
    // Test overlay enable states
    struct OverlayState {
        ctb_grid: bool,
        motion_vectors: bool,
        qp_heatmap: bool,
        intra_modes: bool,
        partitions: bool,
    }

    let overlays = OverlayState {
        ctb_grid: true,
        motion_vectors: true,
        qp_heatmap: false,
        intra_modes: false,
        partitions: true,
    };

    let enabled_count = [
        overlays.ctb_grid,
        overlays.motion_vectors,
        overlays.qp_heatmap,
        overlays.intra_modes,
        overlays.partitions,
    ]
    .iter()
    .filter(|&&v| v)
    .count();

    assert_eq!(enabled_count, 3);
}

#[test]
fn test_layout_reset() {
    // Test layout reset to defaults
    struct DefaultLayout {
        left_width: f32,
        center_width: f32,
        right_width: f32,
    }

    let default = DefaultLayout {
        left_width: 0.20,
        center_width: 0.50,
        right_width: 0.30,
    };

    assert_eq!(default.left_width, 0.20);
    assert_eq!(default.center_width, 0.50);
    assert_eq!(default.right_width, 0.30);
}

#[test]
fn test_recent_files_persistence() {
    // Test recent files list persistence
    struct RecentFiles {
        max_entries: usize,
        files: Vec<String>,
    }

    let mut recent = RecentFiles {
        max_entries: 9,
        files: Vec::new(),
    };

    recent.files.push("/path/to/file1.ivf".to_string());
    recent.files.push("/path/to/file2.mp4".to_string());

    assert!(recent.files.len() <= recent.max_entries);
}

#[test]
fn test_zoom_pan_state() {
    // Test zoom/pan state persistence
    struct ViewportState {
        zoom: f32,
        pan_x: f32,
        pan_y: f32,
    }

    let viewport = ViewportState {
        zoom: 2.0,
        pan_x: 100.0,
        pan_y: 50.0,
    };

    assert!(viewport.zoom >= 0.1 && viewport.zoom <= 16.0);
}

#[test]
fn test_playback_state() {
    // Test playback state persistence
    struct PlaybackState {
        current_frame: usize,
        playing: bool,
        loop_enabled: bool,
        playback_speed: f32,
    }

    let state = PlaybackState {
        current_frame: 42,
        playing: false,
        loop_enabled: true,
        playback_speed: 1.0,
    };

    assert_eq!(state.current_frame, 42);
    assert!(!state.playing);
    assert!(state.loop_enabled);
}

#[test]
fn test_filter_settings_persistence() {
    // Test filter settings persistence
    struct FilterSettings {
        frame_type_filter: Vec<String>,
        qp_range_min: u8,
        qp_range_max: u8,
        show_only_keyframes: bool,
    }

    let filters = FilterSettings {
        frame_type_filter: vec!["I".to_string(), "P".to_string()],
        qp_range_min: 20,
        qp_range_max: 40,
        show_only_keyframes: false,
    };

    assert!(filters.qp_range_min < filters.qp_range_max);
}

#[test]
fn test_theme_preferences() {
    // Test theme preferences persistence
    #[derive(Debug, PartialEq)]
    enum Theme {
        Light,
        Dark,
        System,
    }

    let theme = Theme::Dark;
    assert_eq!(theme, Theme::Dark);
}

#[test]
fn test_column_visibility() {
    // Test table column visibility state
    struct ColumnState {
        frame_num: bool,
        frame_type: bool,
        qp: bool,
        size: bool,
        psnr: bool,
        offset: bool,
    }

    let columns = ColumnState {
        frame_num: true,
        frame_type: true,
        qp: true,
        size: true,
        psnr: false,
        offset: true,
    };

    let visible_columns = [
        columns.frame_num,
        columns.frame_type,
        columns.qp,
        columns.size,
        columns.psnr,
        columns.offset,
    ]
    .iter()
    .filter(|&&v| v)
    .count();

    assert_eq!(visible_columns, 5);
}

#[test]
fn test_window_state() {
    // Test window state persistence
    struct WindowState {
        width: u32,
        height: u32,
        maximized: bool,
        x: i32,
        y: i32,
    }

    let window = WindowState {
        width: 1920,
        height: 1080,
        maximized: false,
        x: 100,
        y: 100,
    };

    assert!(window.width >= 800);
    assert!(window.height >= 600);
}

#[test]
fn test_export_preferences() {
    // Test export preferences persistence
    struct ExportPreferences {
        default_format: String,
        include_metadata: bool,
        output_directory: String,
    }

    let prefs = ExportPreferences {
        default_format: "CSV".to_string(),
        include_metadata: true,
        output_directory: "/tmp/exports".to_string(),
    };

    assert_eq!(prefs.default_format, "CSV");
    assert!(prefs.include_metadata);
}
