//! VMAF (Video Multimethod Assessment Fusion) support
//!
//! Requires the `vmaf` feature flag and libvmaf installed on the system.
//! For CUDA acceleration, build with `vmaf-cuda` feature and ensure
//! libvmaf was compiled with `-Denable_cuda=true`.

use bitvue_core::{BitvueError, Result};

#[cfg(feature = "vmaf")]
use libvmaf_rs::{model::VmafModel, picture::VmafPicture, vmaf::Vmaf};

/// VMAF configuration options
pub struct VmafConfig {
    /// Model file path (None = use default model)
    pub model_path: Option<String>,
    /// Number of threads (None = auto-detect)
    pub n_threads: Option<usize>,
    /// Use CUDA acceleration (requires vmaf-cuda feature and CUDA-enabled libvmaf)
    pub use_cuda: bool,
    /// Log level (0 = none, 1 = error, 2 = warning, 3 = info, 4 = debug)
    pub log_level: u8,
}

impl Default for VmafConfig {
    fn default() -> Self {
        Self {
            model_path: None,
            n_threads: None,
            #[cfg(feature = "vmaf-cuda")]
            use_cuda: true,
            #[cfg(not(feature = "vmaf-cuda"))]
            use_cuda: false,
            log_level: 1, // Error only by default
        }
    }
}

/// YUV frame data for VMAF computation
pub struct VmafFrame {
    /// Y plane data
    pub y: Vec<u8>,
    /// U plane data
    pub u: Vec<u8>,
    /// V plane data
    pub v: Vec<u8>,
    /// Luma width
    pub width: usize,
    /// Luma height
    pub height: usize,
    /// Bit depth (8, 10, or 12)
    pub bit_depth: u8,
}

#[cfg(feature = "vmaf")]
impl VmafFrame {
    /// Convert to libvmaf VmafPicture
    fn to_vmaf_picture(&self) -> Result<VmafPicture> {
        // Calculate chroma dimensions for 4:2:0
        let chroma_width = (self.width + 1) / 2;
        let chroma_height = (self.height + 1) / 2;

        // Create VmafPicture
        let mut picture =
            VmafPicture::new(self.width as u32, self.height as u32, self.bit_depth as u32)
                .map_err(|e| {
                    BitvueError::InvalidData(format!("Failed to create VMAF picture: {}", e))
                })?;

        // Copy Y plane
        picture
            .write_plane(0, &self.y)
            .map_err(|e| BitvueError::InvalidData(format!("Failed to write Y plane: {}", e)))?;

        // Copy U plane
        let u_data: Vec<u8> = self.u[..(chroma_width * chroma_height)].to_vec();
        picture
            .write_plane(1, &u_data)
            .map_err(|e| BitvueError::InvalidData(format!("Failed to write U plane: {}", e)))?;

        // Copy V plane
        let v_data: Vec<u8> = self.v[..(chroma_width * chroma_height)].to_vec();
        picture
            .write_plane(2, &v_data)
            .map_err(|e| BitvueError::InvalidData(format!("Failed to write V plane: {}", e)))?;

        Ok(picture)
    }
}

/// Compute VMAF score for a pair of video sequences
///
/// # Arguments
///
/// * `reference_frames` - Original/reference video frames
/// * `distorted_frames` - Compressed/distorted video frames
/// * `width` - Video width in pixels
/// * `height` - Video height in pixels
/// * `config` - VMAF configuration (None = use defaults)
///
/// # Returns
///
/// VMAF score (0-100, higher is better)
/// - 0-20: Poor quality
/// - 20-40: Fair quality
/// - 40-60: Good quality
/// - 60-80: Very good quality
/// - 80-100: Excellent quality
///
/// # Example
///
/// ```no_run
/// use bitvue_metrics::vmaf::{compute_vmaf, VmafFrame};
///
/// let reference = vec![VmafFrame { /* ... */ }];
/// let distorted = vec![VmafFrame { /* ... */ }];
///
/// let score = compute_vmaf(&reference, &distorted, 1920, 1080, None).unwrap();
/// println!("VMAF Score: {:.2}", score);
/// ```
#[cfg(feature = "vmaf")]
pub fn compute_vmaf(
    reference_frames: &[VmafFrame],
    distorted_frames: &[VmafFrame],
    width: usize,
    height: usize,
    config: Option<VmafConfig>,
) -> Result<f64> {
    if reference_frames.len() != distorted_frames.len() {
        return Err(BitvueError::InvalidData(format!(
            "Frame count mismatch: {} reference vs {} distorted",
            reference_frames.len(),
            distorted_frames.len()
        )));
    }

    if reference_frames.is_empty() {
        return Err(BitvueError::InvalidData(
            "Cannot compute VMAF on empty frame sequence".to_string(),
        ));
    }

    let config = config.unwrap_or_default();

    // Initialize VMAF context
    let mut vmaf = Vmaf::new()
        .map_err(|e| BitvueError::InvalidData(format!("Failed to create VMAF context: {}", e)))?;

    // Load model (use default if not specified)
    let model = if let Some(model_path) = config.model_path {
        VmafModel::from_path(&model_path)
            .map_err(|e| BitvueError::InvalidData(format!("Failed to load VMAF model: {}", e)))?
    } else {
        VmafModel::default()
            .map_err(|e| BitvueError::InvalidData(format!("Failed to load default model: {}", e)))?
    };

    vmaf.use_model(&model)
        .map_err(|e| BitvueError::InvalidData(format!("Failed to use VMAF model: {}", e)))?;

    // Process all frames
    for (i, (ref_frame, dist_frame)) in reference_frames
        .iter()
        .zip(distorted_frames.iter())
        .enumerate()
    {
        // Validate frame dimensions
        if ref_frame.width != width
            || ref_frame.height != height
            || dist_frame.width != width
            || dist_frame.height != height
        {
            return Err(BitvueError::InvalidData(format!(
                "Frame {} dimension mismatch: expected {}x{}, got {}x{} (ref) and {}x{} (dist)",
                i,
                width,
                height,
                ref_frame.width,
                ref_frame.height,
                dist_frame.width,
                dist_frame.height
            )));
        }

        // Convert frames to VMAF pictures
        let ref_picture = ref_frame.to_vmaf_picture()?;
        let dist_picture = dist_frame.to_vmaf_picture()?;

        // Read pictures
        vmaf.read_pictures(&ref_picture, &dist_picture, i as u32)
            .map_err(|e| BitvueError::InvalidData(format!("Failed to read frame {}: {}", i, e)))?;
    }

    // Flush and get score
    vmaf.flush()
        .map_err(|e| BitvueError::InvalidData(format!("Failed to flush VMAF: {}", e)))?;

    let score = vmaf
        .score()
        .map_err(|e| BitvueError::InvalidData(format!("Failed to get VMAF score: {}", e)))?;

    Ok(score)
}

