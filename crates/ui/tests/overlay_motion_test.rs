//! Tests for Motion Vector Overlay System

#[test]
fn test_motion_vector_representation() {
    // Test motion vector data structure
    struct MotionVector {
        x: i16,
        y: i16,
        ref_frame: usize,
    }

    let mv = MotionVector {
        x: 32,
        y: -16,
        ref_frame: 0,
    };

    assert_eq!(mv.x, 32);
    assert_eq!(mv.y, -16);
}

#[test]
fn test_mv_magnitude_calculation() {
    // Test motion vector magnitude
    fn calculate_magnitude(mv_x: i16, mv_y: i16) -> f32 {
        ((mv_x as f32).powi(2) + (mv_y as f32).powi(2)).sqrt()
    }

    let magnitude = calculate_magnitude(3, 4);
    assert_eq!(magnitude, 5.0);
}

#[test]
fn test_mv_angle_calculation() {
    // Test motion vector angle
    fn calculate_angle(mv_x: i16, mv_y: i16) -> f32 {
        (mv_y as f32).atan2(mv_x as f32)
    }

    let angle = calculate_angle(1, 0);
    assert!((angle - 0.0).abs() < 0.01);
}

#[test]
fn test_mv_arrow_rendering() {
    // Test motion vector arrow rendering
    struct MvArrow {
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        thickness: f32,
        arrowhead_size: f32,
    }

    impl MvArrow {
        fn from_mv(block_x: f32, block_y: f32, mv_x: i16, mv_y: i16, scale: f32) -> Self {
            let scaled_mv_x = mv_x as f32 * scale;
            let scaled_mv_y = mv_y as f32 * scale;

            Self {
                start_x: block_x,
                start_y: block_y,
                end_x: block_x + scaled_mv_x,
                end_y: block_y + scaled_mv_y,
                thickness: 2.0,
                arrowhead_size: 8.0,
            }
        }
    }

    let arrow = MvArrow::from_mv(100.0, 100.0, 16, -8, 1.0);
    assert_eq!(arrow.end_x, 116.0);
    assert_eq!(arrow.end_y, 92.0);
}

#[test]
fn test_mv_magnitude_heatmap() {
    // Test magnitude-based coloring
    fn magnitude_to_color(magnitude: f32, max_magnitude: f32) -> (u8, u8, u8) {
        let normalized = (magnitude / max_magnitude).min(1.0);

        // Blue (low) -> Green (medium) -> Red (high)
        if normalized < 0.5 {
            let t = normalized * 2.0;
            (0, (t * 255.0) as u8, 255)
        } else {
            let t = (normalized - 0.5) * 2.0;
            ((t * 255.0) as u8, 255, ((1.0 - t) * 255.0) as u8)
        }
    }

    let low_color = magnitude_to_color(1.0, 10.0);
    let high_color = magnitude_to_color(10.0, 10.0);

    assert!(low_color.2 > 200); // Blue component high for low magnitude
    assert!(high_color.0 > 200); // Red component high for high magnitude
}

#[test]
fn test_mv_filtering() {
    // Test filtering motion vectors
    struct MvFilter {
        min_magnitude: f32,
        max_magnitude: f32,
        show_zero_mvs: bool,
    }

    impl MvFilter {
        fn should_display(&self, mv_x: i16, mv_y: i16) -> bool {
            let magnitude = ((mv_x as f32).powi(2) + (mv_y as f32).powi(2)).sqrt();

            if !self.show_zero_mvs && mv_x == 0 && mv_y == 0 {
                return false;
            }

            magnitude >= self.min_magnitude && magnitude <= self.max_magnitude
        }
    }

    let filter = MvFilter {
        min_magnitude: 2.0,
        max_magnitude: 100.0,
        show_zero_mvs: false,
    };

    assert!(filter.should_display(3, 4)); // magnitude = 5.0
    assert!(!filter.should_display(0, 0)); // zero MV
    assert!(!filter.should_display(1, 0)); // magnitude too small
}

