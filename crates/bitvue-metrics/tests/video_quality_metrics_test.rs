//! Tests for Video Quality Metrics (PSNR, SSIM, VMAF)

#[test]
fn test_psnr_range() {
    // Test PSNR value ranges
    fn is_valid_psnr(psnr: f64) -> bool {
        psnr.is_infinite() || (psnr >= 0.0 && psnr <= 100.0)
    }

    assert!(is_valid_psnr(f64::INFINITY)); // Identical images
    assert!(is_valid_psnr(45.0)); // High quality
    assert!(is_valid_psnr(25.0)); // Low quality
    assert!(!is_valid_psnr(-5.0)); // Invalid
}

#[test]
fn test_psnr_quality_levels() {
    // Test PSNR quality interpretation
    #[derive(Debug, PartialEq)]
    enum QualityLevel {
        Excellent,
        Good,
        Fair,
        Poor,
    }

    fn classify_psnr(psnr: f64) -> QualityLevel {
        if psnr >= 40.0 {
            QualityLevel::Excellent
        } else if psnr >= 35.0 {
            QualityLevel::Good
        } else if psnr >= 30.0 {
            QualityLevel::Fair
        } else {
            QualityLevel::Poor
        }
    }

    assert_eq!(classify_psnr(45.0), QualityLevel::Excellent);
    assert_eq!(classify_psnr(37.0), QualityLevel::Good);
    assert_eq!(classify_psnr(32.0), QualityLevel::Fair);
    assert_eq!(classify_psnr(25.0), QualityLevel::Poor);
}

#[test]
fn test_mse_calculation() {
    // Test Mean Squared Error calculation
    fn calculate_mse(reference: &[u8], distorted: &[u8]) -> f64 {
        let mut sum = 0.0;
        for (r, d) in reference.iter().zip(distorted.iter()) {
            let diff = *r as f64 - *d as f64;
            sum += diff * diff;
        }
        sum / reference.len() as f64
    }

    let ref_data = vec![128u8; 100];
    let dist_data = vec![130u8; 100];

    let mse = calculate_mse(&ref_data, &dist_data);
    assert_eq!(mse, 4.0); // (2^2) = 4
}

#[test]
fn test_ssim_range() {
    // Test SSIM value ranges
    fn is_valid_ssim(ssim: f64) -> bool {
        ssim >= -1.0 && ssim <= 1.0
    }

    assert!(is_valid_ssim(1.0)); // Perfect similarity
    assert!(is_valid_ssim(0.95)); // High similarity
    assert!(is_valid_ssim(0.5)); // Medium similarity
    assert!(is_valid_ssim(0.0)); // Low similarity
    assert!(!is_valid_ssim(1.5)); // Invalid
}

#[test]
fn test_ssim_components() {
    // Test SSIM formula components
    struct SsimComponents {
        luminance: f64,
        contrast: f64,
        structure: f64,
    }

    impl SsimComponents {
        fn compute_ssim(&self) -> f64 {
            self.luminance * self.contrast * self.structure
        }
    }

    let components = SsimComponents {
        luminance: 0.99,
        contrast: 0.98,
        structure: 0.97,
    };

    let ssim = components.compute_ssim();
    assert!(ssim > 0.94 && ssim < 0.95);
}

#[test]
fn test_vmaf_score_range() {
    // Test VMAF score ranges
    fn is_valid_vmaf(vmaf: f64) -> bool {
        vmaf >= 0.0 && vmaf <= 100.0
    }

    assert!(is_valid_vmaf(95.0)); // Excellent
    assert!(is_valid_vmaf(75.0)); // Good
    assert!(is_valid_vmaf(50.0)); // Fair
    assert!(is_valid_vmaf(20.0)); // Poor
    assert!(!is_valid_vmaf(105.0)); // Invalid
}

#[test]
fn test_vmaf_quality_interpretation() {
    // Test VMAF quality levels
    #[derive(Debug, PartialEq)]
    enum VmafQuality {
        Excellent,
        Good,
        Fair,
        Poor,
    }

    fn classify_vmaf(score: f64) -> VmafQuality {
        if score >= 90.0 {
            VmafQuality::Excellent
        } else if score >= 75.0 {
            VmafQuality::Good
        } else if score >= 60.0 {
            VmafQuality::Fair
        } else {
            VmafQuality::Poor
        }
    }

    assert_eq!(classify_vmaf(95.0), VmafQuality::Excellent);
    assert_eq!(classify_vmaf(80.0), VmafQuality::Good);
}

#[test]
fn test_yuv_plane_metrics() {
    // Test YUV plane metric calculation
    struct YuvMetrics {
        y_psnr: f64,
        u_psnr: f64,
        v_psnr: f64,
    }

    impl YuvMetrics {
        fn weighted_average(&self) -> f64 {
            // Y plane weighted more heavily (6:1:1 ratio)
            (6.0 * self.y_psnr + self.u_psnr + self.v_psnr) / 8.0
        }
    }

    let metrics = YuvMetrics {
        y_psnr: 45.0,
        u_psnr: 40.0,
        v_psnr: 40.0,
    };

    let avg = metrics.weighted_average();
    assert!(avg > 43.0 && avg < 44.0); // (6*45 + 40 + 40) / 8 = 43.75
}

