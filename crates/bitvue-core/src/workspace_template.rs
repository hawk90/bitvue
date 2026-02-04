//! Template Method - Workspace rendering template
//!
//! This module provides a template method pattern for consistent workspace rendering,
//! with customizable steps for different workspace types.

use std::sync::Arc;

// =============================================================================
// Template Method Types
// =============================================================================

/// Render context for workspace rendering
#[derive(Debug, Clone)]
pub struct RenderContext {
    /// Width of the render area
    pub width: u32,
    /// Height of the render area
    pub height: u32,
    /// DPI scaling factor
    pub dpi_scale: f32,
    /// Current time in milliseconds
    pub time_ms: u64,
    /// Frame number
    pub frame_number: u64,
}

impl Default for RenderContext {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            dpi_scale: 1.0,
            time_ms: 0,
            frame_number: 0,
        }
    }
}

/// Render statistics for profiling
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    /// Time spent preparing (microseconds)
    pub prepare_time_us: u64,
    /// Time spent rendering (microseconds)
    pub render_time_us: u64,
    /// Time spent cleaning up (microseconds)
    pub cleanup_time_us: u64,
    /// Number of draw calls
    pub draw_calls: u32,
    /// Number of vertices rendered
    pub vertex_count: u32,
}

impl RenderStats {
    /// Get total render time
    pub fn total_time_us(&self) -> u64 {
        self.prepare_time_us + self.render_time_us + self.cleanup_time_us
    }

    /// Get total time in milliseconds
    pub fn total_time_ms(&self) -> f64 {
        self.total_time_us() as f64 / 1000.0
    }
}

/// Render result
#[derive(Debug, Clone)]
pub enum RenderResult {
    /// Render succeeded
    Success { stats: RenderStats },
    /// Render failed with error
    Failed { error: String },
    /// Render skipped (e.g., nothing to render)
    Skipped { reason: String },
}

impl RenderResult {
    /// Check if render succeeded
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Get stats if successful
    pub fn stats(&self) -> Option<&RenderStats> {
        match self {
            Self::Success { stats } => Some(stats),
            _ => None,
        }
    }
}

// =============================================================================
// Template Method Trait
// =============================================================================

/// Template method for workspace rendering
///
/// This trait defines the skeleton of the rendering algorithm, with
/// customizable steps implemented by concrete workspace types.
pub trait WorkspaceRenderer: Send + Sync {
    /// Template method: render the workspace using the standard algorithm
    fn render(&self, ctx: &RenderContext) -> RenderResult {
        // Step 1: Prepare for rendering
        let prepare_start = self.now_us();
        match self.prepare(ctx) {
            Ok(()) => {}
            Err(e) => return RenderResult::Failed { error: e },
        }
        let prepare_time = self.now_us() - prepare_start;

        // Step 2: Validate render state
        if let Err(e) = self.validate(ctx) {
            self.cleanup(ctx, Some(&e));
            return RenderResult::Failed { error: e };
        }

        // Step 3: Clear render target
        self.clear(ctx);

        // Step 4: Pre-render (setup, calculations)
        if let Err(e) = self.pre_render(ctx) {
            self.cleanup(ctx, Some(&e));
            return RenderResult::Failed { error: e };
        }

        // Step 5: Main render
        let render_start = self.now_us();
        let draw_calls_before = self.draw_call_count();
        let vertex_count_before = self.vertex_count();

        match self.do_render(ctx) {
            Ok(()) => {}
            Err(e) => {
                self.cleanup(ctx, Some(&e));
                return RenderResult::Failed { error: e };
            }
        }

        let render_time = self.now_us() - render_start;
        let draw_calls = self.draw_call_count() - draw_calls_before;
        let vertex_count = self.vertex_count() - vertex_count_before;

        // Step 6: Post-render (overlays, debug info)
        if let Err(e) = self.post_render(ctx) {
            self.cleanup(ctx, Some(&e));
            return RenderResult::Failed { error: e };
        }

        // Step 7: Cleanup
        let cleanup_start = self.now_us();
        self.cleanup(ctx, None);
        let cleanup_time = self.now_us() - cleanup_start;

        RenderResult::Success {
            stats: RenderStats {
                prepare_time_us: prepare_time,
                render_time_us: render_time,
                cleanup_time_us: cleanup_time,
                draw_calls,
                vertex_count,
            },
        }
    }

    // ========================================================================
    // Overrideable Steps (with default implementations)
    // ========================================================================

