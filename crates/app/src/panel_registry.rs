//! Panel Registry - Extracted from BitvueApp
//!
//! Groups all UI panels into a single container for better organization.
//!
//! # Lazy Loading Strategy
//!
//! - **Always-initialized (visible in default layout):**
//!   - stream_tree, filmstrip, bitrate_graph, selection_info
//!   - Total: ~50KB initialization cost
//!
//! - **Lazy-initialized (only if user opens advanced panels):**
//!   - hex_view, bit_view, yuv_viewer, syntax_detail, quality_metrics, block_info
//!   - Total deferred: ~100KB
//!
//! This provides modest startup improvements and better resource management.

use crate::lazy_workspace::LazyWorkspace;
use ui::panels::{
    BitViewPanel, BitrateGraphPanel, BlockInfoPanel, FilmstripPanel, HexViewPanel,
    QualityMetricsPanel, SelectionInfoPanel, StreamTreePanel, SyntaxDetailPanel, YuvViewerPanel,
};

/// Registry containing all UI panels
///
/// This struct groups the 10 UI panels that were previously individual fields
/// in BitvueApp, improving organization and reducing field count.
///
/// # Lazy vs Eager Initialization
///
/// - **Eager (always visible)**: stream_tree, filmstrip, bitrate_graph, selection_info
/// - **Lazy (advanced panels)**: hex_view, bit_view, yuv_viewer, syntax_detail, quality_metrics, block_info
pub struct PanelRegistry {
    // ===== Always-initialized panels (visible in default layout) =====
    /// R2: Stream tree panel - OBU/unit hierarchy
    pub stream_tree: StreamTreePanel,
    /// R3: Filmstrip panel - thumbnail strip (VQAnalyzer parity)
    pub filmstrip: FilmstripPanel,
    /// R3: Bitrate graph panel - frame size visualization
    pub bitrate_graph: BitrateGraphPanel,
    /// R4: Selection info panel - current selection details
    pub selection_info: SelectionInfoPanel,

    // ===== Lazy-initialized panels (loaded on first open) =====
    /// R4: Hex view panel - raw bytes (LAZY)
    pub hex_view: LazyWorkspace<HexViewPanel>,
    /// R4: Bit view panel - binary representation (LAZY)
    pub bit_view: LazyWorkspace<BitViewPanel>,
    /// R4: YUV viewer panel - YUV plane visualization (LAZY)
    pub yuv_viewer: LazyWorkspace<YuvViewerPanel>,
    /// R4: Syntax detail panel - parsed syntax elements (LAZY)
    pub syntax_detail: LazyWorkspace<SyntaxDetailPanel>,
    /// R3: Quality metrics panel - PSNR/SSIM/VMAF (LAZY)
    pub quality_metrics: LazyWorkspace<QualityMetricsPanel>,
    /// R4: Block info panel - block-level data (LAZY)
    pub block_info: LazyWorkspace<BlockInfoPanel>,
}

impl PanelRegistry {
    /// Create a new panel registry with default panels
    ///
    /// # Performance
    ///
    /// - Eager panels (4): Initialized immediately (~50KB)
    /// - Lazy panels (6): Deferred until first access (~100KB saved)
    pub fn new() -> Self {
        tracing::debug!("Creating PanelRegistry (4 eager, 6 lazy)");
        Self {
            // Eager initialization (visible in default layout)
            stream_tree: StreamTreePanel::new(),
            filmstrip: FilmstripPanel::new(),
            bitrate_graph: BitrateGraphPanel::new(),
            selection_info: SelectionInfoPanel::new(),

            // Lazy initialization (zero cost until first access)
            hex_view: LazyWorkspace::new(),
            bit_view: LazyWorkspace::new(),
            yuv_viewer: LazyWorkspace::new(),
            syntax_detail: LazyWorkspace::new(),
            quality_metrics: LazyWorkspace::new(),
            block_info: LazyWorkspace::new(),
        }
    }