#[test]
fn test_chroma_subsampling_metrics() {
    // Test metrics for different chroma subsampling
    #[derive(Debug, PartialEq)]
    enum ChromaFormat {
        Yuv420,
        Yuv422,
        Yuv444,
    }

    fn chroma_plane_size(width: usize, height: usize, format: ChromaFormat) -> (usize, usize) {
        match format {
            ChromaFormat::Yuv420 => (width / 2, height / 2),
            ChromaFormat::Yuv422 => (width / 2, height),
            ChromaFormat::Yuv444 => (width, height),
        }
    }

    let (cw, ch) = chroma_plane_size(1920, 1080, ChromaFormat::Yuv420);
    assert_eq!((cw, ch), (960, 540));
}

#[test]
fn test_batch_metrics_aggregation() {
    // Test batch metrics aggregation
    struct BatchMetrics {
        frame_scores: Vec<f64>,
    }

    impl BatchMetrics {
        fn mean(&self) -> f64 {
            self.frame_scores.iter().sum::<f64>() / self.frame_scores.len() as f64
        }

        fn min(&self) -> f64 {
            self.frame_scores
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min)
        }

        fn max(&self) -> f64 {
            self.frame_scores
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max)
        }
    }

    let metrics = BatchMetrics {
        frame_scores: vec![45.0, 42.0, 48.0, 46.0, 44.0],
    };

    assert_eq!(metrics.mean(), 45.0);
    assert_eq!(metrics.min(), 42.0);
    assert_eq!(metrics.max(), 48.0);
}

#[test]
fn test_parallel_processing_partitioning() {
    // Test frame partitioning for parallel processing
    fn partition_frames(total_frames: usize, num_threads: usize) -> Vec<(usize, usize)> {
        let frames_per_thread = (total_frames + num_threads - 1) / num_threads;
        (0..num_threads)
            .map(|i| {
                let start = i * frames_per_thread;
                let end = ((i + 1) * frames_per_thread).min(total_frames);
                (start, end)
            })
            .filter(|(start, end)| start < end)
            .collect()
    }

    let partitions = partition_frames(100, 4);
    assert_eq!(partitions.len(), 4);
    assert_eq!(partitions[0], (0, 25));
    assert_eq!(partitions[3], (75, 100));
}

#[test]
fn test_simd_acceleration_detection() {
    // Test SIMD acceleration detection
    #[derive(Debug, PartialEq)]
    enum SimdLevel {
        None,
        Sse2,
        Avx,
        Avx2,
        Neon,
    }

    fn detect_simd() -> SimdLevel {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                return SimdLevel::Avx2;
            } else if is_x86_feature_detected!("avx") {
                return SimdLevel::Avx;
            } else if is_x86_feature_detected!("sse2") {
                return SimdLevel::Sse2;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            return SimdLevel::Neon;
        }

        SimdLevel::None
    }

    let simd = detect_simd();
    assert_ne!(simd, SimdLevel::None); // Should have at least SSE2 on x86_64
}

#[test]
fn test_metric_caching() {
    // Test metric result caching
    struct MetricsCache {
        cache: std::collections::HashMap<(usize, usize), f64>,
    }

    impl MetricsCache {
        fn new() -> Self {
            Self {
                cache: std::collections::HashMap::new(),
            }
        }

        fn get_or_compute(
            &mut self,
            ref_idx: usize,
            dist_idx: usize,
            compute_fn: impl FnOnce() -> f64,
        ) -> f64 {
            *self
                .cache
                .entry((ref_idx, dist_idx))
                .or_insert_with(compute_fn)
        }
    }

    let mut cache = MetricsCache::new();
    let score1 = cache.get_or_compute(0, 0, || 45.0);
    let score2 = cache.get_or_compute(0, 0, || 99.0); // Should return cached value

    assert_eq!(score1, 45.0);
    assert_eq!(score2, 45.0); // Not 99.0, because cached
}

#[test]
fn test_bit_depth_scaling() {
    // Test PSNR calculation for different bit depths
    fn max_value_for_bit_depth(bit_depth: u8) -> f64 {
        ((1u32 << bit_depth) - 1) as f64
    }

    assert_eq!(max_value_for_bit_depth(8), 255.0);
    assert_eq!(max_value_for_bit_depth(10), 1023.0);
    assert_eq!(max_value_for_bit_depth(12), 4095.0);
}

#[test]
fn test_roi_metrics() {
    // Test Region of Interest metrics
    struct RoiMetrics {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        psnr: f64,
    }

    impl RoiMetrics {
        fn contains_point(&self, px: usize, py: usize) -> bool {
            px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
        }
    }

    let roi = RoiMetrics {
        x: 100,
        y: 100,
        width: 200,
        height: 200,
        psnr: 45.0,
    };

    assert!(roi.contains_point(150, 150));
    assert!(!roi.contains_point(50, 50));
}
