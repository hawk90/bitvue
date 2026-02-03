//! Graph data structures and algorithms.
//!
//! This module provides graph data structures and algorithms similar to
//! Abseil's graph utilities (and Boost Graph Library concepts).
//!
//! # Overview
//!
//! Graph utilities provide data structures for representing graphs and
//! algorithms for operating on them. Supports both directed and undirected
//! graphs, weighted and unweighted graphs.
//!
//! # Components
//!
//! - [`Graph`] - Basic directed graph with adjacency list representation
//! - [`UndirectedGraph`] - Undirected graph
//! - [`WeightedGraph`] - Weighted directed graph
//! - [`algorithms`] - Graph algorithms (DFS, BFS, shortest path, etc.)
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_graph::{Graph, dijkstra};
//!
//! // Create a directed graph
//! let mut graph = Graph::new();
//! let v1 = graph.add_vertex(1);
//! let v2 = graph.add_vertex(2);
//! let v3 = graph.add_vertex(3);
//! graph.add_edge(v1, v2, Some(5.0));
//! graph.add_edge(v2, v3, Some(3.0));
//!
//! // Find shortest path
//! let path = dijkstra(&graph, v1, v3);
//! ```


extern crate alloc;

use alloc::vec::Vec;
use core::ops::Index;

pub mod adjacency;
pub mod algorithms;
pub mod shortest_path;
pub mod traversal;
pub mod topology;
pub mod matching;
pub mod flow;
pub mod coloring;
pub mod connectivity;
pub mod min_spanning_tree;
pub mod pathfinding;
pub mod astar;
pub mod maxflow;

// Re-exports
pub use adjacency::{AdjacencyList, AdjacencyMatrix};
pub use algorithms::{bfs, dfs, topological_sort};
pub use shortest_path::{dijkstra, bellman_ford, floyd_warshall, shortest_path};
pub use traversal::{DFSIterator, BFSIterator, DepthFirst, BreadthFirst};
pub use topology::{TopologicalSort, TopologicalOrder};
pub use matching::{maximum_matching, bipartite_matching};
pub use flow::{MaxFlow, min_cut, max_flow};
pub use coloring::{GraphColoring, greedy_coloring, chromatic_number};
pub use connectivity::{is_connected, components, strongly_connected_components};
pub use min_spanning_tree::{MinimumSpanningTree, kruskal, prim};
pub use pathfinding::{astar, PathFinder};
pub use astar::{astar_search, chebyshev_distance, euclidean_distance, manhattan_distance};
pub use maxflow::{dinic, edmonds_karp, ford_fulkerson, min_st_cut, FlowNetwork};

/// A vertex identifier in a graph.
pub type VertexId = usize;

/// An edge identifier in a graph.
pub type EdgeId = usize;

/// A basic directed graph with adjacency list representation.
#[derive(Clone, Debug)]
pub struct Graph<T> {
    vertices: Vec<Vertex<T>>,
    edges: Vec<Edge>,
    adjacency: Vec<Vec<EdgeId>>,
}

/// A vertex in the graph.
#[derive(Clone, Debug)]
pub struct Vertex<T> {
    pub id: VertexId,
    pub data: T,
}

/// An edge in the graph.
#[derive(Clone, Debug)]
pub struct Edge {
    pub id: EdgeId,
    pub from: VertexId,
    pub to: VertexId,
    pub weight: Option<f64>,
}

impl<T> Graph<T> {
    /// Creates a new empty graph.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_graph::Graph;
    ///
    /// let graph: Graph<i32> = Graph::new();
    /// ```
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            edges: Vec::new(),
            adjacency: Vec::new(),
        }
    }

    /// Adds a vertex to the graph and returns its ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_graph::Graph;
    ///
    /// let mut graph = Graph::new();
    /// let v1 = graph.add_vertex(42);
    /// assert_eq!(graph.vertex(v1).unwrap().data, 42);
    /// ```
    pub fn add_vertex(&mut self, data: T) -> VertexId {
        let id = self.vertices.len();
        self.vertices.push(Vertex { id, data });
        self.adjacency.push(Vec::new());
        id
    }

    /// Adds an edge to the graph.
    ///
    /// # Panics
    ///
    /// Panics if `from` or `to` vertex IDs are invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_graph::Graph;
    ///
    /// let mut graph = Graph::new();
    /// let v1 = graph.add_vertex(1);
    /// let v2 = graph.add_vertex(2);
    /// graph.add_edge(v1, v2, None);
    /// ```
    pub fn add_edge(&mut self, from: VertexId, to: VertexId, weight: Option<f64>) -> EdgeId {
        // SAFETY: Validate vertex IDs before accessing adjacency list
        if from >= self.adjacency.len() {
            panic!(
                "Source vertex ID {} out of bounds (vertex count: {})",
                from,
                self.adjacency.len()
            );
        }
        if to >= self.adjacency.len() {
            panic!(
                "Target vertex ID {} out of bounds (vertex count: {})",
                to,
                self.adjacency.len()
            );
        }

        let id = self.edges.len();
        self.edges.push(Edge { id, from, to, weight });
        self.adjacency[from].push(id);
        id
    }

    /// Adds a weighted edge to the graph.
    pub fn add_weighted_edge(&mut self, from: VertexId, to: VertexId, weight: f64) -> EdgeId {
        self.add_edge(from, to, Some(weight))
    }

    /// Returns the number of vertices in the graph.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns a reference to a vertex, if it exists.
    pub fn vertex(&self, id: VertexId) -> Option<&Vertex<T>> {
        self.vertices.get(id)
    }

    /// Returns a reference to an edge, if it exists.
    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(id)
    }

    /// Returns the neighbors of a vertex.
    pub fn neighbors(&self, id: VertexId) -> impl Iterator<Item = &Vertex<T>> {
        self.adjacency
            .get(id)
            .into_iter()
            .flat_map(|edges| edges.iter())
            .filter_map(move |&edge_id| self.edge(edge_id))
            .filter_map(|edge| self.vertex(edge.to))
    }

    /// Returns the outgoing edges from a vertex.
    pub fn outgoing_edges(&self, id: VertexId) -> impl Iterator<Item = &Edge> {
        self.adjacency
            .get(id)
            .into_iter()
            .flat_map(|edges| edges.iter())
            .filter_map(move |&edge_id| self.edge(edge_id))
    }

    /// Checks if there's an edge between two vertices.
    pub fn has_edge(&self, from: VertexId, to: VertexId) -> bool {
        self.outgoing_edges(from).any(|e| e.to == to)
    }

    /// Returns the weight of an edge, if it exists and is weighted.
    pub fn edge_weight(&self, from: VertexId, to: VertexId) -> Option<f64> {
        self.outgoing_edges(from)
            .find(|e| e.to == to)
            .and_then(|e| e.weight)
    }
}

