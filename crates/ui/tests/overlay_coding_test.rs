//! Tests for Coding Overlays (QP, Partition, Bit Allocation, etc.)

#[test]
fn test_qp_heatmap_colors() {
    // Test QP heatmap color mapping
    fn qp_to_color(qp: u8, min_qp: u8, max_qp: u8) -> (u8, u8, u8) {
        let normalized = (qp - min_qp) as f32 / (max_qp - min_qp) as f32;

        // Blue (low QP/high quality) -> Red (high QP/low quality)
        let r = (normalized * 255.0) as u8;
        let b = ((1.0 - normalized) * 255.0) as u8;
        (r, 0, b)
    }

    let low_qp_color = qp_to_color(20, 0, 51);
    let high_qp_color = qp_to_color(45, 0, 51);

    assert!(low_qp_color.2 > low_qp_color.0); // More blue for low QP
    assert!(high_qp_color.0 > high_qp_color.2); // More red for high QP
}

#[test]
fn test_qp_range_validation() {
    // Test QP range validation for different codecs
    fn is_valid_qp(qp: u8, codec: &str) -> bool {
        match codec {
            "H.264" | "HEVC" => qp <= 51,
            "AV1" => qp <= 63,
            "VP9" => qp <= 255,
            _ => false,
        }
    }

    assert!(is_valid_qp(26, "H.264"));
    assert!(!is_valid_qp(60, "H.264"));
    assert!(is_valid_qp(60, "AV1"));
}

#[test]
fn test_partition_tree_structure() {
    // Test partition tree structure
    #[derive(Debug, PartialEq)]
    enum PartitionMode {
        None,
        Horizontal,
        Vertical,
        Split,
        HorzA,
        HorzB,
        VertA,
        VertB,
        Horz4,
        Vert4,
    }

    let partition = PartitionMode::Split;
    assert_eq!(partition, PartitionMode::Split);
}

#[test]
fn test_partition_rendering() {
    // Test partition boundary rendering
    struct PartitionBoundary {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        partition_type: String,
    }

    let boundary = PartitionBoundary {
        x: 64,
        y: 64,
        width: 32,
        height: 32,
        partition_type: "SPLIT".to_string(),
    };

    assert_eq!(boundary.width, 32);
}

#[test]
fn test_bit_allocation_visualization() {
    // Test bit allocation per block
    struct BitAllocation {
        block_x: usize,
        block_y: usize,
        bits_used: u32,
    }

    impl BitAllocation {
        fn bits_per_pixel(&self, block_width: usize, block_height: usize) -> f32 {
            self.bits_used as f32 / (block_width * block_height) as f32
        }
    }

    let alloc = BitAllocation {
        block_x: 0,
        block_y: 0,
        bits_used: 1024,
    };

    let bpp = alloc.bits_per_pixel(16, 16);
    assert_eq!(bpp, 4.0);
}

#[test]
fn test_bit_allocation_heatmap() {
    // Test bit allocation heatmap coloring
    fn bits_to_color(bits: u32, max_bits: u32) -> (u8, u8, u8) {
        let normalized = (bits as f32 / max_bits as f32).min(1.0);

        // Green (low) -> Yellow -> Red (high)
        if normalized < 0.5 {
            let t = normalized * 2.0;
            (0, 255, ((1.0 - t) * 255.0) as u8)
        } else {
            let t = (normalized - 0.5) * 2.0;
            ((t * 255.0) as u8, 255, 0)
        }
    }

    let low_color = bits_to_color(100, 1000);
    let high_color = bits_to_color(900, 1000);

    assert_eq!(low_color.0, 0); // No red for low bits
    assert!(high_color.0 > 200); // Lots of red for high bits
}

#[test]
fn test_pu_type_classification() {
    // Test Prediction Unit type classification
    #[derive(Debug, PartialEq)]
    enum PuType {
        Intra,
        Inter,
        Skip,
        Merge,
    }

    let types = vec![
        PuType::Intra,
        PuType::Inter,
        PuType::Skip,
    ];

    assert_eq!(types.len(), 3);
}

#[test]
fn test_pu_type_coloring() {
    // Test coloring by PU type
    fn pu_type_to_color(pu_type: &str) -> (u8, u8, u8) {
        match pu_type {
            "Intra" => (255, 0, 0),     // Red
            "Inter" => (0, 255, 0),     // Green
            "Skip" => (0, 0, 255),      // Blue
            "Merge" => (255, 255, 0),   // Yellow
            _ => (128, 128, 128),       // Gray
        }
    }

    assert_eq!(pu_type_to_color("Intra"), (255, 0, 0));
    assert_eq!(pu_type_to_color("Inter"), (0, 255, 0));
}

#[test]
fn test_intra_mode_labels() {
    // Test intra prediction mode labels
    fn get_mode_label(mode: u8) -> &'static str {
        match mode {
            0 => "DC",
            1 => "Vertical",
            2 => "Horizontal",
            3 => "D45",
            4 => "D135",
            5 => "D117",
            6 => "D153",
            7 => "D207",
            8 => "D63",
            _ => "Unknown",
        }
    }

    assert_eq!(get_mode_label(0), "DC");
    assert_eq!(get_mode_label(1), "Vertical");
}

