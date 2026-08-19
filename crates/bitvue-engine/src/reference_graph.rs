//! Reference Graph View - T5-1
//!
//! Per WS_REFERENCE_DPB.md and UI_INTERACTION_RULEBOOK:
//! - Frame reference graph visualization
//! - World bounds clamp for pan/zoom
//! - Zoom limits (min/max)
//! - Summary mode for complexity reduction
//! - Search/filter capabilities

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Reference type (L0 forward, L1 backward)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceType {
    /// L0 (forward prediction)
    L0,
    /// L1 (backward prediction)
    L1,
    /// Long-term reference
    LongTerm,
}

impl ReferenceType {
    pub fn name(&self) -> &'static str {
        match self {
            ReferenceType::L0 => "L0",
            ReferenceType::L1 => "L1",
            ReferenceType::LongTerm => "Long-term",
        }
    }

    pub fn color_hint(&self) -> &'static str {
        match self {
            ReferenceType::L0 => "blue",
            ReferenceType::L1 => "green",
            ReferenceType::LongTerm => "orange",
        }
    }
}

/// Reference edge between frames
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceEdge {
    /// Source frame (display_idx)
    pub from_idx: usize,
    /// Target frame (display_idx)
    pub to_idx: usize,
    /// Reference type
    pub ref_type: ReferenceType,
    /// Optional weight/importance
    pub weight: Option<f32>,
}

impl ReferenceEdge {
    pub fn new(from_idx: usize, to_idx: usize, ref_type: ReferenceType) -> Self {
        Self {
            from_idx,
            to_idx,
            ref_type,
            weight: None,
        }
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = Some(weight);
        self
    }
}

/// Graph node representing a frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Frame display_idx
    pub display_idx: usize,
    /// Frame type (I, P, B)
    pub frame_type: String,
    /// Node position (world coordinates)
    pub position: (f32, f32),
    /// Is selected
    pub is_selected: bool,
    /// Is highlighted (as ref or dependent)
    pub is_highlighted: bool,
}

impl GraphNode {
    pub fn new(display_idx: usize, frame_type: String, position: (f32, f32)) -> Self {
        Self {
            display_idx,
            frame_type,
            position,
            is_selected: false,
            is_highlighted: false,
        }
    }
}

/// World bounds for viewport safety
///
/// Per T5-1 deliverable: "World bounds clamp"
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl WorldBounds {
    pub fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    /// Clamp a point to world bounds
    pub fn clamp_point(&self, x: f32, y: f32) -> (f32, f32) {
        let clamped_x = x.max(self.min_x).min(self.max_x);
        let clamped_y = y.max(self.min_y).min(self.max_y);
        (clamped_x, clamped_y)
    }

    /// Check if point is within bounds
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    /// Get bounds width
    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    /// Get bounds height
    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }
}

/// Zoom bounds for viewport safety
///
/// Per T5-1 deliverable: "Zoom bounds"
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ZoomBounds {
    /// Minimum zoom level (e.g., 0.1 = 10%)
    pub min_zoom: f32,
    /// Maximum zoom level (e.g., 10.0 = 1000%)
    pub max_zoom: f32,
}

impl ZoomBounds {
    pub fn new(min_zoom: f32, max_zoom: f32) -> Self {
        Self { min_zoom, max_zoom }
    }

    /// Clamp zoom level to bounds
    pub fn clamp(&self, zoom: f32) -> f32 {
        zoom.max(self.min_zoom).min(self.max_zoom)
    }

    /// Check if zoom is within bounds
    pub fn is_valid(&self, zoom: f32) -> bool {
        zoom >= self.min_zoom && zoom <= self.max_zoom
    }
}

impl Default for ZoomBounds {
    fn default() -> Self {
        Self::new(0.1, 10.0)
    }
}

/// Graph display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphMode {
    /// Full mode: Show all nodes and edges
    Full,
    /// Summary mode: Reduce complexity (cluster, simplify)
    Summary,
}

/// Reference filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceFilter {
    /// Show only nodes with references (hide isolated)
    pub refs_only: bool,
    /// Show only nodes that are dependencies of others
    pub deps_only: bool,
    /// Filter by frame type (I, P, B)
    pub frame_types: HashSet<String>,
    /// Filter by reference type
    pub ref_types: HashSet<ReferenceType>,
}

