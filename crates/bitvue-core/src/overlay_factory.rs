//! Overlay Factory - Factory pattern for creating overlay renderers
//!
//! This module provides a factory interface for creating overlay renderers,
//! allowing codec-specific overlay creation and making it easier to add new overlays.
//!
//! # Example
//!
//! ```ignore
//! use bitvue_core::{OverlayFactory, Av1OverlayFactory};
//!
//! let factory = Av1OverlayFactory::new();
//! let qp_renderer = factory.create_qp_heatmap()?;
//! let mv_renderer = factory.create_mv_overlay()?;
//! ```

use std::fmt;

/// Overlay type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlayType {
    /// Grid overlay (tile/block boundaries)
    Grid,
    /// Motion vector overlay
    MotionVectors,
    /// QP (Quantization Parameter) heatmap
    QpHeatmap,
    /// Partition grid overlay
    Partition,
    /// Reference frame overlay
    ReferenceFrames,
    /// Mode labels overlay (prediction modes)
    ModeLabels,
    /// Bit allocation overlay
    BitAllocation,
    /// MV magnitude heatmap
    MvMagnitude,
    /// PU (Prediction Unit) type overlay
    PuType,
    /// Transform block overlay
    Transform,
    /// Deblocking filter overlay
    Deblocking,
    /// CDEF (Constrained Directional Enhancement Filter) overlay (AV1)
    Cdef,
    /// Super-resolution overlay (AV1)
    SuperRes,
    /// Film grain overlay (AV1)
    FilmGrain,
    /// SAO (Sample Adaptive Offset) overlay (HEVC)
    Sao,
    /// ALF (Adaptive Loop Filter) overlay (VVC)
    Alf,
}

impl fmt::Display for OverlayType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OverlayType::Grid => write!(f, "Grid"),
            OverlayType::MotionVectors => write!(f, "Motion Vectors"),
            OverlayType::QpHeatmap => write!(f, "QP Heatmap"),
            OverlayType::Partition => write!(f, "Partition"),
            OverlayType::ReferenceFrames => write!(f, "Reference Frames"),
            OverlayType::ModeLabels => write!(f, "Mode Labels"),
            OverlayType::BitAllocation => write!(f, "Bit Allocation"),
            OverlayType::MvMagnitude => write!(f, "MV Magnitude"),
            OverlayType::PuType => write!(f, "PU Type"),
            OverlayType::Transform => write!(f, "Transform"),
            OverlayType::Deblocking => write!(f, "Deblocking"),
            OverlayType::Cdef => write!(f, "CDEF"),
            OverlayType::SuperRes => write!(f, "SuperRes"),
            OverlayType::FilmGrain => write!(f, "Film Grain"),
            OverlayType::Sao => write!(f, "SAO"),
            OverlayType::Alf => write!(f, "ALF"),
        }
    }
}

/// Overlay rendering capabilities
#[derive(Debug, Clone, Default)]
pub struct OverlayCapabilities {
    /// Supported overlay types
    pub supported_overlays: Vec<OverlayType>,
    /// Maximum overlay depth for nested overlays
    pub max_depth: Option<usize>,
    /// Supports dynamic overlay switching
    pub dynamic_switching: bool,
    /// Supports overlay opacity control
    pub opacity_control: bool,
    /// Supports overlay color customization
    pub color_customization: bool,
}

impl OverlayCapabilities {
    /// Check if an overlay type is supported
    pub fn supports(&self, overlay_type: OverlayType) -> bool {
        self.supported_overlays.contains(&overlay_type)
    }

    /// Create a new capabilities builder
    pub fn builder() -> OverlayCapabilitiesBuilder {
        OverlayCapabilitiesBuilder::default()
    }
}

/// Builder for constructing OverlayCapabilities
#[derive(Debug, Clone, Default)]
pub struct OverlayCapabilitiesBuilder {
    capabilities: OverlayCapabilities,
}

impl OverlayCapabilitiesBuilder {
    /// Add a supported overlay type
    pub fn add_overlay(mut self, overlay_type: OverlayType) -> Self {
        self.capabilities.supported_overlays.push(overlay_type);
        self
    }

    /// Add multiple supported overlay types
    pub fn add_overlays(mut self, overlay_types: impl IntoIterator<Item = OverlayType>) -> Self {
        self.capabilities.supported_overlays.extend(overlay_types);
        self
    }