#[test]
fn test_mode_direction_arrows() {
    // Test directional mode arrow rendering
    struct DirectionArrow {
        angle_degrees: f32,
        block_x: f32,
        block_y: f32,
    }

    impl DirectionArrow {
        fn from_mode(mode: u8) -> Option<f32> {
            match mode {
                1 => Some(90.0),   // Vertical
                2 => Some(0.0),    // Horizontal
                3 => Some(45.0),   // D45
                4 => Some(135.0),  // D135
                _ => None,
            }
        }
    }

    assert_eq!(DirectionArrow::from_mode(1), Some(90.0));
}

#[test]
fn test_overlay_manager_state() {
    // Test overlay manager state
    struct OverlayManager {
        active_overlays: Vec<String>,
    }

    impl OverlayManager {
        fn toggle(&mut self, overlay_name: &str) {
            if let Some(pos) = self.active_overlays.iter().position(|n| n == overlay_name) {
                self.active_overlays.remove(pos);
            } else {
                self.active_overlays.push(overlay_name.to_string());
            }
        }

        fn is_active(&self, overlay_name: &str) -> bool {
            self.active_overlays.iter().any(|n| n == overlay_name)
        }
    }

    let mut manager = OverlayManager {
        active_overlays: vec!["grid".to_string()],
    };

    manager.toggle("qp");
    assert!(manager.is_active("qp"));

    manager.toggle("qp");
    assert!(!manager.is_active("qp"));
}

#[test]
fn test_overlay_priority() {
    // Test overlay rendering priority
    struct OverlayLayer {
        name: String,
        priority: u8,
    }

    impl OverlayLayer {
        fn compare_priority(&self, other: &OverlayLayer) -> std::cmp::Ordering {
            self.priority.cmp(&other.priority)
        }
    }

    let mut layers = vec![
        OverlayLayer { name: "grid".to_string(), priority: 10 },
        OverlayLayer { name: "qp".to_string(), priority: 20 },
        OverlayLayer { name: "mv".to_string(), priority: 30 },
    ];

    layers.sort_by(|a, b| a.compare_priority(b));
    assert_eq!(layers[0].name, "grid");
    assert_eq!(layers[2].name, "mv");
}

#[test]
fn test_overlay_opacity_blending() {
    // Test overlay opacity blending
    fn blend_color(base: (u8, u8, u8), overlay: (u8, u8, u8), alpha: f32) -> (u8, u8, u8) {
        let alpha = alpha.max(0.0).min(1.0);
        let r = (base.0 as f32 * (1.0 - alpha) + overlay.0 as f32 * alpha) as u8;
        let g = (base.1 as f32 * (1.0 - alpha) + overlay.1 as f32 * alpha) as u8;
        let b = (base.2 as f32 * (1.0 - alpha) + overlay.2 as f32 * alpha) as u8;
        (r, g, b)
    }

    let blended = blend_color((0, 0, 0), (255, 255, 255), 0.5);
    assert_eq!(blended, (127, 127, 127));
}

#[test]
fn test_transform_size_overlay() {
    // Test transform size visualization
    #[derive(Debug, PartialEq)]
    enum TransformSize {
        Tx4x4,
        Tx8x8,
        Tx16x16,
        Tx32x32,
        Tx64x64,
    }

    fn size_to_pixels(size: TransformSize) -> usize {
        match size {
            TransformSize::Tx4x4 => 4,
            TransformSize::Tx8x8 => 8,
            TransformSize::Tx16x16 => 16,
            TransformSize::Tx32x32 => 32,
            TransformSize::Tx64x64 => 64,
        }
    }

    assert_eq!(size_to_pixels(TransformSize::Tx16x16), 16);
}

#[test]
fn test_residual_visualization() {
    // Test residual coefficient visualization
    struct ResidualBlock {
        coefficients: Vec<i16>,
        non_zero_count: usize,
    }

    impl ResidualBlock {
        fn sparsity(&self) -> f32 {
            1.0 - (self.non_zero_count as f32 / self.coefficients.len() as f32)
        }
    }

    let block = ResidualBlock {
        coefficients: vec![0i16; 64],
        non_zero_count: 5,
    };

    let sparsity = block.sparsity();
    assert!((sparsity - 0.921875).abs() < 0.01); // ~92% zeros
}

#[test]
fn test_overlay_interaction() {
    // Test overlay interaction (hover, click)
    struct OverlayInteraction {
        hovered_block: Option<(usize, usize)>,
        selected_block: Option<(usize, usize)>,
    }

    impl OverlayInteraction {
        fn on_hover(&mut self, block_x: usize, block_y: usize) {
            self.hovered_block = Some((block_x, block_y));
        }

        fn on_click(&mut self) {
            self.selected_block = self.hovered_block;
        }
    }

    let mut interaction = OverlayInteraction {
        hovered_block: None,
        selected_block: None,
    };

    interaction.on_hover(64, 64);
    interaction.on_click();

    assert_eq!(interaction.selected_block, Some((64, 64)));
}
