//! Comparison Strategy - Strategy pattern for quality metrics comparison
//!
//! This module provides a strategy pattern for comparing video quality
//! using different metrics (PSNR, SSIM, VMAF, etc.).
//!
//! # Example
//!
//! ```ignore
//! use bitvue_core::{ComparisonStrategy, PsnrComparisonStrategy};
//!
//! let strategy = PsnrComparisonStrategy::new();
//! let result = strategy.compare_frames(&reference, &distorted)?;
//! println!("PSNR: {:.2} dB", result.score);
//! ```

use std::fmt;

/// Comparison type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComparisonType {
    /// Peak Signal-to-Noise Ratio
    PSNR,
    /// Structural Similarity Index
    SSIM,
    /// Video Multimethod Assessment Fusion
    VMAF,
    /// Bitwise comparison
    Bitwise,
    /// Mean Squared Error
    MSE,
    /// Mean Absolute Error
    MAE,
    /// Custom comparison
    Custom,
}

impl fmt::Display for ComparisonType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComparisonType::PSNR => write!(f, "PSNR"),
            ComparisonType::SSIM => write!(f, "SSIM"),
            ComparisonType::VMAF => write!(f, "VMAF"),
            ComparisonType::Bitwise => write!(f, "Bitwise"),
            ComparisonType::MSE => write!(f, "MSE"),
            ComparisonType::MAE => write!(f, "MAE"),
            ComparisonType::Custom => write!(f, "Custom"),
        }
    }
}

/// Comparison result containing score and metadata
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Comparison type used
    pub comparison_type: ComparisonType,
    /// Quality score (higher is better for most metrics)
    pub score: f64,
    /// Additional metadata
    pub metadata: ComparisonMetadata,
}

impl ComparisonResult {
    /// Create a new comparison result
    pub fn new(comparison_type: ComparisonType, score: f64) -> Self {
        Self {
            comparison_type,
            score,
            metadata: ComparisonMetadata::default(),
        }
    }

    /// Check if result is excellent quality
    pub fn is_excellent(&self) -> bool {
        match self.comparison_type {
            ComparisonType::PSNR => self.score >= 40.0,
            ComparisonType::SSIM => self.score >= 0.95,
            ComparisonType::VMAF => self.score >= 90.0,
            _ => false,
        }
    }

    /// Check if result is good quality
    pub fn is_good(&self) -> bool {
        match self.comparison_type {
            ComparisonType::PSNR => self.score >= 30.0,
            ComparisonType::SSIM => self.score >= 0.85,
            ComparisonType::VMAF => self.score >= 75.0,
            _ => false,
        }
    }

    /// Get quality tier description
    pub fn quality_tier(&self) -> &'static str {
        match self.comparison_type {
            ComparisonType::PSNR => {
                if self.score >= 40.0 { "Excellent" }
                else if self.score >= 30.0 { "Good" }
                else if self.score >= 20.0 { "Fair" }
                else { "Poor" }
            }
            ComparisonType::SSIM => {
                if self.score >= 0.95 { "Excellent" }
                else if self.score >= 0.85 { "Good" }
                else if self.score >= 0.70 { "Fair" }
                else { "Poor" }
            }
            ComparisonType::VMAF => {
                if self.score >= 90.0 { "Excellent" }
                else if self.score >= 75.0 { "Good" }
                else if self.score >= 60.0 { "Fair" }
                else { "Poor" }
            }
            _ => "Unknown",
        }
    }
}

/// Metadata from comparison operation
#[derive(Debug, Clone, Default)]
pub struct ComparisonMetadata {
    /// Frame dimensions
    pub width: usize,
    pub height: usize,
    /// Number of pixels compared
    pub pixel_count: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: Option<u64>,
    /// Per-plane scores (for YUV comparison)
    pub per_plane_scores: Option<(f64, f64, f64)>, // (Y, U, V)
    /// Minimum value in reference
    pub min_value: Option<u8>,
    /// Maximum value in reference
    pub max_value: Option<u8>,
    /// Mean value in reference
    pub mean_value: Option<f64>,
}

/// Comparison error types
#[derive(Debug, Clone)]
pub enum ComparisonError {
    /// Size mismatch between images
    SizeMismatch { expected: usize, got: usize },
    /// Invalid data format
    InvalidData { message: String },
    /// Unsupported comparison type
    UnsupportedType { comparison_type: ComparisonType },
    /// Calculation error
    CalculationError { message: String },
}