    /// Set maximum overlay depth
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.capabilities.max_depth = Some(depth);
        self
    }

    /// Enable dynamic switching
    pub fn dynamic_switching(mut self, enabled: bool) -> Self {
        self.capabilities.dynamic_switching = enabled;
        self
    }

    /// Enable opacity control
    pub fn opacity_control(mut self, enabled: bool) -> Self {
        self.capabilities.opacity_control = enabled;
        self
    }

    /// Enable color customization
    pub fn color_customization(mut self, enabled: bool) -> Self {
        self.capabilities.color_customization = enabled;
        self
    }

    /// Build the capabilities
    pub fn build(self) -> OverlayCapabilities {
        self.capabilities
    }
}

/// Overlay renderer configuration
#[derive(Debug, Clone)]
pub struct OverlayConfig {
    /// Overlay type
    pub overlay_type: OverlayType,
    /// Opacity (0.0 - 1.0)
    pub opacity: f32,
    /// Show overlay
    pub enabled: bool,
    /// Custom color (if applicable)
    pub color: Option<(u8, u8, u8)>,
    /// Grid size (for grid overlay)
    pub grid_size: Option<u32>,
    /// Scale factor (for heatmaps)
    pub scale: Option<f32>,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            overlay_type: OverlayType::Grid,
            opacity: 1.0,
            enabled: true,
            color: None,
            grid_size: None,
            scale: None,
        }
    }
}

impl OverlayConfig {
    /// Create a new overlay config
    pub fn new(overlay_type: OverlayType) -> Self {
        Self {
            overlay_type,
            ..Default::default()
        }
    }

    /// Set opacity
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set custom color
    pub fn with_color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.color = Some((r, g, b));
        self
    }

    /// Set grid size
    pub fn with_grid_size(mut self, size: u32) -> Self {
        self.grid_size = Some(size);
        self
    }

    /// Set scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = Some(scale);
        self
    }
}

/// Result type for overlay factory operations
pub type OverlayFactoryResult<T> = Result<T, OverlayFactoryError>;

/// Errors that can occur during overlay factory operations
#[derive(Debug, Clone)]
pub enum OverlayFactoryError {
    /// Overlay type not supported by this factory
    UnsupportedOverlayType { overlay_type: OverlayType },
    /// Missing required configuration
    MissingConfiguration { field: String },
    /// Invalid configuration value
    InvalidConfiguration { field: String, value: String },
    /// Factory initialization failed
    InitializationFailed { message: String },
}

impl fmt::Display for OverlayFactoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOverlayType { overlay_type } => {
                write!(f, "Unsupported overlay type: {}", overlay_type)
            }
            Self::MissingConfiguration { field } => {
                write!(f, "Missing required configuration: {}", field)
            }
            Self::InvalidConfiguration { field, value } => {
                write!(f, "Invalid configuration for '{}': {}", field, value)
            }
            Self::InitializationFailed { message } => {
                write!(f, "Factory initialization failed: {}", message)
            }
        }
    }
}

impl std::error::Error for OverlayFactoryError {}

/// Trait for overlay factories
///
/// This trait defines the interface for creating codec-specific overlay renderers.
/// Each codec (AV1, AVC, HEVC, VVC, etc.) implements this trait to provide
/// its own overlay renderers.
pub trait OverlayFactory: Send + Sync {
    /// Get the factory name (e.g., "AV1", "AVC", "HEVC")
    fn name(&self) -> &str;

    /// Get the capabilities of this factory
    fn capabilities(&self) -> &OverlayCapabilities;

