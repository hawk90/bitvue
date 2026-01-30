//! Workspace Strategy Pattern
//!
//! Provides trait-based abstractions for codec workspaces, allowing:
//! - Common workspace interface via `CodecWorkspace` trait
//! - Pluggable view rendering via `ViewRenderer` strategy
//! - Codec-specific color schemes via `ColorScheme` strategy
//! - Custom partition rendering via `PartitionRenderer` strategy
//!
//! # Strategy Pattern Benefits
//!
//! - **Open/Closed**: New codecs can be added without modifying existing code
//! - **Single Responsibility**: Each strategy handles one aspect
//! - **Runtime Flexibility**: Rendering strategies can be swapped at runtime
//! - **Testability**: Each strategy can be tested in isolation

use egui::{Color32, Ui};

// =============================================================================
// COLOR SCHEME STRATEGY
// =============================================================================

/// Color scheme strategy for codec-specific colors
///
/// Each codec has its own color palette for visualizing:
/// - Block boundaries
/// - Prediction modes (intra/inter)
/// - Reference frames
/// - Slice/frame types
pub trait ColorScheme: Send + Sync {
    /// Get color for block boundary
    fn block_boundary(&self) -> Color32;

    /// Get color for superblock/macroblock boundary
    fn superblock_boundary(&self) -> Color32;

    /// Get color for intra prediction
    fn intra_prediction(&self) -> Color32;

    /// Get color for inter prediction
    fn inter_prediction(&self) -> Color32;

    /// Get color for skip mode
    fn skip_mode(&self) -> Color32;

    /// Get color for I-frame/slice
    fn iframe(&self) -> Color32;

    /// Get color for P-frame/slice
    fn pframe(&self) -> Color32;

    /// Get color for B-frame/slice
    fn bframe(&self) -> Color32;

    /// Get color for transform boundary (if applicable)
    fn transform_boundary(&self) -> Option<Color32> {
        None
    }

    /// Get color for deblocking boundary (if applicable)
    fn deblocking_boundary(&self) -> Option<Color32> {
        None
    }
}

// =============================================================================
// VIEW RENDERER STRATEGY
// =============================================================================

/// Context for view rendering
#[derive(Clone)]
pub struct ViewContext {
    /// Frame dimensions
    pub frame_width: u32,
    pub frame_height: u32,

    /// Current zoom level
    pub zoom: f32,

    /// Show grid flag
    pub show_grid: bool,

    /// Show block boundaries flag
    pub show_block_boundaries: bool,

    /// Show superblock boundaries flag
    pub show_superblock_boundaries: bool,

    /// Show references flag
    pub show_references: bool,

    /// Selected block index (if any)
    pub selected_block: Option<usize>,
}

impl Default for ViewContext {
    fn default() -> Self {
        Self {
            frame_width: 1920,
            frame_height: 1080,
            zoom: 1.0,
            show_grid: true,
            show_block_boundaries: true,
            show_superblock_boundaries: true,
            show_references: true,
            selected_block: None,
        }
    }
}

/// View renderer strategy
///
/// Different rendering strategies for different view modes:
/// - Overview: Summary statistics
/// - Partitions: Block partition visualization
/// - References: Reference frame visualization
/// - Codec-specific: CDEF, Deblocking, Transform, etc.
pub trait ViewRenderer: Send + Sync {
    /// Get the label for this view
    fn label(&self) -> &str;

    /// Render the view
    ///
    /// Returns `Some(command)` if a UI command should be emitted
    fn render(&self, ui: &mut Ui, ctx: &ViewContext) -> Option<ViewRenderResult>;
}

/// Result from view rendering
#[derive(Debug, Clone)]
pub enum ViewRenderResult {
    /// No action
    None,

    /// Block selected
    BlockSelected {
        index: usize,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },

    /// View mode changed
    ViewChanged(String),
}

// =============================================================================
// PARTITION RENDERER STRATEGY
// =============================================================================

/// Partition data for rendering
#[derive(Debug, Clone)]
pub struct PartitionData {
    /// X position in pixels
    pub x: u32,

    /// Y position in pixels
    pub y: u32,

    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,

    /// Partition type label
    pub partition_type: String,