#[test]
fn test_mv_block_association() {
    // Test associating MVs with blocks
    struct MvBlock {
        block_x: usize,
        block_y: usize,
        block_width: usize,
        block_height: usize,
        mv_x: i16,
        mv_y: i16,
    }

    impl MvBlock {
        fn center(&self) -> (f32, f32) {
            (
                self.block_x as f32 + self.block_width as f32 / 2.0,
                self.block_y as f32 + self.block_height as f32 / 2.0,
            )
        }
    }

    let block = MvBlock {
        block_x: 64,
        block_y: 64,
        block_width: 16,
        block_height: 16,
        mv_x: 8,
        mv_y: -4,
    };

    assert_eq!(block.center(), (72.0, 72.0));
}

#[test]
fn test_mv_scaling_with_zoom() {
    // Test MV arrow scaling with zoom level
    fn scale_mv_arrow(base_length: f32, zoom_level: f32) -> f32 {
        base_length * zoom_level.max(0.5)
    }

    let scaled = scale_mv_arrow(16.0, 2.0);
    assert_eq!(scaled, 32.0);

    let min_scaled = scale_mv_arrow(16.0, 0.25);
    assert_eq!(min_scaled, 8.0); // min 0.5x
}

#[test]
fn test_mv_quarter_pixel_precision() {
    // Test quarter-pixel MV precision
    fn mv_to_pixels(mv_quarter_pixels: i16) -> f32 {
        mv_quarter_pixels as f32 / 4.0
    }

    assert_eq!(mv_to_pixels(16), 4.0); // 16 quarter-pixels = 4 pixels
    assert_eq!(mv_to_pixels(1), 0.25); // 1 quarter-pixel = 0.25 pixels
}

#[test]
fn test_bidirectional_mvs() {
    // Test bidirectional motion vectors
    struct BidirectionalMv {
        fwd_x: i16,
        fwd_y: i16,
        bwd_x: i16,
        bwd_y: i16,
    }

    let bi_mv = BidirectionalMv {
        fwd_x: 8,
        fwd_y: -4,
        bwd_x: -6,
        bwd_y: 2,
    };

    assert_ne!(bi_mv.fwd_x, bi_mv.bwd_x);
}

#[test]
fn test_mv_reference_coloring() {
    // Test coloring MVs by reference frame
    fn get_ref_color(ref_frame: usize) -> (u8, u8, u8) {
        match ref_frame {
            0 => (255, 0, 0),     // LAST - Red
            1 => (0, 255, 0),     // GOLDEN - Green
            2 => (0, 0, 255),     // ALTREF - Blue
            _ => (128, 128, 128), // Unknown - Gray
        }
    }

    assert_eq!(get_ref_color(0), (255, 0, 0));
    assert_eq!(get_ref_color(1), (0, 255, 0));
}

#[test]
fn test_mv_grid_sampling() {
    // Test sampling MVs at grid points
    struct MvField {
        width: usize,
        height: usize,
        block_size: usize,
    }

    impl MvField {
        fn grid_positions(&self) -> Vec<(usize, usize)> {
            let mut positions = Vec::new();
            for y in (0..self.height).step_by(self.block_size) {
                for x in (0..self.width).step_by(self.block_size) {
                    positions.push((x, y));
                }
            }
            positions
        }
    }

    let field = MvField {
        width: 128,
        height: 128,
        block_size: 16,
    };

    let positions = field.grid_positions();
    assert_eq!(positions.len(), 64); // 8x8 grid
}

#[test]
fn test_mv_subpixel_refinement() {
    // Test subpixel motion vector refinement
    struct SubpixelMv {
        integer_x: i16,
        integer_y: i16,
        frac_x: u8, // 0-15 for 1/16 pixel
        frac_y: u8,
    }

    impl SubpixelMv {
        fn to_full_pixels(&self) -> (f32, f32) {
            (
                self.integer_x as f32 + self.frac_x as f32 / 16.0,
                self.integer_y as f32 + self.frac_y as f32 / 16.0,
            )
        }
    }

    let mv = SubpixelMv {
        integer_x: 3,
        integer_y: -2,
        frac_x: 8, // 0.5 pixel
        frac_y: 4, // 0.25 pixel
    };

    assert_eq!(mv.to_full_pixels(), (3.5, -1.75));
}

#[test]
fn test_mv_density_visualization() {
    // Test visualizing MV density
    fn calculate_mv_density(mvs_in_region: usize, region_area: usize) -> f32 {
        mvs_in_region as f32 / region_area as f32
    }

    let density = calculate_mv_density(100, 1000);
    assert_eq!(density, 0.1);
}