    // ===== Lazy panel accessor methods =====
    // These ensure panels are initialized before use.

    /// Get mutable reference to hex view panel (initializes on first call)
    pub fn hex_view_mut(&mut self) -> &mut HexViewPanel {
        self.hex_view.get_or_init(|| HexViewPanel::new())
    }

    /// Get mutable reference to bit view panel (initializes on first call)
    pub fn bit_view_mut(&mut self) -> &mut BitViewPanel {
        self.bit_view.get_or_init(|| BitViewPanel::new())
    }

    /// Get mutable reference to YUV viewer panel (initializes on first call)
    pub fn yuv_viewer_mut(&mut self) -> &mut YuvViewerPanel {
        self.yuv_viewer.get_or_init(|| YuvViewerPanel::new())
    }

    /// Get mutable reference to syntax detail panel (initializes on first call)
    pub fn syntax_detail_mut(&mut self) -> &mut SyntaxDetailPanel {
        self.syntax_detail.get_or_init(|| SyntaxDetailPanel::new())
    }

    /// Get mutable reference to quality metrics panel (initializes on first call)
    pub fn quality_metrics_mut(&mut self) -> &mut QualityMetricsPanel {
        self.quality_metrics
            .get_or_init(|| QualityMetricsPanel::new())
    }

    /// Get mutable reference to block info panel (initializes on first call)
    pub fn block_info_mut(&mut self) -> &mut BlockInfoPanel {
        self.block_info.get_or_init(|| BlockInfoPanel::new())
    }

    // ===== Debugging and inspection methods =====

    /// Returns the number of panels that have been initialized
    #[allow(dead_code)]
    pub fn initialized_count(&self) -> usize {
        let mut count = 4; // Always-initialized: stream_tree, filmstrip, bitrate_graph, selection_info
        if self.hex_view.is_initialized() {
            count += 1;
        }
        if self.bit_view.is_initialized() {
            count += 1;
        }
        if self.yuv_viewer.is_initialized() {
            count += 1;
        }
        if self.syntax_detail.is_initialized() {
            count += 1;
        }
        if self.quality_metrics.is_initialized() {
            count += 1;
        }
        if self.block_info.is_initialized() {
            count += 1;
        }
        count
    }
}

impl Default for PanelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        // Verify all panels can be constructed
        let _registry = PanelRegistry::new();
    }

    #[test]
    fn test_default() {
        let _registry: PanelRegistry = Default::default();
    }

    #[test]
    fn test_lazy_panels_not_initialized_on_new() {
        let registry = PanelRegistry::new();

        // Only 4 eager panels should be initialized
        assert_eq!(registry.initialized_count(), 4);

        // Lazy panels should NOT be initialized yet
        assert!(!registry.hex_view.is_initialized());
        assert!(!registry.bit_view.is_initialized());
        assert!(!registry.yuv_viewer.is_initialized());
        assert!(!registry.syntax_detail.is_initialized());
        assert!(!registry.quality_metrics.is_initialized());
        assert!(!registry.block_info.is_initialized());
    }

    #[test]
    fn test_hex_view_mut_initializes_on_first_call() {
        let mut registry = PanelRegistry::new();
        assert!(!registry.hex_view.is_initialized());
        assert_eq!(registry.initialized_count(), 4);

        let _panel = registry.hex_view_mut();
        assert!(registry.hex_view.is_initialized());
        assert_eq!(registry.initialized_count(), 5);
    }

    #[test]
    fn test_all_lazy_accessors() {
        let mut registry = PanelRegistry::new();

        // Access each lazy panel
        let _hex_view = registry.hex_view_mut();
        let _bit_view = registry.bit_view_mut();
        let _yuv_viewer = registry.yuv_viewer_mut();
        let _syntax_detail = registry.syntax_detail_mut();
        let _quality_metrics = registry.quality_metrics_mut();
        let _block_info = registry.block_info_mut();

        // All panels should now be initialized
        assert_eq!(registry.initialized_count(), 10);
    }
}