impl ReferenceFilter {
    pub fn new() -> Self {
        Self {
            refs_only: false,
            deps_only: false,
            frame_types: HashSet::new(),
            ref_types: HashSet::new(),
        }
    }

    /// Check if a node passes the filter
    pub fn matches_node(&self, node: &GraphNode, has_refs: bool, is_dep: bool) -> bool {
        // Frame type filter
        if !self.frame_types.is_empty() && !self.frame_types.contains(&node.frame_type) {
            return false;
        }

        // Refs only filter
        if self.refs_only && !has_refs {
            return false;
        }

        // Deps only filter
        if self.deps_only && !is_dep {
            return false;
        }

        true
    }

    /// Check if an edge passes the filter
    pub fn matches_edge(&self, edge: &ReferenceEdge) -> bool {
        if !self.ref_types.is_empty() && !self.ref_types.contains(&edge.ref_type) {
            return false;
        }
        true
    }
}

impl Default for ReferenceFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Reference Graph View
///
/// Per T5-1 deliverable: ReferenceGraphView with viewport safety
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceGraphView {
    /// All graph nodes (frames)
    pub nodes: Vec<GraphNode>,

    /// All reference edges
    pub edges: Vec<ReferenceEdge>,

    /// Current display mode
    pub mode: GraphMode,

    /// World bounds for viewport safety
    pub world_bounds: WorldBounds,

    /// Zoom bounds
    pub zoom_bounds: ZoomBounds,

    /// Current zoom level
    pub zoom: f32,

    /// Pan offset (world coordinates)
    pub pan_offset: (f32, f32),

    /// Reference filter
    pub filter: ReferenceFilter,

    /// Search query (frame index or type)
    pub search_query: Option<String>,

    /// Selected frame index
    pub selected_frame: Option<usize>,
}