    /// Prepare rendering resources
    fn prepare(&self, _ctx: &RenderContext) -> Result<(), String> {
        Ok(())
    }

    /// Validate render state before rendering
    fn validate(&self, _ctx: &RenderContext) -> Result<(), String> {
        Ok(())
    }

    /// Clear the render target
    fn clear(&self, _ctx: &RenderContext) {
        // Default: do nothing
    }

    /// Pre-render step (setup, calculations)
    fn pre_render(&self, _ctx: &RenderContext) -> Result<(), String> {
        Ok(())
    }

    /// Main render step - MUST be implemented
    fn do_render(&self, ctx: &RenderContext) -> Result<(), String>;

    /// Post-render step (overlays, debug info)
    fn post_render(&self, _ctx: &RenderContext) -> Result<(), String> {
        Ok(())
    }

    /// Cleanup rendering resources
    fn cleanup(&self, _ctx: &RenderContext, _error: Option<&String>) {
        // Default: do nothing
    }

    // ========================================================================
    // Utility Methods (with default implementations)
    // ========================================================================

    /// Get current time in microseconds
    fn now_us(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0)
    }

    /// Get current draw call count
    fn draw_call_count(&self) -> u32 {
        0 // Default implementation
    }

    /// Get current vertex count
    fn vertex_count(&self) -> u32 {
        0 // Default implementation
    }

    /// Get renderer name
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

// =============================================================================
// Concrete Implementations
// =============================================================================

/// Single stream workspace renderer
#[derive(Debug, Clone)]
pub struct SingleStreamRenderer {
    /// Stream identifier
    stream_id: String,
    /// Show grid overlay
    show_grid: bool,
    /// Show motion vectors
    show_mv: bool,
    /// Show QP heatmap
    show_qp: bool,
}

impl SingleStreamRenderer {
    /// Create a new single stream renderer
    pub fn new(stream_id: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.into(),
            show_grid: false,
            show_mv: false,
            show_qp: false,
        }
    }

    /// Enable grid overlay
    pub fn with_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    /// Enable motion vector overlay
    pub fn with_mv(mut self, show: bool) -> Self {
        self.show_mv = show;
        self
    }

    /// Enable QP heatmap
    pub fn with_qp(mut self, show: bool) -> Self {
        self.show_qp = show;
        self
    }
}

impl WorkspaceRenderer for SingleStreamRenderer {
    fn do_render(&self, ctx: &RenderContext) -> Result<(), String> {
        // Render single stream
        tracing::debug!(
            "Rendering single stream: {} at {}x{}",
            self.stream_id,
            ctx.width,
            ctx.height
        );

        // In a real implementation, this would:
        // 1. Decode the frame
        // 2. Upload textures
        // 3. Render to framebuffer
        // 4. Apply overlays

        Ok(())
    }

    fn post_render(&self, _ctx: &RenderContext) -> Result<(), String> {
        // Render overlays
        if self.show_grid {
            tracing::debug!("Rendering grid overlay");
        }
        if self.show_mv {
            tracing::debug!("Rendering MV overlay");
        }
        if self.show_qp {
            tracing::debug!("Rendering QP heatmap");
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "SingleStreamRenderer"
    }
}

/// Dual stream workspace renderer
#[derive(Debug, Clone)]
pub struct DualStreamRenderer {
    /// Left stream identifier
    left_stream: String,
    /// Right stream identifier
    right_stream: String,
    /// Sync mode
    sync_mode: RendererSyncMode,
}

/// Sync mode for dual stream renderer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererSyncMode {
    /// No synchronization
    Off,
    /// Sync playhead only
    Playhead,
    /// Full synchronization
    Full,
}

impl DualStreamRenderer {
    /// Create a new dual stream renderer
    pub fn new(left: impl Into<String>, right: impl Into<String>) -> Self {
        Self {
            left_stream: left.into(),
            right_stream: right.into(),
            sync_mode: RendererSyncMode::Off,
        }
    }

    /// Set sync mode
    pub fn with_sync_mode(mut self, mode: RendererSyncMode) -> Self {
        self.sync_mode = mode;
        self
    }
}