impl fmt::Display for ComparisonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeMismatch { expected, got } => {
                write!(f, "Size mismatch: expected {}, got {}", expected, got)
            }
            Self::InvalidData { message } => write!(f, "Invalid data: {}", message),
            Self::UnsupportedType { comparison_type } => {
                write!(f, "Unsupported comparison type: {}", comparison_type)
            }
            Self::CalculationError { message } => write!(f, "Calculation error: {}", message),
        }
    }
}

impl std::error::Error for ComparisonError {}

/// Result type for comparison operations
pub type ComparisonResultType<T> = Result<T, ComparisonError>;

/// Trait for comparison strategies
///
/// This trait defines the interface for different comparison metrics.
/// Each metric type implements this trait to provide its comparison logic.
pub trait ComparisonStrategy: Send + Sync {
    /// Get the comparison type
    fn comparison_type(&self) -> ComparisonType;

    /// Get the strategy name
    fn name(&self) -> &str;

    /// Compare two frames and return quality score
    ///
    /// Returns a comparison result with score and metadata.
    fn compare_frames(
        &self,
        reference: &[u8],
        distorted: &[u8],
        width: usize,
        height: usize,
    ) -> ComparisonResultType<ComparisonResult>;

    /// Check if frames are compatible for comparison
    fn check_compatibility(
        &self,
        reference: &[u8],
        distorted: &[u8],
        width: usize,
        height: usize,
    ) -> ComparisonResultType<()> {
        let expected_size = width * height;
        if reference.len() != expected_size {
            return Err(ComparisonError::SizeMismatch {
                expected: expected_size,
                got: reference.len(),
            });
        }
        if distorted.len() != expected_size {
            return Err(ComparisonError::SizeMismatch {
                expected: expected_size,
                got: distorted.len(),
            });
        }
        Ok(())
    }

    /// Compare multiple frame pairs
    ///
    /// Convenience method for batch comparison.
    fn compare_frame_pairs(
        &self,
        pairs: &[(&[u8], &[u8])],
        width: usize,
        height: usize,
    ) -> ComparisonResultType<Vec<ComparisonResult>> {
        pairs
            .iter()
            .map(|(reference, distorted)| self.compare_frames(reference, distorted, width, height))
            .collect()
    }

    /// Get threshold for excellent quality
    fn excellent_threshold(&self) -> f64;

    /// Get threshold for good quality
    fn good_threshold(&self) -> f64;

    /// Get threshold for fair quality
    fn fair_threshold(&self) -> f64;

    /// Get unit for score display
    fn score_unit(&self) -> &str;
}

// =============================================================================
// PSNR Comparison Strategy
// =============================================================================

/// PSNR (Peak Signal-to-Noise Ratio) comparison strategy
#[derive(Debug, Clone)]
pub struct PsnrComparisonStrategy;

impl PsnrComparisonStrategy {
    /// Create a new PSNR comparison strategy
    pub fn new() -> Self {
        Self
    }
}

impl Default for PsnrComparisonStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ComparisonStrategy for PsnrComparisonStrategy {
    fn comparison_type(&self) -> ComparisonType {
        ComparisonType::PSNR
    }

    fn name(&self) -> &str {
        "PSNR"
    }

    fn compare_frames(
        &self,
        reference: &[u8],
        distorted: &[u8],
        width: usize,
        height: usize,
    ) -> ComparisonResultType<ComparisonResult> {
        self.check_compatibility(reference, distorted, width, height)?;

        let size = width * height;

        // Calculate Mean Squared Error (MSE)
        let mut mse: f64 = 0.0;
        for i in 0..size {
            let diff = reference[i] as f64 - distorted[i] as f64;
            mse += diff * diff;
        }
        mse /= size as f64;

        // Handle identical images
        let score = if mse == 0.0 {
            f64::INFINITY
        } else {
            // Calculate PSNR
            let max_value = 255.0;
            10.0 * (max_value * max_value / mse).log10()
        };

        let mut metadata = ComparisonMetadata {
            width,
            height,
            pixel_count: size,
            ..Default::default()
        };

        // Calculate statistics
        let min_value = *reference.iter().min().unwrap();
        let max_value = *reference.iter().max().unwrap();
        let mean_value = reference.iter().map(|&v| v as f64).sum::<f64>() / size as f64;

        metadata.min_value = Some(min_value);
        metadata.max_value = Some(max_value);
        metadata.mean_value = Some(mean_value);

        Ok(ComparisonResult {
            comparison_type: ComparisonType::PSNR,
            score,
            metadata,
        })
    }