impl ReferenceGraphView {
    /// Create a new reference graph view
    pub fn new(world_bounds: WorldBounds) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            mode: GraphMode::Full,
            world_bounds,
            zoom_bounds: ZoomBounds::default(),
            zoom: 1.0,
            pan_offset: (0.0, 0.0),
            filter: ReferenceFilter::default(),
            search_query: None,
            selected_frame: None,
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: ReferenceEdge) {
        self.edges.push(edge);
    }

    /// Set display mode
    ///
    /// Per T5-1 deliverable: "Summary mode"
    pub fn set_mode(&mut self, mode: GraphMode) {
        self.mode = mode;
    }

    /// Set zoom level (with bounds clamping)
    ///
    /// Per T5-1 deliverable: "Zoom bounds"
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = self.zoom_bounds.clamp(zoom);
    }

    /// Zoom in (multiply by factor)
    pub fn zoom_in(&mut self, factor: f32) {
        self.set_zoom(self.zoom * factor);
    }

    /// Zoom out (divide by factor)
    pub fn zoom_out(&mut self, factor: f32) {
        self.set_zoom(self.zoom / factor);
    }

    /// Pan viewport (with bounds clamping)
    ///
    /// Per T5-1 deliverable: "World bounds clamp"
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let new_x = self.pan_offset.0 + delta_x;
        let new_y = self.pan_offset.1 + delta_y;
        self.pan_offset = self.world_bounds.clamp_point(new_x, new_y);
    }

    /// Reset view to fit all nodes
    pub fn fit_to_content(&mut self) {
        if self.nodes.is_empty() {
            return;
        }

        // Calculate bounding box of all nodes
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for node in &self.nodes {
            let (x, y) = node.position;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }

        // Center pan on content
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;
        self.pan_offset = (center_x, center_y);

        // Adjust zoom to fit
        let content_width = max_x - min_x;
        let content_height = max_y - min_y;
        let world_width = self.world_bounds.width();
        let world_height = self.world_bounds.height();

        let zoom_x = world_width / content_width;
        let zoom_y = world_height / content_height;
        let target_zoom = zoom_x.min(zoom_y) * 0.9; // 90% to leave padding

        self.set_zoom(target_zoom);
    }

    /// Select a frame
    pub fn select_frame(&mut self, display_idx: usize) {
        // Clear previous selection
        for node in &mut self.nodes {
            node.is_selected = false;
            node.is_highlighted = false;
        }

        // Set new selection
        self.selected_frame = Some(display_idx);

        // Highlight selected node and its refs/deps
        let refs = self.get_references(display_idx);
        let deps = self.get_dependents(display_idx);

        for node in &mut self.nodes {
            if node.display_idx == display_idx {
                node.is_selected = true;
            } else if refs.contains(&node.display_idx) || deps.contains(&node.display_idx) {
                node.is_highlighted = true;
            }
        }
    }

    /// Get references for a frame (frames it depends on)
    pub fn get_references(&self, display_idx: usize) -> HashSet<usize> {
        self.edges
            .iter()
            .filter(|e| e.from_idx == display_idx)
            .map(|e| e.to_idx)
            .collect()
    }

    /// Get dependents for a frame (frames that depend on it)
    pub fn get_dependents(&self, display_idx: usize) -> HashSet<usize> {
        self.edges
            .iter()
            .filter(|e| e.to_idx == display_idx)
            .map(|e| e.from_idx)
            .collect()
    }

    /// Get reference count for a frame
    pub fn reference_count(&self, display_idx: usize) -> usize {
        self.edges
            .iter()
            .filter(|e| e.from_idx == display_idx)
            .count()
    }

    /// Get dependent count for a frame
    pub fn dependent_count(&self, display_idx: usize) -> usize {
        self.edges
            .iter()
            .filter(|e| e.to_idx == display_idx)
            .count()
    }

    /// Set search query
    ///
    /// Per T5-1 deliverable: "Search/Filter"
    pub fn set_search_query(&mut self, query: Option<String>) {
        self.search_query = query;
    }

    /// Check if a node matches search query
    pub fn matches_search(&self, node: &GraphNode) -> bool {
        if let Some(ref query) = self.search_query {
            let query_lower = query.to_lowercase();
            node.display_idx.to_string().contains(&query_lower)
                || node.frame_type.to_lowercase().contains(&query_lower)
        } else {
            true
        }
    }

    /// Get filtered nodes
    pub fn filtered_nodes(&self) -> Vec<&GraphNode> {
        let has_refs_map: HashMap<usize, bool> = self
            .nodes
            .iter()
            .map(|n| (n.display_idx, self.reference_count(n.display_idx) > 0))
            .collect();

        let is_dep_map: HashMap<usize, bool> = self
            .nodes
            .iter()
            .map(|n| (n.display_idx, self.dependent_count(n.display_idx) > 0))
            .collect();

        self.nodes
            .iter()
            .filter(|n| {
                let has_refs = has_refs_map.get(&n.display_idx).copied().unwrap_or(false);
                let is_dep = is_dep_map.get(&n.display_idx).copied().unwrap_or(false);
                self.filter.matches_node(n, has_refs, is_dep) && self.matches_search(n)
            })
            .collect()
    }

    /// Get filtered edges
    pub fn filtered_edges(&self) -> Vec<&ReferenceEdge> {
        self.edges
            .iter()
            .filter(|e| self.filter.matches_edge(e))
            .collect()
    }

    /// Calculate reference depth (max chain length) for a frame
    ///
    /// Per WS_REFERENCE_DPB: "ref depth metric + missing ref flags"
    pub fn reference_depth(&self, display_idx: usize) -> usize {
        let mut visited = HashSet::new();
        self.reference_depth_recursive(display_idx, &mut visited)
    }

    fn reference_depth_recursive(&self, idx: usize, visited: &mut HashSet<usize>) -> usize {
        if visited.contains(&idx) {
            return 0; // Cycle detected
        }
        visited.insert(idx);

        let refs = self.get_references(idx);
        if refs.is_empty() {
            1
        } else {
            1 + refs
                .iter()
                .map(|&ref_idx| self.reference_depth_recursive(ref_idx, visited))
                .max()
                .unwrap_or(0)
        }
    }

    /// Get statistics summary for UI display
    ///
    /// Per COMPETITOR_PARITY_STATUS.md §4.3: Reference list / DPB view
    pub fn statistics(&self) -> ReferenceGraphStats {
        let total_nodes = self.nodes.len();
        let total_edges = self.edges.len();

        // Count by frame type
        let mut i_frames = 0;
        let mut p_frames = 0;
        let mut b_frames = 0;
        let mut other_frames = 0;

        for node in &self.nodes {
            match node.frame_type.as_str() {
                "I" => i_frames += 1,
                "P" => p_frames += 1,
                "B" => b_frames += 1,
                _ => other_frames += 1,
            }
        }

        // Count by reference type
        let l0_edges = self
            .edges
            .iter()
            .filter(|e| e.ref_type == ReferenceType::L0)
            .count();
        let l1_edges = self
            .edges
            .iter()
            .filter(|e| e.ref_type == ReferenceType::L1)
            .count();
        let lt_edges = self
            .edges
            .iter()
            .filter(|e| e.ref_type == ReferenceType::LongTerm)
            .count();

        // Calculate max reference depth
        let max_depth = self
            .nodes
            .iter()
            .map(|n| self.reference_depth(n.display_idx))
            .max()
            .unwrap_or(0);

        // Count isolated frames (no refs, no deps)
        let isolated_count = self
            .nodes
            .iter()
            .filter(|n| {
                self.reference_count(n.display_idx) == 0 && self.dependent_count(n.display_idx) == 0
            })
            .count();

        // Average refs per frame
        let avg_refs_per_frame = if total_nodes > 0 {
            total_edges as f32 / total_nodes as f32
        } else {
            0.0
        };

        ReferenceGraphStats {
            total_nodes,
            total_edges,
            i_frames,
            p_frames,
            b_frames,
            other_frames,
            l0_edges,
            l1_edges,
            lt_edges,
            max_depth,
            isolated_count,
            avg_refs_per_frame,
        }
    }

    /// Auto-layout nodes in a timeline style
    ///
    /// Positions nodes left-to-right by display_idx with frame type rows
    pub fn auto_layout_timeline(&mut self, spacing_x: f32, spacing_y: f32) {
        // Group by frame type for Y positioning
        let mut type_offsets: HashMap<String, f32> = HashMap::new();
        type_offsets.insert("I".to_string(), 0.0);
        type_offsets.insert("P".to_string(), spacing_y);
        type_offsets.insert("B".to_string(), spacing_y * 2.0);

        for node in &mut self.nodes {
            let x = node.display_idx as f32 * spacing_x;
            let y = type_offsets
                .get(&node.frame_type)
                .copied()
                .unwrap_or(spacing_y * 3.0);
            node.position = (x, y);
        }

        // Update world bounds based on nodes
        if !self.nodes.is_empty() {
            let max_x = self
                .nodes
                .iter()
                .map(|n| n.position.0)
                .fold(0.0f32, |a, b| a.max(b));
            let max_y = self
                .nodes
                .iter()
                .map(|n| n.position.1)
                .fold(0.0f32, |a, b| a.max(b));
            self.world_bounds =
                WorldBounds::new(-spacing_x, -spacing_y, max_x + spacing_x, max_y + spacing_y);
        }
    }

    /// Get summary text for panel header
    pub fn summary_text(&self) -> String {
        let stats = self.statistics();
        format!(
            "{} frames ({} I / {} P / {} B) | {} refs | Depth: {}",
            stats.total_nodes,
            stats.i_frames,
            stats.p_frames,
            stats.b_frames,
            stats.total_edges,
            stats.max_depth
        )
    }

    /// Get node at position (for click detection)
    pub fn node_at_position(&self, x: f32, y: f32, radius: f32) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| {
            let dx = n.position.0 - x;
            let dy = n.position.1 - y;
            (dx * dx + dy * dy).sqrt() <= radius
        })
    }

    /// Center view on a specific node
    pub fn center_on_node(&mut self, display_idx: usize) {
        if let Some(node) = self.nodes.iter().find(|n| n.display_idx == display_idx) {
            self.pan_offset = node.position;
        }
    }
}

/// Reference graph statistics for UI display
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReferenceGraphStats {
    /// Total number of nodes
    pub total_nodes: usize,
    /// Total number of edges
    pub total_edges: usize,
    /// I-frame count
    pub i_frames: usize,
    /// P-frame count
    pub p_frames: usize,
    /// B-frame count
    pub b_frames: usize,
    /// Other frame types
    pub other_frames: usize,
    /// L0 reference count
    pub l0_edges: usize,
    /// L1 reference count
    pub l1_edges: usize,
    /// Long-term reference count
    pub lt_edges: usize,
    /// Maximum reference depth
    pub max_depth: usize,
    /// Count of isolated frames (no refs/deps)
    pub isolated_count: usize,
    /// Average references per frame
    pub avg_refs_per_frame: f32,
}