    /// Reference frame label (if inter)
    pub reference_frame: Option<String>,

    /// Prediction mode label
    pub prediction_mode: Option<String>,

    /// Color to use for this partition
    pub color: Color32,

    /// Is selected?
    pub is_selected: bool,
}

/// Partition renderer strategy
///
/// Different codecs have different partition structures:
/// - AV1: 128x128 or 64x64 superblocks, recursive partitioning
/// - AVC: 16x16 macroblocks, sub-MB partitions
/// - HEVC: CTUs up to 64x64, CU partitioning
/// - VVC: CTUs up to 128x128, QT-MTT partitioning
/// - MPEG2: 16x16 macroblocks (simpler)
pub trait PartitionRenderer: Send + Sync {
    /// Render partition grid
    ///
    /// Returns vector of partition data for rendering
    fn render_partitions(
        &self,
        frame_width: u32,
        frame_height: u32,
        zoom: f32,
    ) -> Vec<PartitionData>;

    /// Get maximum partition depth
    fn max_depth(&self) -> usize;

    /// Get base block size (superblock/CTU/MB size)
    fn base_block_size(&self) -> u32;
}

// =============================================================================
// CODEC WORKSPACE TRAIT
// =============================================================================

/// Codec workspace trait
///
/// Provides a common interface for all codec-specific workspaces.
/// This allows the workspace registry to work with any codec workspace
/// through dynamic dispatch or generics.
pub trait CodecWorkspace: Send + Sync {
    /// Get the codec name (e.g., "AV1", "AVC", "HEVC")
    fn codec_name(&self) -> &str;

    /// Get the current view mode label
    fn current_view_label(&self) -> &str;

    /// Set view mode by index (for keyboard shortcuts)
    fn set_view_by_index(&mut self, index: bool);

    /// Show the workspace UI
    ///
    /// This is the main entry point for rendering the workspace.
    /// Different workspaces may accept different context parameters.
    fn show(&mut self, ui: &mut Ui);

    /// Get color scheme
    fn color_scheme(&self) -> &dyn ColorScheme;

    /// Get available view renderers
    fn view_renderers(&self) -> Vec<Box<dyn ViewRenderer>>;

    /// Get partition renderer (if applicable)
    fn partition_renderer(&self) -> Option<&dyn PartitionRenderer> {
        None
    }
}

// =============================================================================
// STRATEGY BUILDER
// =============================================================================

/// Builder for creating codec workspace strategies
///
/// This helps construct the strategy pattern components for each codec.
pub struct StrategyBuilder {
    codec_name: String,
    color_scheme: Option<Box<dyn ColorScheme>>,
    view_renderers: Vec<Box<dyn ViewRenderer>>,
    partition_renderer: Option<Box<dyn PartitionRenderer>>,
}

impl StrategyBuilder {
    /// Create a new strategy builder
    pub fn new(codec_name: impl Into<String>) -> Self {
        Self {
            codec_name: codec_name.into(),
            color_scheme: None,
            view_renderers: Vec::new(),
            partition_renderer: None,
        }
    }

    /// Set the color scheme
    pub fn with_color_scheme(mut self, scheme: Box<dyn ColorScheme>) -> Self {
        self.color_scheme = Some(scheme);
        self
    }

    /// Add a view renderer
    pub fn with_view_renderer(mut self, renderer: Box<dyn ViewRenderer>) -> Self {
        self.view_renderers.push(renderer);
        self
    }

    /// Set the partition renderer
    pub fn with_partition_renderer(mut self, renderer: Box<dyn PartitionRenderer>) -> Self {
        self.partition_renderer = Some(renderer);
        self
    }

    /// Build the strategy set
    pub fn build(self) -> Result<StrategySet, String> {
        let color_scheme = self
            .color_scheme
            .ok_or_else(|| "Color scheme is required".to_string())?;

        if self.view_renderers.is_empty() {
            return Err("At least one view renderer is required".to_string());
        }

        Ok(StrategySet {
            codec_name: self.codec_name,
            color_scheme,
            view_renderers: self.view_renderers,
            partition_renderer: self.partition_renderer,
        })
    }
}