    fn excellent_threshold(&self) -> f64 {
        40.0
    }

    fn good_threshold(&self) -> f64 {
        30.0
    }

    fn fair_threshold(&self) -> f64 {
        20.0
    }

    fn score_unit(&self) -> &str {
        "dB"
    }
}

// =============================================================================
// SSIM Comparison Strategy
// =============================================================================

/// SSIM (Structural Similarity Index) comparison strategy
#[derive(Debug, Clone)]
pub struct SsimComparisonStrategy;

impl SsimComparisonStrategy {
    /// Create a new SSIM comparison strategy
    pub fn new() -> Self {
        Self
    }
}

impl Default for SsimComparisonStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ComparisonStrategy for SsimComparisonStrategy {
    fn comparison_type(&self) -> ComparisonType {
        ComparisonType::SSIM
    }

    fn name(&self) -> &str {
        "SSIM"
    }

    fn compare_frames(
        &self,
        reference: &[u8],
        distorted: &[u8],
        width: usize,
        height: usize,
    ) -> ComparisonResultType<ComparisonResult> {
        self.check_compatibility(reference, distorted, width, height)?;

        // SSIM constants
        let k1 = 0.01;
        let k2 = 0.03;
        let l = 255.0;
        let c1 = (k1 * l) * (k1 * l);
        let c2 = (k2 * l) * (k2 * l);

        // Use sliding window approach (8x8 blocks)
        let window_size = 8;
        let mut ssim_sum = 0.0;
        let mut count = 0;

        for y in (0..height).step_by(window_size) {
            for x in (0..width).step_by(window_size) {
                let win_width = window_size.min(width - x);
                let win_height = window_size.min(height - y);
                let win_size = win_width * win_height;

                if win_size == 0 {
                    continue;
                }

                // Calculate statistics for this window
                let mut sum_x = 0.0;
                let mut sum_y = 0.0;
                let mut sum_xx = 0.0;
                let mut sum_yy = 0.0;
                let mut sum_xy = 0.0;

                for dy in 0..win_height {
                    for dx in 0..win_width {
                        let idx = (y + dy) * width + (x + dx);
                        let px = reference[idx] as f64;
                        let py = distorted[idx] as f64;

                        sum_x += px;
                        sum_y += py;
                        sum_xx += px * px;
                        sum_yy += py * py;
                        sum_xy += px * py;
                    }
                }

                // Calculate means
                let n = win_size as f64;
                let mean_x = sum_x / n;
                let mean_y = sum_y / n;

                // Calculate variances and covariance
                let var_x = (sum_xx / n) - (mean_x * mean_x);
                let var_y = (sum_yy / n) - (mean_y * mean_y);
                let cov_xy = (sum_xy / n) - (mean_x * mean_y);

                // Calculate SSIM for this window
                let numerator = (2.0 * mean_x * mean_y + c1) * (2.0 * cov_xy + c2);
                let denominator = (mean_x * mean_x + mean_y * mean_y + c1) * (var_x + var_y + c2);

                let window_ssim = numerator / denominator;
                ssim_sum += window_ssim;
                count += 1;
            }
        }

        let score = ssim_sum / count as f64;

        Ok(ComparisonResult {
            comparison_type: ComparisonType::SSIM,
            score,
            metadata: ComparisonMetadata {
                width,
                height,
                pixel_count: width * height,
                ..Default::default()
            },
        })
    }

    fn excellent_threshold(&self) -> f64 {
        0.95
    }

    fn good_threshold(&self) -> f64 {
        0.85
    }

    fn fair_threshold(&self) -> f64 {
        0.70
    }

    fn score_unit(&self) -> &str {
        ""
    }
}

// =============================================================================
// MSE Comparison Strategy
// =============================================================================

/// MSE (Mean Squared Error) comparison strategy
#[derive(Debug, Clone)]
pub struct MseComparisonStrategy;

impl MseComparisonStrategy {
    /// Create a new MSE comparison strategy
    pub fn new() -> Self {
        Self
    }
}

impl Default for MseComparisonStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ComparisonStrategy for MseComparisonStrategy {
    fn comparison_type(&self) -> ComparisonType {
        ComparisonType::MSE
    }

