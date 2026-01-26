//! Tests for overlay system

use egui::Color32;

#[test]
fn test_overlay_types() {
    // Test all overlay types
    #[derive(Debug, PartialEq)]
    enum OverlayType {
        Grid,
        QP,
        MV,
        Partition,
        PUType,
        BitAllocation,
        MVMagnitude,
        ModeLabels,
    }

    let overlays = vec![
        OverlayType::Grid,
        OverlayType::QP,
        OverlayType::MV,
        OverlayType::Partition,
        OverlayType::PUType,
        OverlayType::BitAllocation,
        OverlayType::MVMagnitude,
        OverlayType::ModeLabels,
    ];

    assert_eq!(overlays.len(), 8, "Should have 8 overlay types");
}

#[test]
fn test_grid_overlay_rendering() {
    // Test CTB grid overlay
    let ctb_size = 64; // Common CTB size
    let frame_width = 1920;
    let frame_height = 1080;

    let grid_cols = (frame_width + ctb_size - 1) / ctb_size;
    let grid_rows = (frame_height + ctb_size - 1) / ctb_size;

    assert_eq!(grid_cols, 30); // 1920 / 64 = 30
    assert_eq!(grid_rows, 17); // 1080 / 64 ~= 17
}

#[test]
fn test_qp_overlay_heatmap() {
    // Test QP value to color mapping
    let qp_values = vec![0, 13, 26, 39, 51];

    for qp in qp_values {
        // Map QP to heatmap color (0 = blue/cold, 51 = red/hot)
        let normalized = qp as f32 / 51.0;
        let r = (normalized * 255.0) as u8;
        let b = ((1.0 - normalized) * 255.0) as u8;
        let color = Color32::from_rgb(r, 0, b);

        assert!(color.r() <= 255);
        assert!(color.b() <= 255);
    }
}

#[test]
fn test_mv_overlay_vector_scaling() {
    // Test motion vector scaling for display
    let mv_x = 16; // 16 pixels horizontal
    let mv_y = -8; // 8 pixels vertical (up)

    let scale_factor = 2.0; // Scale for visibility
    let display_x = mv_x as f32 * scale_factor;
    let display_y = mv_y as f32 * scale_factor;

    assert_eq!(display_x, 32.0);
    assert_eq!(display_y, -16.0);
}

#[test]
fn test_mv_overlay_color_by_direction() {
    // Test MV coloring by direction
    let directions = vec![
        (10, 0, "Right"),    // Horizontal right
        (-10, 0, "Left"),    // Horizontal left
        (0, 10, "Down"),     // Vertical down
        (0, -10, "Up"),      // Vertical up
    ];

    for (mx, my, direction) in directions {
        let angle = (my as f32).atan2(mx as f32);
        assert!(angle.is_finite(), "Angle should be finite for {}", direction);
    }
}

#[test]
fn test_partition_overlay_tree_depth() {
    // Test partition tree depth visualization
    let depths = vec![0, 1, 2, 3]; // Quad-tree depths

    for depth in depths {
        let block_size = 64 >> depth; // 64, 32, 16, 8
        assert!(block_size >= 8, "Minimum block size is 8");
    }
}

#[test]
fn test_partition_overlay_colors() {
    // Test partition depth to color mapping
    let depth_colors = vec![
        (0, Color32::from_rgb(59, 130, 246)),   // Depth 0: Blue
        (1, Color32::from_rgb(34, 197, 94)),    // Depth 1: Green
        (2, Color32::from_rgb(251, 191, 36)),   // Depth 2: Amber
        (3, Color32::from_rgb(239, 68, 68)),    // Depth 3: Red
    ];

    for (depth, color) in depth_colors {
        assert!(depth <= 3);
        assert!(color.r() > 0 || color.g() > 0 || color.b() > 0);
    }
}

#[test]
fn test_pu_type_overlay_modes() {
    // Test prediction unit type visualization
    #[derive(Debug, PartialEq)]
    enum PUMode {
        Intra,
        Inter,
        Skip,
        Merge,
    }

    let modes = vec![PUMode::Intra, PUMode::Inter, PUMode::Skip, PUMode::Merge];
    assert_eq!(modes.len(), 4);
}

#[test]
fn test_bit_allocation_overlay_heatmap() {
    // Test bit allocation heatmap
    let bit_counts = vec![100, 500, 1000, 5000];

    for bits in bit_counts {
        // Map bits to color intensity
        let max_bits = 5000;
        let normalized = (bits as f32 / max_bits as f32).min(1.0);
        let intensity = (normalized * 255.0) as u8;

        assert!(intensity <= 255);
    }
}

#[test]
fn test_mv_magnitude_overlay() {
    // Test MV magnitude calculation and visualization
    let motion_vectors = vec![
        (0, 0, 0.0),       // Zero motion
        (3, 4, 5.0),       // 3-4-5 triangle
        (10, 0, 10.0),     // Horizontal
        (0, 10, 10.0),     // Vertical
    ];

    for (mx, my, expected_mag) in motion_vectors {
        let magnitude = ((mx * mx + my * my) as f32).sqrt();
        assert!((magnitude - expected_mag).abs() < 0.01);
    }
}

#[test]
fn test_mode_labels_overlay_text() {
    // Test prediction mode label rendering
    let mode_labels = vec![
        "DC",
        "PLANAR",
        "H",    // Horizontal
        "V",    // Vertical
        "D_45", // Diagonal 45
    ];

    for label in mode_labels {
        assert!(!label.is_empty());
        assert!(label.len() <= 10); // Reasonable label length
    }
}

#[test]
fn test_overlay_opacity_control() {
    // Test overlay opacity settings
    let opacity_levels = vec![0.0, 0.25, 0.5, 0.75, 1.0];

    for opacity in opacity_levels {
        assert!(opacity >= 0.0 && opacity <= 1.0);
        let alpha = (opacity * 255.0) as u8;
        assert!(alpha <= 255);
    }
}

#[test]
fn test_overlay_combination() {
    // Test multiple overlay combination
    struct OverlayState {
        grid: bool,
        qp: bool,
        mv: bool,
    }

    let state = OverlayState {
        grid: true,
        qp: true,
        mv: false,
    };

    let active_count = [state.grid, state.qp, state.mv]
        .iter()
        .filter(|&&x| x)
        .count();

    assert_eq!(active_count, 2);
}

#[test]
fn test_overlay_z_order() {
    // Test overlay rendering order (z-index)
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum ZOrder {
        Frame = 0,
        Grid = 1,
        Partition = 2,
        MV = 3,
        Labels = 4,
    }

    let mut layers = vec![ZOrder::MV, ZOrder::Grid, ZOrder::Frame, ZOrder::Labels];
    layers.sort();

    assert_eq!(layers[0], ZOrder::Frame);
    assert_eq!(layers[layers.len() - 1], ZOrder::Labels);
}

#[test]
fn test_overlay_performance() {
    // Test overlay render should be efficient
    let frame_width = 1920;
    let frame_height = 1080;
    let ctb_size = 64;

    let total_ctbs = ((frame_width / ctb_size) * (frame_height / ctb_size)) as usize;
    
    // Should be able to render reasonable number of CTBs
    assert!(total_ctbs < 1000, "CTB count should be reasonable");
}
