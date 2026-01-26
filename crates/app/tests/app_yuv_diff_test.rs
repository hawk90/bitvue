//! Tests for App YUV Diff (YUV difference/comparison integration)

#[test]
fn test_yuv_diff_config() {
    struct YuvDiffConfig {
        reference_path: String,
        test_path: String,
        diff_mode: String,
    }

    let config = YuvDiffConfig {
        reference_path: "/tmp/ref.yuv".to_string(),
        test_path: "/tmp/test.yuv".to_string(),
        diff_mode: "absolute".to_string(),
    };

    assert_eq!(config.diff_mode, "absolute");
}

#[test]
fn test_pixel_difference() {
    fn absolute_difference(a: u8, b: u8) -> u8 {
        if a > b {
            a - b
        } else {
            b - a
        }
    }

    assert_eq!(absolute_difference(100, 80), 20);
    assert_eq!(absolute_difference(80, 100), 20);
}

#[test]
fn test_plane_diff() {
    struct PlaneDiff {
        width: usize,
        height: usize,
        differences: Vec<u8>,
    }

    impl PlaneDiff {
        fn compute(ref_plane: &[u8], test_plane: &[u8]) -> Self {
            assert_eq!(ref_plane.len(), test_plane.len());
            let differences = ref_plane
                .iter()
                .zip(test_plane.iter())
                .map(|(r, t)| if *r > *t { r - t } else { t - r })
                .collect();

            Self {
                width: 0,
                height: 0,
                differences,
            }
        }

        fn max_diff(&self) -> u8 {
            *self.differences.iter().max().unwrap_or(&0)
        }
    }

    let ref_data = vec![100u8, 120, 130];
    let test_data = vec![105u8, 110, 135];
    let diff = PlaneDiff::compute(&ref_data, &test_data);

    assert_eq!(diff.differences, vec![5, 10, 5]);
    assert_eq!(diff.max_diff(), 10);
}

#[test]
fn test_diff_statistics() {
    struct DiffStatistics {
        min_diff: u8,
        max_diff: u8,
        mean_diff: f64,
        pixels_different: usize,
        total_pixels: usize,
    }

    impl DiffStatistics {
        fn compute(differences: &[u8]) -> Self {
            let min_diff = *differences.iter().min().unwrap_or(&0);
            let max_diff = *differences.iter().max().unwrap_or(&0);
            let sum: u32 = differences.iter().map(|&d| d as u32).sum();
            let mean_diff = sum as f64 / differences.len() as f64;
            let pixels_different = differences.iter().filter(|&&d| d > 0).count();

            Self {
                min_diff,
                max_diff,
                mean_diff,
                pixels_different,
                total_pixels: differences.len(),
            }
        }

        fn percentage_different(&self) -> f64 {
            if self.total_pixels == 0 {
                0.0
            } else {
                (self.pixels_different as f64 / self.total_pixels as f64) * 100.0
            }
        }
    }

    let diffs = vec![0u8, 5, 10, 0, 15];
    let stats = DiffStatistics::compute(&diffs);

    assert_eq!(stats.min_diff, 0);
    assert_eq!(stats.max_diff, 15);
    assert_eq!(stats.pixels_different, 3);
}

#[test]
fn test_diff_visualization() {
    #[derive(Debug, PartialEq)]
    enum DiffMode {
        Absolute,
        Signed,
        Heatmap,
    }

    struct DiffVisualizer {
        mode: DiffMode,
        amplification: f32,
    }

    impl DiffVisualizer {
        fn visualize_pixel(&self, diff: i16) -> u8 {
            match self.mode {
                DiffMode::Absolute => (diff.abs() as f32 * self.amplification).min(255.0) as u8,
                DiffMode::Signed => ((diff + 128) as f32).max(0.0).min(255.0) as u8,
                DiffMode::Heatmap => (diff.abs() as f32 * self.amplification).min(255.0) as u8,
            }
        }
    }

    let viz = DiffVisualizer {
        mode: DiffMode::Absolute,
        amplification: 2.0,
    };

    assert_eq!(viz.visualize_pixel(10), 20);
}