impl<T> Default for Graph<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Index<VertexId> for Graph<T> {
    type Output = Vertex<T>;

    fn index(&self, index: VertexId) -> &Self::Output {
        self.vertex(index).expect("Vertex index out of bounds")
    }
}

/// An undirected graph.
#[derive(Clone, Debug)]
pub struct UndirectedGraph<T> {
    graph: Graph<T>,
}

impl<T> UndirectedGraph<T> {
    /// Creates a new empty undirected graph.
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    /// Adds a vertex to the graph.
    pub fn add_vertex(&mut self, data: T) -> VertexId {
        self.graph.add_vertex(data)
    }

    /// Adds an undirected edge to the graph.
    pub fn add_edge(&mut self, v1: VertexId, v2: VertexId, weight: Option<f64>) {
        self.graph.add_edge(v1, v2, weight);
        self.graph.add_edge(v2, v1, weight);
    }

    /// Returns the underlying directed graph.
    pub fn as_graph(&self) -> &Graph<T> {
        &self.graph
    }

    /// Returns the number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.graph.vertex_count()
    }

    /// Returns the number of unique edges (each undirected edge counts as 1).
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count() / 2
    }
}

impl<T> Default for UndirectedGraph<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A weighted graph that stores edge weights.
#[derive(Clone, Debug)]
pub struct WeightedGraph<T> {
    graph: Graph<T>,
}

impl<T> WeightedGraph<T> {
    /// Creates a new empty weighted graph.
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    /// Adds a vertex to the graph.
    pub fn add_vertex(&mut self, data: T) -> VertexId {
        self.graph.add_vertex(data)
    }

    /// Adds a weighted edge to the graph.
    pub fn add_edge(&mut self, from: VertexId, to: VertexId, weight: f64) {
        self.graph.add_weighted_edge(from, to, weight);
    }

    /// Returns the underlying graph.
    pub fn as_graph(&self) -> &Graph<T> {
        &self.graph
    }
}

impl<T> Default for WeightedGraph<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_new() {
        let graph: Graph<i32> = Graph::new();
        assert_eq!(graph.vertex_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_graph_add_vertex() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);

        assert_eq!(graph.vertex_count(), 2);
        assert_eq!(graph.vertex(v1).unwrap().data, 1);
        assert_eq!(graph.vertex(v2).unwrap().data, 2);
    }

    #[test]
    fn test_graph_add_edge() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        graph.add_edge(v1, v2, None);

        assert_eq!(graph.edge_count(), 1);
        assert!(graph.has_edge(v1, v2));
        assert!(!graph.has_edge(v2, v1));
    }

    #[test]
    fn test_graph_weighted_edge() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        graph.add_weighted_edge(v1, v2, 5.0);

        assert_eq!(graph.edge_weight(v1, v2), Some(5.0));
    }

    #[test]
    fn test_graph_neighbors() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v1, v3, None);

        let neighbors: Vec<_> = graph.neighbors(v1).map(|v| v.data).collect();
        assert_eq!(neighbors, vec![2, 3]);
    }

    #[test]
    fn test_undirected_graph() {
        let mut graph = UndirectedGraph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        graph.add_edge(v1, v2, None);

        assert_eq!(graph.edge_count(), 1);
        assert!(graph.as_graph().has_edge(v1, v2));
        assert!(graph.as_graph().has_edge(v2, v1));
    }

    #[test]
    fn test_weighted_graph() {
        let mut graph = WeightedGraph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        graph.add_edge(v1, v2, 10.0);

        assert_eq!(graph.as_graph().edge_weight(v1, v2), Some(10.0));
    }

    // Tests for MEDIUM security fix - bounds checking

    #[test]
    #[should_panic(expected = "Source vertex ID 5 out of bounds")]
    fn test_graph_add_edge_invalid_source() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        // Adding edge from non-existent vertex should panic
        graph.add_edge(5, v1, None);
    }

    #[test]
    #[should_panic(expected = "Target vertex ID 5 out of bounds")]
    fn test_graph_add_edge_invalid_target() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        // Adding edge to non-existent vertex should panic
        graph.add_edge(v1, 5, None);
    }

    #[test]
    fn test_graph_add_edge_valid_vertices() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        // This should work fine
        graph.add_edge(v1, v2, None);
        assert_eq!(graph.edge_count(), 1);
    }
}