    /// Create a grid overlay renderer
    fn create_grid(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>>;

    /// Create a motion vector overlay renderer
    fn create_mv_overlay(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>>;

    /// Create a QP heatmap overlay renderer
    fn create_qp_heatmap(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>>;

    /// Create a partition grid overlay renderer
    fn create_partition_grid(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>>;

    /// Create a reference frame overlay renderer
    fn create_reference_frames(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>>;

    /// Create a mode labels overlay renderer
    fn create_mode_labels(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>>;

    /// Create a bit allocation overlay renderer
    fn create_bit_allocation(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>>;

    /// Create an overlay renderer by type
    fn create_overlay(
        &self,
        overlay_type: OverlayType,
        config: &OverlayConfig,
    ) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        if !self.capabilities().supports(overlay_type) {
            return Err(OverlayFactoryError::UnsupportedOverlayType { overlay_type });
        }

        match overlay_type {
            OverlayType::Grid => self.create_grid(config),
            OverlayType::MotionVectors => self.create_mv_overlay(config),
            OverlayType::QpHeatmap => self.create_qp_heatmap(config),
            OverlayType::Partition => self.create_partition_grid(config),
            OverlayType::ReferenceFrames => self.create_reference_frames(config),
            OverlayType::ModeLabels => self.create_mode_labels(config),
            OverlayType::BitAllocation => self.create_bit_allocation(config),
            _ => Err(OverlayFactoryError::UnsupportedOverlayType { overlay_type }),
        }
    }

    /// Create multiple overlays at once
    fn create_overlays(
        &self,
        overlay_types: &[OverlayType],
        config: &OverlayConfig,
    ) -> OverlayFactoryResult<Vec<Box<dyn OverlayRenderer>>> {
        overlay_types
            .iter()
            .map(|&t| self.create_overlay(t, config))
            .collect()
    }
}

/// Trait for overlay renderers
///
/// This trait defines the interface for rendering overlays.
/// Each overlay type implements this trait to provide its rendering logic.
pub trait OverlayRenderer: Send + Sync {
    /// Get the overlay type
    fn overlay_type(&self) -> OverlayType;

    /// Render the overlay
    ///
    /// This method would be implemented by the UI layer to actually
    /// render the overlay to the screen. In the Rust core, this is
    /// a placeholder for the interface.
    fn render(&self);

    /// Get the overlay configuration
    fn config(&self) -> &OverlayConfig;

    /// Update the overlay configuration
    fn update_config(&mut self, config: OverlayConfig);

    /// Check if the overlay is enabled
    fn is_enabled(&self) -> bool {
        self.config().enabled
    }

    /// Enable/disable the overlay
    fn set_enabled(&mut self, enabled: bool) {
        let mut config = self.config().clone();
        config.enabled = enabled;
        self.update_config(config);
    }

    /// Get the overlay opacity
    fn opacity(&self) -> f32 {
        self.config().opacity
    }

    /// Set the overlay opacity
    fn set_opacity(&mut self, opacity: f32) {
        let mut config = self.config().clone();
        config.opacity = opacity.clamp(0.0, 1.0);
        self.update_config(config);
    }
}

/// Base overlay renderer implementation
///
/// This provides a default implementation of OverlayRenderer that can be
/// extended by specific overlay types.
#[derive(Debug, Clone)]
pub struct BaseOverlayRenderer {
    config: OverlayConfig,
}

impl BaseOverlayRenderer {
    /// Create a new base overlay renderer
    pub fn new(overlay_type: OverlayType) -> Self {
        Self {
            config: OverlayConfig::new(overlay_type),
        }
    }

    /// Create with custom config
    pub fn with_config(config: OverlayConfig) -> Self {
        Self { config }
    }
}

impl OverlayRenderer for BaseOverlayRenderer {
    fn overlay_type(&self) -> OverlayType {
        self.config.overlay_type
    }

    fn render(&self) {
        // Placeholder - actual rendering is done in UI layer
    }

    fn config(&self) -> &OverlayConfig {
        &self.config
    }

    fn update_config(&mut self, config: OverlayConfig) {
        self.config = config;
    }
}

// =============================================================================
// Codec-Specific Overlay Factories
// =============================================================================

/// AV1 overlay factory
///
/// Provides overlays specific to AV1 codec:
/// - CDEF (Constrained Directional Enhancement Filter)
/// - SuperRes (Super Resolution)
/// - FilmGrain
/// - Standard overlays (Grid, MV, QP, etc.)
pub struct Av1OverlayFactory {
    capabilities: OverlayCapabilities,
}

impl Av1OverlayFactory {
    /// Create a new AV1 overlay factory
    pub fn new() -> Self {
        let capabilities = OverlayCapabilities::builder()
            .add_overlays([
                OverlayType::Grid,
                OverlayType::MotionVectors,
                OverlayType::QpHeatmap,
                OverlayType::Partition,
                OverlayType::ReferenceFrames,
                OverlayType::ModeLabels,
                OverlayType::BitAllocation,
                OverlayType::MvMagnitude,
                OverlayType::Cdef,
                OverlayType::SuperRes,
                OverlayType::FilmGrain,
            ])
            .max_depth(4)
            .dynamic_switching(true)
            .opacity_control(true)
            .color_customization(true)
            .build();

        Self { capabilities }
    }
}

impl Default for Av1OverlayFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayFactory for Av1OverlayFactory {
    fn name(&self) -> &str {
        "AV1"
    }

    fn capabilities(&self) -> &OverlayCapabilities {
        &self.capabilities
    }

    fn create_grid(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_mv_overlay(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_qp_heatmap(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_partition_grid(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_reference_frames(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_mode_labels(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_bit_allocation(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_overlay(
        &self,
        overlay_type: OverlayType,
        config: &OverlayConfig,
    ) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        // Handle AV1-specific overlays
        match overlay_type {
            OverlayType::Cdef | OverlayType::SuperRes | OverlayType::FilmGrain => {
                Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
            }
            _ => self.create_overlay(overlay_type, config), // Use default implementation
        }
    }
}

/// AVC (H.264) overlay factory
///
/// Provides overlays specific to AVC codec:
/// - Transform blocks
/// - Deblocking filter
/// - Standard overlays
pub struct AvcOverlayFactory {
    capabilities: OverlayCapabilities,
}

impl AvcOverlayFactory {
    /// Create a new AVC overlay factory
    pub fn new() -> Self {
        let capabilities = OverlayCapabilities::builder()
            .add_overlays([
                OverlayType::Grid,
                OverlayType::MotionVectors,
                OverlayType::QpHeatmap,
                OverlayType::Partition,
                OverlayType::ReferenceFrames,
                OverlayType::ModeLabels,
                OverlayType::BitAllocation,
                OverlayType::MvMagnitude,
                OverlayType::Transform,
                OverlayType::Deblocking,
            ])
            .max_depth(4)
            .dynamic_switching(true)
            .opacity_control(true)
            .color_customization(true)
            .build();

        Self { capabilities }
    }
}

impl Default for AvcOverlayFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayFactory for AvcOverlayFactory {
    fn name(&self) -> &str {
        "AVC"
    }

    fn capabilities(&self) -> &OverlayCapabilities {
        &self.capabilities
    }

    fn create_grid(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_mv_overlay(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_qp_heatmap(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_partition_grid(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_reference_frames(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_mode_labels(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_bit_allocation(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }
}

/// HEVC (H.265) overlay factory
///
/// Provides overlays specific to HEVC codec:
/// - SAO (Sample Adaptive Offset)
/// - Standard overlays
pub struct HevcOverlayFactory {
    capabilities: OverlayCapabilities,
}

impl HevcOverlayFactory {
    /// Create a new HEVC overlay factory
    pub fn new() -> Self {
        let capabilities = OverlayCapabilities::builder()
            .add_overlays([
                OverlayType::Grid,
                OverlayType::MotionVectors,
                OverlayType::QpHeatmap,
                OverlayType::Partition,
                OverlayType::ReferenceFrames,
                OverlayType::ModeLabels,
                OverlayType::BitAllocation,
                OverlayType::MvMagnitude,
                OverlayType::Transform,
                OverlayType::Deblocking,
                OverlayType::Sao,
            ])
            .max_depth(4)
            .dynamic_switching(true)
            .opacity_control(true)
            .color_customization(true)
            .build();

        Self { capabilities }
    }
}

impl Default for HevcOverlayFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayFactory for HevcOverlayFactory {
    fn name(&self) -> &str {
        "HEVC"
    }

    fn capabilities(&self) -> &OverlayCapabilities {
        &self.capabilities
    }

    fn create_grid(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_mv_overlay(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_qp_heatmap(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_partition_grid(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_reference_frames(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_mode_labels(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_bit_allocation(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }
}

/// VVC (H.266) overlay factory
///
/// Provides overlays specific to VVC codec:
/// - ALF (Adaptive Loop Filter)
/// - Standard overlays
pub struct VvcOverlayFactory {
    capabilities: OverlayCapabilities,
}

impl VvcOverlayFactory {
    /// Create a new VVC overlay factory
    pub fn new() -> Self {
        let capabilities = OverlayCapabilities::builder()
            .add_overlays([
                OverlayType::Grid,
                OverlayType::MotionVectors,
                OverlayType::QpHeatmap,
                OverlayType::Partition,
                OverlayType::ReferenceFrames,
                OverlayType::ModeLabels,
                OverlayType::BitAllocation,
                OverlayType::MvMagnitude,
                OverlayType::Transform,
                OverlayType::Deblocking,
                OverlayType::Alf,
            ])
            .max_depth(4)
            .dynamic_switching(true)
            .opacity_control(true)
            .color_customization(true)
            .build();

        Self { capabilities }
    }
}

impl Default for VvcOverlayFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayFactory for VvcOverlayFactory {
    fn name(&self) -> &str {
        "VVC"
    }

    fn capabilities(&self) -> &OverlayCapabilities {
        &self.capabilities
    }

    fn create_grid(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_mv_overlay(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_qp_heatmap(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_partition_grid(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_reference_frames(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_mode_labels(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }

    fn create_bit_allocation(&self, config: &OverlayConfig) -> OverlayFactoryResult<Box<dyn OverlayRenderer>> {
        Ok(Box::new(BaseOverlayRenderer::with_config(config.clone())))
    }
}

// =============================================================================
// Registry for Overlay Factories
// =============================================================================

/// Registry for overlay factories
///
/// Allows registering and retrieving codec-specific overlay factories.
pub struct OverlayFactoryRegistry {
    factories: std::collections::HashMap<String, Box<dyn OverlayFactory>>,
}

impl OverlayFactoryRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        let mut registry = Self {
            factories: std::collections::HashMap::new(),
        };

        // Register default factories
        registry.register("AV1".to_string(), Box::new(Av1OverlayFactory::new()));
        registry.register("AVC".to_string(), Box::new(AvcOverlayFactory::new()));
        registry.register("HEVC".to_string(), Box::new(HevcOverlayFactory::new()));
        registry.register("VVC".to_string(), Box::new(VvcOverlayFactory::new()));

        registry
    }

    /// Register a factory
    pub fn register(&mut self, name: String, factory: Box<dyn OverlayFactory>) {
        self.factories.insert(name, factory);
    }

    /// Get a factory by name
    pub fn get(&self, name: &str) -> Option<&dyn OverlayFactory> {
        self.factories.get(name).map(|f| f.as_ref())
    }

    /// Get all registered factory names
    pub fn factory_names(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }

    /// Check if a factory is registered
    pub fn has_factory(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }
}

impl Default for OverlayFactoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_type_display() {
        assert_eq!(OverlayType::Grid.to_string(), "Grid");
        assert_eq!(OverlayType::MotionVectors.to_string(), "Motion Vectors");
        assert_eq!(OverlayType::QpHeatmap.to_string(), "QP Heatmap");
    }

    #[test]
    fn test_overlay_config_builder() {
        let config = OverlayConfig::new(OverlayType::Grid)
            .with_opacity(0.5)
            .with_enabled(false)
            .with_color(255, 0, 0)
            .with_grid_size(64);

        assert_eq!(config.overlay_type, OverlayType::Grid);
        assert_eq!(config.opacity, 0.5);
        assert_eq!(config.enabled, false);
        assert_eq!(config.color, Some((255, 0, 0)));
        assert_eq!(config.grid_size, Some(64));
    }

    #[test]
    fn test_overlay_opacity_clamping() {
        let config1 = OverlayConfig::new(OverlayType::Grid).with_opacity(1.5);
        assert_eq!(config1.opacity, 1.0);

        let config2 = OverlayConfig::new(OverlayType::Grid).with_opacity(-0.5);
        assert_eq!(config2.opacity, 0.0);
    }

    #[test]
    fn test_overlay_capabilities_builder() {
        let capabilities = OverlayCapabilities::builder()
            .add_overlay(OverlayType::Grid)
            .add_overlay(OverlayType::MotionVectors)
            .max_depth(4)
            .dynamic_switching(true)
            .opacity_control(true)
            .build();

        assert!(capabilities.supports(OverlayType::Grid));
        assert!(capabilities.supports(OverlayType::MotionVectors));
        assert!(!capabilities.supports(OverlayType::QpHeatmap));
        assert_eq!(capabilities.max_depth, Some(4));
        assert!(capabilities.dynamic_switching);
        assert!(capabilities.opacity_control);
    }

    #[test]
    fn test_base_overlay_renderer() {
        let mut renderer = BaseOverlayRenderer::new(OverlayType::Grid);

        assert_eq!(renderer.overlay_type(), OverlayType::Grid);
        assert_eq!(renderer.opacity(), 1.0);
        assert!(renderer.is_enabled());

        renderer.set_opacity(0.5);
        assert_eq!(renderer.opacity(), 0.5);

        renderer.set_enabled(false);
        assert!(!renderer.is_enabled());
    }

    #[test]
    fn test_av1_overlay_factory() {
        let factory = Av1OverlayFactory::new();

        assert_eq!(factory.name(), "AV1");
        assert!(factory.capabilities().supports(OverlayType::Grid));
        assert!(factory.capabilities().supports(OverlayType::Cdef));
        assert!(factory.capabilities().supports(OverlayType::SuperRes));
        assert!(factory.capabilities().supports(OverlayType::FilmGrain));
        assert!(!factory.capabilities().supports(OverlayType::Sao));
    }

    #[test]
    fn test_avc_overlay_factory() {
        let factory = AvcOverlayFactory::new();

        assert_eq!(factory.name(), "AVC");
        assert!(factory.capabilities().supports(OverlayType::Grid));
        assert!(factory.capabilities().supports(OverlayType::Transform));
        assert!(factory.capabilities().supports(OverlayType::Deblocking));
        assert!(!factory.capabilities().supports(OverlayType::Cdef));
    }

    #[test]
    fn test_hevc_overlay_factory() {
        let factory = HevcOverlayFactory::new();

        assert_eq!(factory.name(), "HEVC");
        assert!(factory.capabilities().supports(OverlayType::Grid));
        assert!(factory.capabilities().supports(OverlayType::Sao));
        assert!(!factory.capabilities().supports(OverlayType::Cdef));
    }

    #[test]
    fn test_vvc_overlay_factory() {
        let factory = VvcOverlayFactory::new();

        assert_eq!(factory.name(), "VVC");
        assert!(factory.capabilities().supports(OverlayType::Grid));
        assert!(factory.capabilities().supports(OverlayType::Alf));
        assert!(!factory.capabilities().supports(OverlayType::Cdef));
    }

    #[test]
    fn test_overlay_factory_registry() {
        let registry = OverlayFactoryRegistry::new();

        assert!(registry.has_factory("AV1"));
        assert!(registry.has_factory("AVC"));
        assert!(registry.has_factory("HEVC"));
        assert!(registry.has_factory("VVC"));
        assert!(!registry.has_factory("VP9"));

        let av1_factory = registry.get("AV1").unwrap();
        assert_eq!(av1_factory.name(), "AV1");
    }

    #[test]
    fn test_create_overlay_via_factory() {
        let factory = Av1OverlayFactory::new();
        let config = OverlayConfig::new(OverlayType::Grid).with_opacity(0.8);

        let renderer = factory.create_grid(&config).unwrap();
        assert_eq!(renderer.overlay_type(), OverlayType::Grid);
        assert_eq!(renderer.opacity(), 0.8);
    }

    #[test]
    fn test_create_multiple_overlays() {
        let factory = Av1OverlayFactory::new();
        let config = OverlayConfig::new(OverlayType::Grid);

        let overlay_types = vec![
            OverlayType::Grid,
            OverlayType::MotionVectors,
            OverlayType::QpHeatmap,
        ];

        let renderers = factory.create_overlays(&overlay_types, &config).unwrap();
        assert_eq!(renderers.len(), 3);
        assert_eq!(renderers[0].overlay_type(), OverlayType::Grid);
        assert_eq!(renderers[1].overlay_type(), OverlayType::MotionVectors);
        assert_eq!(renderers[2].overlay_type(), OverlayType::QpHeatmap);
    }

    #[test]
    fn test_unsupported_overlay_error() {
        let factory = Av1OverlayFactory::new();
        let config = OverlayConfig::new(OverlayType::Grid);

        // Try to create an SAO overlay (HEVC-specific) with AV1 factory
        let result = factory.create_overlay(OverlayType::Sao, &config);
        assert!(result.is_err());

        // Use if let instead of unwrap_err to avoid requiring Debug on Box<dyn OverlayRenderer>
        if let Err(OverlayFactoryError::UnsupportedOverlayType { overlay_type }) = result {
            assert_eq!(overlay_type, OverlayType::Sao);
        } else {
            panic!("Expected UnsupportedOverlayType error");
        }
    }

    #[test]
    fn test_overlay_factory_error_display() {
        let error = OverlayFactoryError::UnsupportedOverlayType {
            overlay_type: OverlayType::Grid,
        };
        assert_eq!(error.to_string(), "Unsupported overlay type: Grid");

        let error = OverlayFactoryError::MissingConfiguration {
            field: "grid_size".to_string(),
        };
        assert_eq!(error.to_string(), "Missing required configuration: grid_size");
    }
}