#[test]
fn test_frame_alignment() {
    struct FrameAlignment {
        ref_frame: usize,
        test_frame: usize,
        offset: i32,
    }

    impl FrameAlignment {
        fn align(&mut self, ref_frame: usize, test_frame: usize) {
            self.ref_frame = ref_frame;
            self.test_frame = test_frame;
            self.offset = test_frame as i32 - ref_frame as i32;
        }

        fn is_aligned(&self) -> bool {
            self.offset == 0
        }
    }

    let mut alignment = FrameAlignment {
        ref_frame: 0,
        test_frame: 0,
        offset: 0,
    };

    alignment.align(10, 10);
    assert!(alignment.is_aligned());
    alignment.align(10, 12);
    assert!(!alignment.is_aligned());
}

#[test]
fn test_diff_export() {
    struct DiffExport {
        format: String,
        include_stats: bool,
    }

    impl DiffExport {
        fn to_csv(&self, diffs: &[u8]) -> String {
            let mut csv = String::from("index,diff\n");
            for (i, &diff) in diffs.iter().enumerate() {
                csv.push_str(&format!("{},{}\n", i, diff));
            }
            csv
        }
    }

    let export = DiffExport {
        format: "csv".to_string(),
        include_stats: true,
    };

    let csv = export.to_csv(&[5, 10, 15]);
    assert!(csv.contains("0,5"));
}

#[test]
fn test_threshold_detection() {
    struct ThresholdDetector {
        threshold: u8,
    }

    impl ThresholdDetector {
        fn is_significant(&self, diff: u8) -> bool {
            diff > self.threshold
        }

        fn count_significant(&self, diffs: &[u8]) -> usize {
            diffs.iter().filter(|&&d| self.is_significant(d)).count()
        }
    }

    let detector = ThresholdDetector { threshold: 10 };

    assert!(!detector.is_significant(5));
    assert!(detector.is_significant(15));
    assert_eq!(detector.count_significant(&[5, 15, 20, 8]), 2);
}

#[test]
fn test_side_by_side_view() {
    struct SideBySideView {
        show_reference: bool,
        show_test: bool,
        show_diff: bool,
    }

    impl SideBySideView {
        fn view_count(&self) -> usize {
            let mut count = 0;
            if self.show_reference {
                count += 1;
            }
            if self.show_test {
                count += 1;
            }
            if self.show_diff {
                count += 1;
            }
            count
        }
    }

    let view = SideBySideView {
        show_reference: true,
        show_test: true,
        show_diff: true,
    };

    assert_eq!(view.view_count(), 3);
}

#[test]
fn test_color_diff() {
    struct ColorDiff {
        y_diff: u8,
        u_diff: u8,
        v_diff: u8,
    }

    impl ColorDiff {
        fn total_diff(&self) -> u32 {
            self.y_diff as u32 + self.u_diff as u32 + self.v_diff as u32
        }
    }

    let diff = ColorDiff {
        y_diff: 10,
        u_diff: 5,
        v_diff: 3,
    };

    assert_eq!(diff.total_diff(), 18);
}

#[test]
fn test_diff_histogram() {
    struct DiffHistogram {
        bins: Vec<usize>,
        bin_count: usize,
    }

    impl DiffHistogram {
        fn new(bin_count: usize) -> Self {
            Self {
                bins: vec![0; bin_count],
                bin_count,
            }
        }

        fn add_diff(&mut self, diff: u8) {
            let bin_index = (diff as usize).min(self.bin_count - 1);
            self.bins[bin_index] += 1;
        }

        fn total_count(&self) -> usize {
            self.bins.iter().sum()
        }
    }

    let mut histogram = DiffHistogram::new(256);
    histogram.add_diff(10);
    histogram.add_diff(20);
    histogram.add_diff(10);

    assert_eq!(histogram.bins[10], 2);
    assert_eq!(histogram.total_count(), 3);
}