    fn name(&self) -> &str {
        "MSE"
    }

    fn compare_frames(
        &self,
        reference: &[u8],
        distorted: &[u8],
        width: usize,
        height: usize,
    ) -> ComparisonResultType<ComparisonResult> {
        self.check_compatibility(reference, distorted, width, height)?;

        let size = width * height;

        // Calculate Mean Squared Error
        let mut mse: f64 = 0.0;
        for i in 0..size {
            let diff = reference[i] as f64 - distorted[i] as f64;
            mse += diff * diff;
        }
        mse /= size as f64;

        Ok(ComparisonResult {
            comparison_type: ComparisonType::MSE,
            score: mse,
            metadata: ComparisonMetadata {
                width,
                height,
                pixel_count: size,
                ..Default::default()
            },
        })
    }

    fn excellent_threshold(&self) -> f64 {
        0.0 // Lower is better for MSE
    }

    fn good_threshold(&self) -> f64 {
        50.0
    }

    fn fair_threshold(&self) -> f64 {
        200.0
    }

    fn score_unit(&self) -> &str {
        ""
    }
}

// =============================================================================
// MAE Comparison Strategy
// =============================================================================

/// MAE (Mean Absolute Error) comparison strategy
#[derive(Debug, Clone)]
pub struct MaeComparisonStrategy;

impl MaeComparisonStrategy {
    /// Create a new MAE comparison strategy
    pub fn new() -> Self {
        Self
    }
}

impl Default for MaeComparisonStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ComparisonStrategy for MaeComparisonStrategy {
    fn comparison_type(&self) -> ComparisonType {
        ComparisonType::MAE
    }

    fn name(&self) -> &str {
        "MAE"
    }

    fn compare_frames(
        &self,
        reference: &[u8],
        distorted: &[u8],
        width: usize,
        height: usize,
    ) -> ComparisonResultType<ComparisonResult> {
        self.check_compatibility(reference, distorted, width, height)?;

        let size = width * height;

        // Calculate Mean Absolute Error
        let mut mae: f64 = 0.0;
        for i in 0..size {
            mae += (reference[i] as f64 - distorted[i] as f64).abs();
        }
        mae /= size as f64;

        Ok(ComparisonResult {
            comparison_type: ComparisonType::MAE,
            score: mae,
            metadata: ComparisonMetadata {
                width,
                height,
                pixel_count: size,
                ..Default::default()
            },
        })
    }

    fn excellent_threshold(&self) -> f64 {
        0.0 // Lower is better for MAE
    }

    fn good_threshold(&self) -> f64 {
        5.0
    }

    fn fair_threshold(&self) -> f64 {
        15.0
    }

    fn score_unit(&self) -> &str {
        ""
    }
}

// =============================================================================
// Bitwise Comparison Strategy
// =============================================================================

/// Bitwise comparison strategy
#[derive(Debug, Clone)]
pub struct BitwiseComparisonStrategy {
    /// Threshold for considering bits different (0-255)
    threshold: u8,
}

impl BitwiseComparisonStrategy {
    /// Create a new bitwise comparison strategy
    pub fn new() -> Self {
        Self { threshold: 0 }
    }

    /// Create with custom threshold
    pub fn with_threshold(threshold: u8) -> Self {
        Self { threshold }
    }
}

impl Default for BitwiseComparisonStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ComparisonStrategy for BitwiseComparisonStrategy {
    fn comparison_type(&self) -> ComparisonType {
        ComparisonType::Bitwise
    }

    fn name(&self) -> &str {
        "Bitwise"
    }

    fn compare_frames(
        &self,
        reference: &[u8],
        distorted: &[u8],
        width: usize,
        height: usize,
    ) -> ComparisonResultType<ComparisonResult> {
        self.check_compatibility(reference, distorted, width, height)?;

        let size = width * height;
        let mut different_bits = 0;

        for i in 0..size {
            let xor = reference[i] ^ distorted[i];
            if xor > self.threshold {
                // Count different bits
                different_bits += xor.count_ones() as u64;
            }
        }

        let total_bits = size as u64 * 8;
        let similarity_ratio = if total_bits > 0 {
            1.0 - (different_bits as f64 / total_bits as f64)
        } else {
            1.0
        };

        Ok(ComparisonResult {
            comparison_type: ComparisonType::Bitwise,
            score: similarity_ratio * 100.0, // Convert to percentage
            metadata: ComparisonMetadata {
                width,
                height,
                pixel_count: size,
                ..Default::default()
            },
        })
    }

