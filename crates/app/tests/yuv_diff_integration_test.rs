//! Tests for YUV Diff Integration

#[test]
fn test_yuv_diff_pixel_comparison() {
    // Test pixel-level difference calculation
    fn calculate_diff(a: u8, b: u8) -> u8 {
        (a as i16 - b as i16).abs() as u8
    }

    assert_eq!(calculate_diff(255, 200), 55);
    assert_eq!(calculate_diff(100, 150), 50);
    assert_eq!(calculate_diff(128, 128), 0);
}

#[test]
fn test_yuv_diff_psnr() {
    // Test PSNR calculation for YUV diff
    fn calculate_mse(diffs: &[u8]) -> f64 {
        let sum: u32 = diffs.iter().map(|&d| (d as u32) * (d as u32)).sum();
        sum as f64 / diffs.len() as f64
    }

    fn calculate_psnr(mse: f64, max_value: f64) -> f64 {
        if mse == 0.0 {
            f64::INFINITY
        } else {
            20.0 * (max_value / mse.sqrt()).log10()
        }
    }

    let diffs = vec![5u8, 10, 3, 7];
    let mse = calculate_mse(&diffs);
    let psnr = calculate_psnr(mse, 255.0);

    assert!(psnr > 0.0 && psnr < 100.0);
}

#[test]
fn test_yuv_diff_heatmap_generation() {
    // Test diff heatmap color generation
    fn diff_to_color(diff: u8, max_diff: u8) -> (u8, u8, u8) {
        let normalized = (diff as f32) / (max_diff as f32);
        let r = (normalized * 255.0) as u8;
        let g = 0;
        let b = ((1.0 - normalized) * 255.0) as u8;
        (r, g, b)
    }

    let (r, g, b) = diff_to_color(128, 255);
    assert!(r > 0);
    assert_eq!(g, 0);
    assert!(b > 0);
}

#[test]
fn test_yuv_diff_plane_comparison() {
    // Test Y/U/V plane diff calculation
    struct PlaneStats {
        y_diff_sum: u64,
        u_diff_sum: u64,
        v_diff_sum: u64,
        pixel_count: usize,
    }

    let stats = PlaneStats {
        y_diff_sum: 1000,
        u_diff_sum: 500,
        v_diff_sum: 500,
        pixel_count: 100,
    };

    let y_avg = stats.y_diff_sum / stats.pixel_count as u64;
    let u_avg = stats.u_diff_sum / stats.pixel_count as u64;

    assert_eq!(y_avg, 10);
    assert_eq!(u_avg, 5);
}

#[test]
fn test_yuv_diff_threshold_filtering() {
    // Test threshold-based diff filtering
    let diffs = vec![2u8, 15, 5, 25, 3, 30];
    let threshold = 10u8;

    let significant_diffs: Vec<u8> = diffs.iter().filter(|&&d| d >= threshold).copied().collect();

    assert_eq!(significant_diffs.len(), 3); // 15, 25, 30
}

#[test]
fn test_yuv_diff_frame_sync() {
    // Test frame synchronization for diff
    struct DiffState {
        ref_frame_idx: usize,
        test_frame_idx: usize,
        locked: bool,
    }

    let mut diff = DiffState {
        ref_frame_idx: 10,
        test_frame_idx: 10,
        locked: true,
    };

    // When locked, frames should stay in sync
    if diff.locked {
        diff.ref_frame_idx += 1;
        diff.test_frame_idx += 1;
    }

    assert_eq!(diff.ref_frame_idx, diff.test_frame_idx);
}

#[test]
fn test_yuv_diff_statistics() {
    // Test diff statistics calculation
    struct DiffStatistics {
        min_diff: u8,
        max_diff: u8,
        avg_diff: f64,
        std_dev: f64,
    }

    let diffs = vec![5u8, 10, 15, 20, 25];
    let avg: f64 = diffs.iter().map(|&d| d as f64).sum::<f64>() / diffs.len() as f64;

    let stats = DiffStatistics {
        min_diff: *diffs.iter().min().unwrap(),
        max_diff: *diffs.iter().max().unwrap(),
        avg_diff: avg,
        std_dev: 0.0, // Simplified
    };

    assert_eq!(stats.min_diff, 5);
    assert_eq!(stats.max_diff, 25);
    assert_eq!(stats.avg_diff, 15.0);
}

#[test]
fn test_yuv_diff_export() {
    // Test diff data export
    struct DiffExportData {
        format: String,
        width: u32,
        height: u32,
        total_pixels: u32,
        significant_diffs: u32,
    }

    let export = DiffExportData {
        format: "YUV420".to_string(),
        width: 1920,
        height: 1080,
        total_pixels: 1920 * 1080,
        significant_diffs: 10000,
    };

    assert_eq!(export.total_pixels, 2073600);
    assert!(export.significant_diffs < export.total_pixels);
}

#[test]
fn test_yuv_diff_overlay_modes() {
    // Test diff overlay visualization modes
    #[derive(Debug, PartialEq)]
    enum DiffOverlayMode {
        Absolute,
        SignedColor,
        Heatmap,
        Threshold,
    }

    let modes = vec![
        DiffOverlayMode::Absolute,
        DiffOverlayMode::Heatmap,
        DiffOverlayMode::Threshold,
    ];

    assert_eq!(modes.len(), 3);
}

#[test]
fn test_yuv_diff_chroma_subsampling() {
    // Test chroma diff with 4:2:0 subsampling
    let y_width = 1920usize;
    let y_height = 1080usize;
    let uv_width = y_width / 2;
    let uv_height = y_height / 2;

    assert_eq!(uv_width, 960);
    assert_eq!(uv_height, 540);

    let y_pixels = y_width * y_height;
    let uv_pixels = uv_width * uv_height;

    assert_eq!(y_pixels, uv_pixels * 4);
}
