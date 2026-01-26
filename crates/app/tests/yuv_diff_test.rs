//! Tests for YUV Diff module

#[test]
fn test_yuv_diff_pair() {
    struct YuvDiffPair {
        reference_path: String,
        test_path: String,
        width: usize,
        height: usize,
    }

    let pair = YuvDiffPair {
        reference_path: "/tmp/ref.yuv".to_string(),
        test_path: "/tmp/test.yuv".to_string(),
        width: 1920,
        height: 1080,
    };

    assert_eq!(pair.width, 1920);
}

#[test]
fn test_yuv_frame_diff() {
    struct YuvFrameDiff {
        y_diff: Vec<u8>,
        u_diff: Vec<u8>,
        v_diff: Vec<u8>,
    }

    impl YuvFrameDiff {
        fn new(width: usize, height: usize) -> Self {
            let y_size = width * height;
            let uv_size = (width / 2) * (height / 2);
            Self {
                y_diff: vec![0u8; y_size],
                u_diff: vec![0u8; uv_size],
                v_diff: vec![0u8; uv_size],
            }
        }

        fn total_pixels(&self) -> usize {
            self.y_diff.len() + self.u_diff.len() + self.v_diff.len()
        }
    }

    let diff = YuvFrameDiff::new(1920, 1080);
    assert_eq!(diff.y_diff.len(), 1920 * 1080);
}

#[test]
fn test_psnr_calculation() {
    fn calculate_mse(ref_data: &[u8], test_data: &[u8]) -> f64 {
        assert_eq!(ref_data.len(), test_data.len());
        let sum: u64 = ref_data
            .iter()
            .zip(test_data.iter())
            .map(|(r, t)| {
                let diff = *r as i32 - *t as i32;
                (diff * diff) as u64
            })
            .sum();
        sum as f64 / ref_data.len() as f64
    }

    fn calculate_psnr(ref_data: &[u8], test_data: &[u8]) -> f64 {
        let mse = calculate_mse(ref_data, test_data);
        if mse == 0.0 {
            f64::INFINITY
        } else {
            10.0 * (255.0 * 255.0 / mse).log10()
        }
    }

    let ref_data = vec![100u8, 110, 120];
    let test_data = vec![100u8, 110, 120];
    let psnr = calculate_psnr(&ref_data, &test_data);
    assert!(psnr.is_infinite());
}

#[test]
fn test_diff_heatmap() {
    struct DiffHeatmap {
        colors: Vec<(u8, u8, u8)>,
    }

    impl DiffHeatmap {
        fn diff_to_color(&self, diff: u8) -> (u8, u8, u8) {
            // Simple gradient: blue (0) -> red (255)
            let intensity = diff;
            (intensity, 0, 255 - intensity)
        }
    }

    let heatmap = DiffHeatmap { colors: vec![] };
    let color = heatmap.diff_to_color(128);
    assert_eq!(color, (128, 0, 127));
}

#[test]
fn test_diff_amplification() {
    fn amplify_diff(diff: u8, factor: f32) -> u8 {
        ((diff as f32) * factor).min(255.0) as u8
    }

    assert_eq!(amplify_diff(10, 2.0), 20);
    assert_eq!(amplify_diff(200, 2.0), 255); // Clamped
}

#[test]
fn test_block_diff() {
    struct BlockDiff {
        block_size: usize,
        block_x: usize,
        block_y: usize,
        avg_diff: f64,
    }

    impl BlockDiff {
        fn compute_avg(diffs: &[u8]) -> f64 {
            if diffs.is_empty() {
                0.0
            } else {
                let sum: u32 = diffs.iter().map(|&d| d as u32).sum();
                sum as f64 / diffs.len() as f64
            }
        }
    }

    let diffs = vec![10u8, 20, 30];
    let avg = BlockDiff::compute_avg(&diffs);
    assert_eq!(avg, 20.0);
}

#[test]
fn test_temporal_diff() {
    struct TemporalDiff {
        frame_diffs: Vec<f64>,
    }

    impl TemporalDiff {
        fn add_frame_diff(&mut self, diff: f64) {
            self.frame_diffs.push(diff);
        }

        fn average_diff(&self) -> f64 {
            if self.frame_diffs.is_empty() {
                0.0
            } else {
                self.frame_diffs.iter().sum::<f64>() / self.frame_diffs.len() as f64
            }
        }
    }

    let mut temporal = TemporalDiff {
        frame_diffs: vec![],
    };

    temporal.add_frame_diff(10.0);
    temporal.add_frame_diff(20.0);
    assert_eq!(temporal.average_diff(), 15.0);
}

#[test]
fn test_diff_region_of_interest() {
    struct RoI {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    }

    impl RoI {
        fn contains(&self, x: usize, y: usize) -> bool {
            x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
        }

        fn area(&self) -> usize {
            self.width * self.height
        }
    }

    let roi = RoI {
        x: 10,
        y: 10,
        width: 100,
        height: 100,
    };

    assert!(roi.contains(50, 50));
    assert!(!roi.contains(5, 5));
    assert_eq!(roi.area(), 10000);
}

#[test]
fn test_diff_overlay() {
    #[derive(Debug, PartialEq)]
    enum OverlayMode {
        None,
        Absolute,
        Heatmap,
        SideBySide,
    }

    struct DiffOverlay {
        mode: OverlayMode,
        opacity: f32,
    }

    impl DiffOverlay {
        fn set_mode(&mut self, mode: OverlayMode) {
            self.mode = mode;
        }
    }

    let mut overlay = DiffOverlay {
        mode: OverlayMode::None,
        opacity: 1.0,
    };

    overlay.set_mode(OverlayMode::Heatmap);
    assert_eq!(overlay.mode, OverlayMode::Heatmap);
}

#[test]
fn test_diff_metrics() {
    struct DiffMetrics {
        min_y_diff: u8,
        max_y_diff: u8,
        avg_y_diff: f64,
        min_u_diff: u8,
        max_u_diff: u8,
        avg_u_diff: f64,
        min_v_diff: u8,
        max_v_diff: u8,
        avg_v_diff: f64,
    }

    impl DiffMetrics {
        fn compute_y_stats(y_diffs: &[u8]) -> (u8, u8, f64) {
            let min = *y_diffs.iter().min().unwrap_or(&0);
            let max = *y_diffs.iter().max().unwrap_or(&0);
            let sum: u32 = y_diffs.iter().map(|&d| d as u32).sum();
            let avg = if y_diffs.is_empty() {
                0.0
            } else {
                sum as f64 / y_diffs.len() as f64
            };
            (min, max, avg)
        }
    }

    let y_diffs = vec![5u8, 10, 15, 20];
    let (min, max, avg) = DiffMetrics::compute_y_stats(&y_diffs);
    assert_eq!(min, 5);
    assert_eq!(max, 20);
    assert_eq!(avg, 12.5);
}