/// Complete strategy set for a codec workspace
pub struct StrategySet {
    pub codec_name: String,
    pub color_scheme: Box<dyn ColorScheme>,
    pub view_renderers: Vec<Box<dyn ViewRenderer>>,
    pub partition_renderer: Option<Box<dyn PartitionRenderer>>,
}

// =============================================================================
// DEFAULT COLOR SCHEMES
// =============================================================================

/// Default AV1 color scheme
pub struct Av1ColorScheme;

impl ColorScheme for Av1ColorScheme {
    fn block_boundary(&self) -> Color32 {
        Color32::from_rgb(100, 149, 237) // Cornflower blue
    }

    fn superblock_boundary(&self) -> Color32 {
        Color32::from_rgb(255, 128, 0) // Orange
    }

    fn intra_prediction(&self) -> Color32 {
        Color32::from_rgb(147, 112, 219) // Medium purple
    }

    fn inter_prediction(&self) -> Color32 {
        Color32::from_rgb(50, 205, 50) // Lime green
    }

    fn skip_mode(&self) -> Color32 {
        Color32::from_rgb(200, 200, 200) // Light gray
    }

    fn iframe(&self) -> Color32 {
        Color32::from_rgb(255, 0, 0) // Red
    }

    fn pframe(&self) -> Color32 {
        Color32::from_rgb(0, 255, 0) // Green
    }

    fn bframe(&self) -> Color32 {
        Color32::from_rgb(0, 0, 255) // Blue
    }
}

/// Default AVC (H.264) color scheme
pub struct AvcColorScheme;

impl ColorScheme for AvcColorScheme {
    fn block_boundary(&self) -> Color32 {
        Color32::from_rgb(100, 149, 237) // Cornflower blue
    }

    fn superblock_boundary(&self) -> Color32 {
        Color32::from_rgb(255, 128, 0) // Orange (MB boundary)
    }

    fn intra_prediction(&self) -> Color32 {
        Color32::from_rgb(147, 112, 219) // Medium purple
    }

    fn inter_prediction(&self) -> Color32 {
        Color32::from_rgb(30, 144, 255) // Dodger blue
    }

    fn skip_mode(&self) -> Color32 {
        Color32::from_rgb(50, 205, 50) // Lime green
    }

    fn iframe(&self) -> Color32 {
        Color32::from_rgb(255, 0, 0) // Red
    }

    fn pframe(&self) -> Color32 {
        Color32::from_rgb(0, 255, 0) // Green
    }

    fn bframe(&self) -> Color32 {
        Color32::from_rgb(0, 0, 255) // Blue
    }

    fn transform_boundary(&self) -> Option<Color32> {
        Some(Color32::from_rgb(144, 238, 144)) // Light green
    }

    fn deblocking_boundary(&self) -> Option<Color32> {
        Some(Color32::from_rgb(255, 100, 100)) // Light red
    }
}

/// Default HEVC color scheme
pub struct HevcColorScheme;

impl ColorScheme for HevcColorScheme {
    fn block_boundary(&self) -> Color32 {
        Color32::from_rgb(100, 149, 237) // Cornflower blue
    }

    fn superblock_boundary(&self) -> Color32 {
        Color32::from_rgb(255, 128, 0) // Orange (CTU boundary)
    }

    fn intra_prediction(&self) -> Color32 {
        Color32::from_rgb(147, 112, 219) // Medium purple
    }

    fn inter_prediction(&self) -> Color32 {
        Color32::from_rgb(30, 144, 255) // Dodger blue
    }

    fn skip_mode(&self) -> Color32 {
        Color32::from_rgb(50, 205, 50) // Lime green
    }

    fn iframe(&self) -> Color32 {
        Color32::from_rgb(255, 0, 0) // Red
    }

    fn pframe(&self) -> Color32 {
        Color32::from_rgb(0, 255, 0) // Green
    }

    fn bframe(&self) -> Color32 {
        Color32::from_rgb(0, 0, 255) // Blue
    }

    fn transform_boundary(&self) -> Option<Color32> {
        Some(Color32::from_rgb(144, 238, 144)) // Light green
    }
}

/// Default VVC color scheme
pub struct VvcColorScheme;

