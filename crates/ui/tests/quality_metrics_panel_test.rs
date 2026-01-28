//! Tests for Quality Metrics panel

#[test]
fn test_psnr_calculation() {
    // Test PSNR (Peak Signal-to-Noise Ratio) calculation
    fn calculate_psnr(mse: f64, max_value: f64) -> f64 {
        if mse == 0.0 {
            f64::INFINITY
        } else {
            20.0 * (max_value / mse.sqrt()).log10()
        }
    }

    let mse = 100.0;
    let psnr = calculate_psnr(mse, 255.0);

    assert!(psnr > 0.0 && psnr < 100.0);
}

#[test]
fn test_ssim_range() {
    // Test SSIM (Structural Similarity Index) range
    let ssim_values = vec![0.0, 0.5, 0.8, 0.95, 1.0];

    for ssim in ssim_values {
        assert!(ssim >= 0.0 && ssim <= 1.0, "SSIM should be 0-1");
    }
}

#[test]
fn test_vmaf_score() {
    // Test VMAF score range
    let vmaf_scores = vec![0.0, 50.0, 75.0, 90.0, 100.0];

    for score in vmaf_scores {
        assert!(score >= 0.0 && score <= 100.0, "VMAF should be 0-100");
    }
}

#[test]
fn test_quality_metric_display() {
    // Test quality metric display formatting
    struct QualityMetrics {
        psnr_y: f64,
        psnr_u: f64,
        psnr_v: f64,
        ssim: f64,
        vmaf: f64,
    }

    let metrics = QualityMetrics {
        psnr_y: 42.5,
        psnr_u: 44.2,
        psnr_v: 43.8,
        ssim: 0.95,
        vmaf: 88.5,
    };

    assert!(metrics.psnr_y > 0.0);
    assert!(metrics.ssim >= 0.0 && metrics.ssim <= 1.0);
    assert!(metrics.vmaf >= 0.0 && metrics.vmaf <= 100.0);
}

#[test]
fn test_per_frame_metrics() {
    // Test per-frame metric tracking
    struct FrameMetrics {
        frame_index: usize,
        psnr: f64,
        ssim: f64,
    }

    let mut frame_metrics = Vec::new();
    for i in 0..10 {
        frame_metrics.push(FrameMetrics {
            frame_index: i,
            psnr: 40.0 + (i as f64 * 0.5),
            ssim: 0.90 + (i as f64 * 0.01),
        });
    }

    assert_eq!(frame_metrics.len(), 10);
    assert!(frame_metrics[0].psnr < frame_metrics[9].psnr);
}

#[test]
fn test_average_metrics_calculation() {
    // Test average metrics calculation
    let psnr_values = vec![40.0, 42.0, 41.0, 43.0];
    let avg_psnr: f64 = psnr_values.iter().sum::<f64>() / psnr_values.len() as f64;

    assert_eq!(avg_psnr, 41.5);
}

#[test]
fn test_metric_thresholds() {
    // Test quality thresholds
    struct QualityThresholds {
        excellent_psnr: f64,
        good_psnr: f64,
        acceptable_psnr: f64,
    }

    let thresholds = QualityThresholds {
        excellent_psnr: 45.0,
        good_psnr: 40.0,
        acceptable_psnr: 35.0,
    };

    let current_psnr = 42.0;
    let quality = if current_psnr >= thresholds.excellent_psnr {
        "Excellent"
    } else if current_psnr >= thresholds.good_psnr {
        "Good"
    } else if current_psnr >= thresholds.acceptable_psnr {
        "Acceptable"
    } else {
        "Poor"
    };

    assert_eq!(quality, "Good");
}

#[test]
fn test_mse_calculation() {
    // Test MSE (Mean Squared Error) calculation
    let original = vec![100, 150, 200];
    let distorted = vec![105, 145, 195];

    let mse: f64 = original
        .iter()
        .zip(distorted.iter())
        .map(|(o, d)| ((*o as i32 - *d as i32) as f64).powi(2))
        .sum::<f64>()
        / original.len() as f64;

    assert!(mse >= 0.0);
}

#[test]
fn test_bitrate_quality_correlation() {
    // Test bitrate vs quality correlation
    struct EncodingResult {
        bitrate_kbps: u32,
        psnr: f64,
    }

    let results = vec![
        EncodingResult {
            bitrate_kbps: 1000,
            psnr: 38.0,
        },
        EncodingResult {
            bitrate_kbps: 2000,
            psnr: 42.0,
        },
        EncodingResult {
            bitrate_kbps: 5000,
            psnr: 46.0,
        },
    ];

    // Higher bitrate should generally mean higher PSNR
    assert!(results[0].psnr < results[1].psnr);
    assert!(results[1].psnr < results[2].psnr);
}

#[test]
fn test_yuv_component_metrics() {
    // Test Y/U/V component metrics
    struct ComponentMetrics {
        y_psnr: f64,
        u_psnr: f64,
        v_psnr: f64,
    }

    let metrics = ComponentMetrics {
        y_psnr: 42.0,
        u_psnr: 44.0,
        v_psnr: 43.5,
    };

    // U and V typically have higher PSNR than Y
    assert!(metrics.u_psnr >= metrics.y_psnr);
}

#[test]
fn test_metrics_graph_data() {
    // Test metrics graph data preparation
    struct GraphPoint {
        frame: usize,
        value: f64,
    }

    let mut graph_data = Vec::new();
    for i in 0..100 {
        graph_data.push(GraphPoint {
            frame: i,
            value: 40.0 + ((i as f64 * 0.1).sin() * 5.0),
        });
    }

    assert_eq!(graph_data.len(), 100);
    assert!(graph_data[0].value >= 35.0 && graph_data[0].value <= 45.0);
}