    fn excellent_threshold(&self) -> f64 {
        99.0 // Higher is better for Bitwise
    }

    fn good_threshold(&self) -> f64 {
        95.0
    }

    fn fair_threshold(&self) -> f64 {
        90.0
    }

    fn score_unit(&self) -> &str {
        "%"
    }
}

// =============================================================================
// Comparison Strategy Factory
// =============================================================================

/// Factory for creating comparison strategies
pub struct ComparisonStrategyFactory;

impl ComparisonStrategyFactory {
    /// Create a strategy for the given comparison type
    pub fn create(comparison_type: ComparisonType) -> ComparisonResultType<Box<dyn ComparisonStrategy>> {
        match comparison_type {
            ComparisonType::PSNR => Ok(Box::new(PsnrComparisonStrategy::new())),
            ComparisonType::SSIM => Ok(Box::new(SsimComparisonStrategy::new())),
            ComparisonType::MSE => Ok(Box::new(MseComparisonStrategy::new())),
            ComparisonType::MAE => Ok(Box::new(MaeComparisonStrategy::new())),
            ComparisonType::Bitwise => Ok(Box::new(BitwiseComparisonStrategy::new())),
            _ => Err(ComparisonError::UnsupportedType { comparison_type }),
        }
    }

    /// Get list of supported comparison types
    pub fn supported_types() -> Vec<ComparisonType> {
        vec![
            ComparisonType::PSNR,
            ComparisonType::SSIM,
            ComparisonType::MSE,
            ComparisonType::MAE,
            ComparisonType::Bitwise,
        ]
    }