impl WorkspaceRenderer for DualStreamRenderer {
    fn prepare(&self, ctx: &RenderContext) -> Result<(), String> {
        tracing::debug!(
            "Preparing dual stream: {} | {} at {}x{}",
            self.left_stream,
            self.right_stream,
            ctx.width,
            ctx.height
        );

        if self.sync_mode == RendererSyncMode::Full {
            // Sync frames between streams
            tracing::debug!("Syncing frames for full sync mode");
        }

        Ok(())
    }

    fn do_render(&self, ctx: &RenderContext) -> Result<(), String> {
        let split_x = ctx.width / 2;

        tracing::debug!(
            "Rendering dual stream: left={} right={} split={}",
            self.left_stream,
            self.right_stream,
            split_x
        );

        // In a real implementation, this would:
        // 1. Render left stream to left half
        // 2. Render right stream to right half
        // 3. Draw separator line

        Ok(())
    }

    fn name(&self) -> &str {
        "DualStreamRenderer"
    }
}

/// Compare workspace renderer
#[derive(Debug, Clone)]
pub struct CompareRenderer {
    /// Reference stream identifier
    reference: String,
    /// Distorted stream identifier
    distorted: String,
    /// Comparison mode
    compare_mode: CompareMode,
    /// Show difference heatmap
    show_diff: bool,
}

/// Comparison mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareMode {
    /// Side by side
    SideBySide,
    /// Difference
    Difference,
    /// Toggle (A/B)
    Toggle,
    /// Slider (wipe)
    Slider,
}

impl CompareRenderer {
    /// Create a new compare renderer
    pub fn new(reference: impl Into<String>, distorted: impl Into<String>) -> Self {
        Self {
            reference: reference.into(),
            distorted: distorted.into(),
            compare_mode: CompareMode::SideBySide,
            show_diff: false,
        }
    }

    /// Set comparison mode
    pub fn with_mode(mut self, mode: CompareMode) -> Self {
        self.compare_mode = mode;
        self
    }

    /// Enable difference visualization
    pub fn with_diff(mut self, show: bool) -> Self {
        self.show_diff = show;
        self
    }
}

impl WorkspaceRenderer for CompareRenderer {
    fn prepare(&self, ctx: &RenderContext) -> Result<(), String> {
        if self.show_diff {
            tracing::debug!("Preparing difference visualization");
        }

        match self.compare_mode {
            CompareMode::SideBySide => {
                tracing::debug!(
                    "Setting up side-by-side comparison at {}x{}",
                    ctx.width,
                    ctx.height
                );
            }
            CompareMode::Difference => {
                tracing::debug!("Setting up difference visualization");
            }
            CompareMode::Toggle => {
                tracing::debug!("Setting up toggle comparison");
            }
            CompareMode::Slider => {
                tracing::debug!("Setting up slider comparison");
            }
        }

        Ok(())
    }

    fn validate(&self, ctx: &RenderContext) -> Result<(), String> {
        if ctx.width < 640 || ctx.height < 480 {
            return Err("Resolution too low for comparison".to_string());
        }
        Ok(())
    }

    fn do_render(&self, ctx: &RenderContext) -> Result<(), String> {
        match self.compare_mode {
            CompareMode::SideBySide => {
                let split_x = ctx.width / 2;
                tracing::debug!(
                    "Rendering side-by-side: ref={} dist={} split={}",
                    self.reference,
                    self.distorted,
                    split_x
                );
            }
            CompareMode::Difference => {
                tracing::debug!("Rendering difference visualization");
            }
            CompareMode::Toggle => {
                // Toggle based on time
                let show_ref = (ctx.time_ms / 1000) % 2 == 0;
                tracing::debug!(
                    "Rendering toggle mode: showing {}",
                    if show_ref { "reference" } else { "distorted" }
                );
            }
            CompareMode::Slider => {
                tracing::debug!("Rendering slider comparison");
            }
        }

        Ok(())
    }

    fn post_render(&self, _ctx: &RenderContext) -> Result<(), String> {
        if self.show_diff {
            tracing::debug!("Rendering diff heatmap overlay");
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "CompareRenderer"
    }
}

// =============================================================================
// Renderer Factory
// =============================================================================

/// Workspace renderer factory
pub struct RendererFactory;

impl RendererFactory {
    /// Create a single stream renderer
    pub fn create_single_stream(stream_id: impl Into<String>) -> SingleStreamRenderer {
        SingleStreamRenderer::new(stream_id)
    }