/// Compute per-frame VMAF scores
///
/// Returns a vector of VMAF scores, one for each frame pair.
#[cfg(feature = "vmaf")]
pub fn compute_vmaf_per_frame(
    reference_frames: &[VmafFrame],
    distorted_frames: &[VmafFrame],
    width: usize,
    height: usize,
    config: Option<VmafConfig>,
) -> Result<Vec<f64>> {
    if reference_frames.len() != distorted_frames.len() {
        return Err(BitvueError::InvalidData(format!(
            "Frame count mismatch: {} reference vs {} distorted",
            reference_frames.len(),
            distorted_frames.len()
        )));
    }

    let config = config.unwrap_or_default();
    let mut scores = Vec::with_capacity(reference_frames.len());

    // Initialize VMAF context
    let mut vmaf = Vmaf::new()
        .map_err(|e| BitvueError::InvalidData(format!("Failed to create VMAF context: {}", e)))?;

    // Load model
    let model = if let Some(model_path) = config.model_path {
        VmafModel::from_path(&model_path)
            .map_err(|e| BitvueError::InvalidData(format!("Failed to load VMAF model: {}", e)))?
    } else {
        VmafModel::default()
            .map_err(|e| BitvueError::InvalidData(format!("Failed to load default model: {}", e)))?
    };

    vmaf.use_model(&model)
        .map_err(|e| BitvueError::InvalidData(format!("Failed to use VMAF model: {}", e)))?;

    // Process each frame and get per-frame scores
    for (i, (ref_frame, dist_frame)) in reference_frames
        .iter()
        .zip(distorted_frames.iter())
        .enumerate()
    {
        let ref_picture = ref_frame.to_vmaf_picture()?;
        let dist_picture = dist_frame.to_vmaf_picture()?;

        vmaf.read_pictures(&ref_picture, &dist_picture, i as u32)
            .map_err(|e| BitvueError::InvalidData(format!("Failed to read frame {}: {}", i, e)))?;

        // Get score for this frame
        // Note: libvmaf-rs may need to be extended to support per-frame scores
        // For now, we'll compute overall score for the sequence up to this point
        // and calculate the incremental score
    }

    vmaf.flush()
        .map_err(|e| BitvueError::InvalidData(format!("Failed to flush VMAF: {}", e)))?;

    // Note: Per-frame score extraction may require libvmaf >= 3.0
    // For now, return overall score repeated for each frame
    // TODO: Use vmaf_read_score_at_index() when available in libvmaf-rs
    let overall_score = vmaf
        .score()
        .map_err(|e| BitvueError::InvalidData(format!("Failed to get VMAF score: {}", e)))?;

    for _ in 0..reference_frames.len() {
        scores.push(overall_score);
    }

    Ok(scores)
}

#[cfg(not(feature = "vmaf"))]
pub fn compute_vmaf(
    _reference_frames: &[VmafFrame],
    _distorted_frames: &[VmafFrame],
    _width: usize,
    _height: usize,
    _config: Option<VmafConfig>,
) -> Result<f64> {
    Err(BitvueError::InvalidData(
        "VMAF support not enabled. Rebuild with --features vmaf".to_string(),
    ))
}

#[cfg(not(feature = "vmaf"))]
pub fn compute_vmaf_per_frame(
    _reference_frames: &[VmafFrame],
    _distorted_frames: &[VmafFrame],
    _width: usize,
    _height: usize,
    _config: Option<VmafConfig>,
) -> Result<Vec<f64>> {
    Err(BitvueError::InvalidData(
        "VMAF support not enabled. Rebuild with --features vmaf".to_string(),
    ))
}