    /// Check if a comparison type is supported
    pub fn is_supported(comparison_type: ComparisonType) -> bool {
        !matches!(comparison_type, ComparisonType::VMAF | ComparisonType::Custom)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison_type_display() {
        assert_eq!(ComparisonType::PSNR.to_string(), "PSNR");
        assert_eq!(ComparisonType::SSIM.to_string(), "SSIM");
        assert_eq!(ComparisonType::MSE.to_string(), "MSE");
        assert_eq!(ComparisonType::Bitwise.to_string(), "Bitwise");
    }

    #[test]
    fn test_psnr_identical_frames() {
        let strategy = PsnrComparisonStrategy::new();
        let reference = vec![128u8; 100];
        let distorted = vec![128u8; 100];

        let result = strategy.compare_frames(&reference, &distorted, 10, 10).unwrap();
        assert_eq!(result.comparison_type, ComparisonType::PSNR);
        assert!(result.score.is_infinite());
        assert!(result.is_excellent());
    }

    #[test]
    fn test_psnr_different_frames() {
        let strategy = PsnrComparisonStrategy::new();
        let reference = vec![128u8; 100];
        let mut distorted = vec![128u8; 100];
        distorted[50] = 130; // Small difference

        let result = strategy.compare_frames(&reference, &distorted, 10, 10).unwrap();
        assert!(result.score.is_finite());
        assert!(result.score > 40.0); // Should be high PSNR
        assert!(result.is_excellent());
    }

    #[test]
    fn test_ssim_identical_frames() {
        let strategy = SsimComparisonStrategy::new();
        let reference = vec![128u8; 64]; // 8x8
        let distorted = vec![128u8; 64];

        let result = strategy.compare_frames(&reference, &distorted, 8, 8).unwrap();
        assert_eq!(result.comparison_type, ComparisonType::SSIM);
        assert!((result.score - 1.0).abs() < 0.01); // Should be ~1.0
        assert!(result.is_excellent());
    }

    #[test]
    fn test_ssim_different_frames() {
        let strategy = SsimComparisonStrategy::new();
        let reference = vec![128u8; 64]; // 8x8
        let mut distorted = vec![128u8; 64];
        distorted[30] = 130; // Small difference

        let result = strategy.compare_frames(&reference, &distorted, 8, 8).unwrap();
        assert!(result.score > 0.95); // Should be high SSIM
        assert!(result.score < 1.0);
        assert!(result.is_excellent());
    }

    #[test]
    fn test_mse_identical_frames() {
        let strategy = MseComparisonStrategy::new();
        let reference = vec![128u8; 100];
        let distorted = vec![128u8; 100];

        let result = strategy.compare_frames(&reference, &distorted, 10, 10).unwrap();
        assert_eq!(result.comparison_type, ComparisonType::MSE);
        assert_eq!(result.score, 0.0);
        assert!(result.is_excellent()); // MSE of 0 is excellent
    }

    #[test]
    fn test_mae_identical_frames() {
        let strategy = MaeComparisonStrategy::new();
        let reference = vec![128u8; 100];
        let distorted = vec![128u8; 100];

        let result = strategy.compare_frames(&reference, &distorted, 10, 10).unwrap();
        assert_eq!(result.comparison_type, ComparisonType::MAE);
        assert_eq!(result.score, 0.0);
        assert!(result.is_excellent()); // MAE of 0 is excellent
    }

    #[test]
    fn test_bitwise_identical_frames() {
        let strategy = BitwiseComparisonStrategy::new();
        let reference = vec![128u8; 100];
        let distorted = vec![128u8; 100];

        let result = strategy.compare_frames(&reference, &distorted, 10, 10).unwrap();
        assert_eq!(result.comparison_type, ComparisonType::Bitwise);
        assert_eq!(result.score, 100.0); // 100% identical
        assert!(result.is_excellent());
    }

    #[test]
    fn test_bitwise_different_frames() {
        let strategy = BitwiseComparisonStrategy::new();
        let reference = vec![128u8; 100];
        let mut distorted = vec![128u8; 100];
        distorted[0] = 129; // One bit different

        let result = strategy.compare_frames(&reference, &distorted, 10, 10).unwrap();
        assert!(result.score < 100.0);
        assert!(result.score > 99.0); // Should still be very close
        assert!(result.is_excellent());
    }

    #[test]
    fn test_quality_tiers_psnr() {
        let reference = vec![128u8; 100];
        let mut distorted = vec![128u8; 100];
        let strategy = PsnrComparisonStrategy::new();

        // Excellent (identical)
        let result = strategy.compare_frames(&reference, &distorted, 10, 10).unwrap();
        assert_eq!(result.quality_tier(), "Excellent");

        // Good (small difference)
        distorted[50] = 130;
        let result = strategy.compare_frames(&reference, &distorted, 10, 10).unwrap();
        assert_eq!(result.quality_tier(), "Excellent");
    }

    #[test]
    fn test_size_mismatch() {
        let strategy = PsnrComparisonStrategy::new();
        let reference = vec![128u8; 100];
        let distorted = vec![128u8; 50]; // Different size

        let result = strategy.compare_frames(&reference, &distorted, 10, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_factory() {
        let psnr = ComparisonStrategyFactory::create(ComparisonType::PSNR).unwrap();
        assert_eq!(psnr.comparison_type(), ComparisonType::PSNR);

        let ssim = ComparisonStrategyFactory::create(ComparisonType::SSIM).unwrap();
        assert_eq!(ssim.comparison_type(), ComparisonType::SSIM);

        let vmaf = ComparisonStrategyFactory::create(ComparisonType::VMAF);
        assert!(vmaf.is_err());
    }

    #[test]
    fn test_supported_types() {
        let types = ComparisonStrategyFactory::supported_types();
        assert_eq!(types.len(), 5);
        assert!(types.contains(&ComparisonType::PSNR));
        assert!(types.contains(&ComparisonType::SSIM));
    }

    #[test]
    fn test_thresholds() {
        let strategy = PsnrComparisonStrategy::new();
        assert_eq!(strategy.excellent_threshold(), 40.0);
        assert_eq!(strategy.good_threshold(), 30.0);
        assert_eq!(strategy.fair_threshold(), 20.0);
        assert_eq!(strategy.score_unit(), "dB");
    }

    #[test]
    fn test_frame_pairs() {
        let strategy = SsimComparisonStrategy::new();
        let reference1 = vec![128u8; 64];
        let distorted1 = vec![128u8; 64];
        let reference2 = vec![200u8; 64];
        let distorted2 = vec![200u8; 64];

        let pairs = vec![
            (reference1.as_slice(), distorted1.as_slice()),
            (reference2.as_slice(), distorted2.as_slice()),
        ];

        let results = strategy.compare_frame_pairs(&pairs, 8, 8).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].score > 0.99); // Should be very high
        assert!(results[1].score > 0.99);
    }
}