    /// Create a dual stream renderer
    pub fn create_dual_stream(
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> DualStreamRenderer {
        DualStreamRenderer::new(left, right)
    }

    /// Create a compare renderer
    pub fn create_compare(
        reference: impl Into<String>,
        distorted: impl Into<String>,
    ) -> CompareRenderer {
        CompareRenderer::new(reference, distorted)
    }
}

// =============================================================================
// Renderer Composition
// =============================================================================

/// Composite renderer that renders multiple renderers
pub struct CompositeRenderer {
    /// Child renderers
    renderers: Vec<Arc<dyn WorkspaceRenderer>>,
}

impl CompositeRenderer {
    /// Create a new composite renderer
    pub fn new() -> Self {
        Self {
            renderers: Vec::new(),
        }
    }

    /// Add a child renderer
    pub fn add(mut self, renderer: Arc<dyn WorkspaceRenderer>) -> Self {
        self.renderers.push(renderer);
        self
    }
}

impl Default for CompositeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceRenderer for CompositeRenderer {
    fn do_render(&self, ctx: &RenderContext) -> Result<(), String> {
        for renderer in &self.renderers {
            let result = renderer.render(ctx);
            if !result.is_success() {
                return Err(format!("Renderer failed: {}", renderer.name()));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "CompositeRenderer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_context_default() {
        let ctx = RenderContext::default();
        assert_eq!(ctx.width, 1920);
        assert_eq!(ctx.height, 1080);
        assert_eq!(ctx.dpi_scale, 1.0);
    }

    #[test]
    fn test_render_stats_total_time() {
        let stats = RenderStats {
            prepare_time_us: 100,
            render_time_us: 500,
            cleanup_time_us: 50,
            ..Default::default()
        };

        assert_eq!(stats.total_time_us(), 650);
        assert_eq!(stats.total_time_ms(), 0.65);
    }

    #[test]
    fn test_render_result_success() {
        let result = RenderResult::Success {
            stats: RenderStats::default(),
        };

        assert!(result.is_success());
        assert!(result.stats().is_some());
    }

    #[test]
    fn test_render_result_failed() {
        let result = RenderResult::Failed {
            error: "Test error".to_string(),
        };

        assert!(!result.is_success());
        assert!(result.stats().is_none());
    }

    #[test]
    fn test_single_stream_renderer() {
        let renderer = SingleStreamRenderer::new("stream_0")
            .with_grid(true)
            .with_mv(false)
            .with_qp(true);

        let ctx = RenderContext::default();
        let result = renderer.render(&ctx);

        assert!(result.is_success());
        assert_eq!(renderer.name(), "SingleStreamRenderer");
    }

    #[test]
    fn test_dual_stream_renderer() {
        let renderer =
            DualStreamRenderer::new("stream_a", "stream_b").with_sync_mode(RendererSyncMode::Full);

        let ctx = RenderContext::default();
        let result = renderer.render(&ctx);

        assert!(result.is_success());
        assert_eq!(renderer.name(), "DualStreamRenderer");
    }

    #[test]
    fn test_compare_renderer() {
        let renderer = CompareRenderer::new("ref", "dist")
            .with_mode(CompareMode::Difference)
            .with_diff(true);

        let ctx = RenderContext::default();
        let result = renderer.render(&ctx);

        assert!(result.is_success());
        assert_eq!(renderer.name(), "CompareRenderer");
    }

    #[test]
    fn test_compare_renderer_low_resolution() {
        let renderer = CompareRenderer::new("ref", "dist");

        let ctx = RenderContext {
            width: 320,
            height: 240,
            ..Default::default()
        };

        let result = renderer.render(&ctx);

        assert!(!result.is_success());
    }

    #[test]
    fn test_composite_renderer() {
        let renderer = CompositeRenderer::new()
            .add(Arc::new(SingleStreamRenderer::new("stream_0")))
            .add(Arc::new(SingleStreamRenderer::new("stream_1")));

        let ctx = RenderContext::default();
        let result = renderer.render(&ctx);

        assert!(result.is_success());
        assert_eq!(renderer.name(), "CompositeRenderer");
    }

    #[test]
    fn test_renderer_factory() {
        let single = RendererFactory::create_single_stream("stream_0");
        let dual = RendererFactory::create_dual_stream("stream_a", "stream_b");
        let compare = RendererFactory::create_compare("ref", "dist");

        assert_eq!(single.name(), "SingleStreamRenderer");
        assert_eq!(dual.name(), "DualStreamRenderer");
        assert_eq!(compare.name(), "CompareRenderer");
    }
}