impl ColorScheme for VvcColorScheme {
    fn block_boundary(&self) -> Color32 {
        Color32::from_rgb(100, 149, 237) // Cornflower blue
    }

    fn superblock_boundary(&self) -> Color32 {
        Color32::from_rgb(255, 128, 0) // Orange (CTU boundary)
    }

    fn intra_prediction(&self) -> Color32 {
        Color32::from_rgb(147, 112, 219) // Medium purple
    }

    fn inter_prediction(&self) -> Color32 {
        Color32::from_rgb(30, 144, 255) // Dodger blue
    }

    fn skip_mode(&self) -> Color32 {
        Color32::from_rgb(50, 205, 50) // Lime green
    }

    fn iframe(&self) -> Color32 {
        Color32::from_rgb(255, 0, 0) // Red
    }

    fn pframe(&self) -> Color32 {
        Color32::from_rgb(0, 255, 0) // Green
    }

    fn bframe(&self) -> Color32 {
        Color32::from_rgb(0, 0, 255) // Blue
    }

    fn transform_boundary(&self) -> Option<Color32> {
        Some(Color32::from_rgb(144, 238, 144)) // Light green
    }
}

/// Default MPEG2 color scheme
pub struct Mpeg2ColorScheme;

impl ColorScheme for Mpeg2ColorScheme {
    fn block_boundary(&self) -> Color32 {
        Color32::from_rgb(100, 149, 237) // Cornflower blue
    }

    fn superblock_boundary(&self) -> Color32 {
        Color32::from_rgb(255, 128, 0) // Orange (MB boundary)
    }

    fn intra_prediction(&self) -> Color32 {
        Color32::from_rgb(147, 112, 219) // Medium purple
    }

    fn inter_prediction(&self) -> Color32 {
        Color32::from_rgb(30, 144, 255) // Dodger blue
    }

    fn skip_mode(&self) -> Color32 {
        Color32::from_rgb(50, 205, 50) // Lime green
    }

    fn iframe(&self) -> Color32 {
        Color32::from_rgb(255, 0, 0) // Red
    }

    fn pframe(&self) -> Color32 {
        Color32::from_rgb(0, 255, 0) // Green
    }

    fn bframe(&self) -> Color32 {
        Color32::from_rgb(0, 0, 255) // Blue
    }
}

// =============================================================================
// GENERIC WORKSPACE IMPLEMENTATION
// =============================================================================

/// Generic codec workspace implementation using strategies
///
/// This provides a common workspace implementation that can be
/// configured with different strategies for each codec.
pub struct GenericCodecWorkspace {
    /// Codec name
    pub codec_name: String,

    /// Strategy set
    strategies: StrategySet,

    /// Current view index
    current_view: usize,

    /// View context
    view_context: ViewContext,

    /// Show grid flag
    pub show_grid: bool,

    /// Show block boundaries flag
    pub show_block_boundaries: bool,

    /// Show superblock boundaries flag
    pub show_superblock_boundaries: bool,

    /// Show references flag
    pub show_references: bool,
}

impl GenericCodecWorkspace {
    /// Create a new generic codec workspace
    pub fn new(strategies: StrategySet) -> Self {
        let codec_name = strategies.codec_name.clone();
        Self {
            codec_name,
            strategies,
            current_view: 0,
            view_context: ViewContext::default(),
            show_grid: true,
            show_block_boundaries: true,
            show_superblock_boundaries: true,
            show_references: true,
        }
    }

    /// Get current view renderer
    pub fn current_renderer(&self) -> Option<&dyn ViewRenderer> {
        self.strategies.view_renderers.get(self.current_view).map(|r| r.as_ref())
    }

    /// Set view by index
    pub fn set_view_by_index(&mut self, index: usize) {
        if index < self.strategies.view_renderers.len() {
            self.current_view = index;
        }
    }

    /// Get color scheme
    pub fn color_scheme(&self) -> &dyn ColorScheme {
        self.strategies.color_scheme.as_ref()
    }

    /// Get partition renderer
    pub fn partition_renderer(&self) -> Option<&dyn PartitionRenderer> {
        self.strategies.partition_renderer.as_ref().map(|r| r.as_ref())
    }

    /// Show the workspace UI
    pub fn show(&mut self, ui: &mut Ui) {
        // Update view context from flags
        self.view_context.show_grid = self.show_grid;
        self.view_context.show_block_boundaries = self.show_block_boundaries;
        self.view_context.show_superblock_boundaries = self.show_superblock_boundaries;
        self.view_context.show_references = self.show_references;

        // Header: View selector
        ui.horizontal(|ui| {
            ui.label(format!("{} Workspace", self.codec_name));

            ui.separator();

            for (idx, renderer) in self.strategies.view_renderers.iter().enumerate() {
                if idx > 0 {
                    ui.separator();
                }

                let is_selected = idx == self.current_view;
                if ui.selectable_label(is_selected, renderer.label()).clicked() {
                    self.current_view = idx;
                }
            }
        });

        ui.separator();

        // Render current view
        if let Some(renderer) = self.current_renderer() {
            let _result = renderer.render(ui, &self.view_context);
        }
    }

    /// Show controls toolbar
    pub fn show_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Display:");
            ui.checkbox(&mut self.show_grid, "Grid");
            ui.checkbox(&mut self.show_block_boundaries, "Blocks");
            ui.checkbox(&mut self.show_superblock_boundaries, "Superblocks");
            ui.checkbox(&mut self.show_references, "References");
        });
    }

    /// Get current view label
    pub fn current_view_label(&self) -> &str {
        self.current_renderer()
            .map(|r| r.label())
            .unwrap_or("Unknown")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_builder_valid() {
        let builder = StrategyBuilder::new("TEST")
            .with_color_scheme(Box::<Av1ColorScheme>::default())
            .with_view_renderer(Box::new(NullViewRenderer::new("Overview")))
            .with_partition_renderer(Box::new(NullPartitionRenderer::new()));

        let result = builder.build();
        assert!(result.is_ok());
        let strategies = result.unwrap();
        assert_eq!(strategies.codec_name, "TEST");
    }

    #[test]
    fn test_strategy_builder_missing_color_scheme() {
        let builder = StrategyBuilder::new("TEST")
            .with_view_renderer(Box::new(NullViewRenderer::new("Overview")));

        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_strategy_builder_no_views() {
        let builder = StrategyBuilder::new("TEST")
            .with_color_scheme(Box::<Av1ColorScheme>::default());

        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_generic_workspace() {
        let strategies = StrategyBuilder::new("TEST")
            .with_color_scheme(Box::<Av1ColorScheme>::default())
            .with_view_renderer(Box::new(NullViewRenderer::new("Overview")))
            .with_view_renderer(Box::new(NullViewRenderer::new("Partitions")))
            .build()
            .unwrap();

        let mut workspace = GenericCodecWorkspace::new(strategies);

        assert_eq!(workspace.codec_name, "TEST");
        assert_eq!(workspace.current_view, 0);
        assert_eq!(workspace.current_view_label(), "Overview");

        workspace.set_view_by_index(1);
        assert_eq!(workspace.current_view_label(), "Partitions");

        // Invalid index should be ignored
        workspace.set_view_by_index(10);
        assert_eq!(workspace.current_view, 1); // Unchanged
    }

    // Null implementations for testing

    struct NullViewRenderer {
        label: String,
    }

    impl NullViewRenderer {
        fn new(label: &str) -> Self {
            Self {
                label: label.to_string(),
            }
        }
    }

    impl ViewRenderer for NullViewRenderer {
        fn label(&self) -> &str {
            &self.label
        }

        fn render(&self, _ui: &mut Ui, _ctx: &ViewContext) -> Option<ViewRenderResult> {
            None
        }
    }

    struct NullPartitionRenderer;

    impl NullPartitionRenderer {
        fn new() -> Self {
            Self
        }
    }

    impl PartitionRenderer for NullPartitionRenderer {
        fn render_partitions(
            &self,
            _frame_width: u32,
            _frame_height: u32,
            _zoom: f32,
        ) -> Vec<PartitionData> {
            Vec::new()
        }

        fn max_depth(&self) -> usize {
            0
        }

        fn base_block_size(&self) -> u32 {
            16
        }
    }
}
