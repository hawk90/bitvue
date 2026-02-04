//! Maximum flow algorithms.
//!
//! Implements Ford-Fulkerson, Edmonds-Karp, and related algorithms.

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::vec::Vec;

use crate::absl_graph::{Graph, VertexId};

/// Represents an edge with flow and capacity information.
#[derive(Clone, Debug)]
pub struct FlowEdge {
    pub to: VertexId,
    pub capacity: u64,
    pub flow: u64,
}

/// A flow network representation.
#[derive(Clone, Debug)]
pub struct FlowNetwork {
    pub adj: BTreeMap<VertexId, Vec<FlowEdge>>,
}

impl FlowNetwork {
    /// Creates a new empty flow network.
    pub fn new() -> Self {
        Self {
            adj: BTreeMap::new(),
        }
    }

    /// Adds an edge to the network.
    pub fn add_edge(&mut self, from: VertexId, to: VertexId, capacity: u64) {
        self.adj.entry(from).or_default().push(FlowEdge {
            to,
            capacity,
            flow: 0,
        });
    }

    /// Builds a flow network from a Graph with edge weights as capacities.
    pub fn from_graph<T>(graph: &Graph<T>) -> Self {
        let mut network = Self::new();

        for v in 0..graph.vertex_count() {
            for edge in graph.outgoing_edges(v) {
                if let Some(cap) = edge.weight {
                    network.add_edge(edge.from, edge.to, cap as u64);
                }
            }
        }

        network
    }
}

/// Ford-Fulkerson maximum flow algorithm.
///
/// Uses depth-first search to find augmenting paths.
/// Returns the maximum flow value and the final flow network.
///
/// # Arguments
///
/// * `network` - The flow network (will be modified)
/// * `source` - Source vertex
/// * `sink` - Sink vertex
///
/// # Returns
///
/// The maximum flow value.
pub fn ford_fulkerson(network: &mut FlowNetwork, source: VertexId, sink: VertexId) -> u64 {
    let mut max_flow = 0;

    // While there's an augmenting path
    while let Some(path_flow) = dfs_find_path(network, source, sink, u64::MAX) {
        max_flow += path_flow;
    }

    max_flow
}

/// Edmonds-Karp algorithm (BFS-based Ford-Fulkerson).
///
/// Uses breadth-first search to find augmenting paths.
/// More efficient than basic Ford-Fulkerson for most cases.
///
/// # Arguments
///
/// * `network` - The flow network (will be modified)
/// * `source` - Source vertex
/// * `sink` - Sink vertex
///
/// # Returns
///
/// The maximum flow value.
pub fn edmonds_karp(network: &mut FlowNetwork, source: VertexId, sink: VertexId) -> u64 {
    let mut max_flow = 0;

    // While there's an augmenting path
    while let Some(path_flow) = bfs_find_path(network, source, sink, u64::MAX) {
        max_flow += path_flow;
    }

    max_flow
}

/// Finds an augmenting path using DFS.
///
/// Returns the amount of flow that can be pushed, and updates the network.
fn dfs_find_path(
    network: &mut FlowNetwork,
    current: VertexId,
    sink: VertexId,
    min_capacity: u64,
) -> Option<u64> {
    if current == sink {
        return Some(min_capacity);
    }

    // Collect edges to iterate
    let edges: Vec<_> = network.adj.get(&current)?.clone();

    for edge in edges {
        let remaining = edge.capacity - edge.flow;

        if remaining > 0 {
            let flow = dfs_find_path(network, edge.to, sink, min_capacity.min(remaining))?;

            if flow > 0 {
                // Update edge flow
                update_edge_flow(network, current, edge.to, flow);
                return Some(flow);
            }
        }
    }

    None
}

/// Finds an augmenting path using BFS (for Edmonds-Karp).
fn bfs_find_path(
    network: &mut FlowNetwork,
    source: VertexId,
    sink: VertexId,
    _min_capacity: u64,
) -> Option<u64> {
    let mut parent = BTreeMap::new();
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(source);
    visited.insert(source);

    while !queue.is_empty() {
        let current = queue.pop_front().unwrap();

        if current == sink {
            // Reconstruct path and find bottleneck
            return reconstruct_and_apply_path(network, &parent, source, sink);
        }

        if let Some(edges) = network.adj.get(&current) {
            for edge in edges {
                let remaining = edge.capacity - edge.flow;

                if remaining > 0 && !visited.contains(&edge.to) {
                    visited.insert(edge.to);
                    parent.insert(edge.to, (current, remaining));
                    queue.push_back(edge.to);
                }
            }
        }
    }

    None
}

/// Reconstructs path from parent map and applies flow.
fn reconstruct_and_apply_path(
    network: &mut FlowNetwork,
    parent: &BTreeMap<VertexId, (VertexId, u64)>,
    source: VertexId,
    sink: VertexId,
) -> Option<u64> {
    // Find bottleneck
    let mut path = Vec::new();
    let mut current = sink;
    let mut min_capacity = u64::MAX;

    while current != source {
        let &(prev, capacity) = parent.get(&current)?;
        min_capacity = min_capacity.min(capacity);
        path.push((prev, current));
        current = prev;
    }

    if min_capacity == 0 {
        return None;
    }

    // Apply flow
    for (from, to) in path {
        update_edge_flow(network, from, to, min_capacity);
    }

    Some(min_capacity)
}

/// Updates the flow on an edge.
fn update_edge_flow(network: &mut FlowNetwork, from: VertexId, to: VertexId, flow: u64) {
    if let Some(edges) = network.adj.get_mut(&from) {
        for edge in edges {
            if edge.to == to {
                edge.flow += flow;
                return;
            }
        }
    }
}

/// Finds minimum s-t cut in a flow network.
///
/// Returns the vertices reachable from source in the residual graph.
pub fn min_st_cut(network: &FlowNetwork, source: VertexId, sink: VertexId) -> BTreeSet<VertexId> {
    let mut visited = BTreeSet::new();
    let mut queue = Vec::new();

    queue.push(source);
    visited.insert(source);

    while !queue.is_empty() {
        let current = queue.remove(0);

        if let Some(edges) = network.adj.get(&current) {
            for edge in edges {
                let remaining = edge.capacity - edge.flow;

                if remaining > 0 && !visited.contains(&edge.to) && edge.to != sink {
                    visited.insert(edge.to);
                    queue.push(edge.to);
                }
            }
        }
    }

    visited
}

/// Dinic's algorithm - faster max flow algorithm.
///
/// Uses blocking flow concept with level graph and DFS.
///
/// # Arguments
///
/// * `network` - The flow network
/// * `source` - Source vertex
/// * `sink` - Sink vertex
///
/// # Returns
///
/// The maximum flow value.
pub fn dinic(network: &mut FlowNetwork, source: VertexId, sink: VertexId) -> u64 {
    let mut max_flow = 0;

    // Build level graph while there's a path
    while let Some(level) = build_level_graph(network, source, sink) {
        // Send blocking flow
        while let Some(flow) = send_blocking_flow(network, &level, source, sink, u64::MAX) {
            max_flow += flow;
        }
    }

    max_flow
}

/// Builds a level graph for Dinic's algorithm.
fn build_level_graph(
    network: &FlowNetwork,
    source: VertexId,
    sink: VertexId,
) -> Option<Vec<VertexId>> {
    let mut level = BTreeMap::new();
    let mut queue = Vec::new();

    level.insert(source, 0);
    queue.push(source);

    while !queue.is_empty() {
        let current = queue.remove(0);

        if current == sink {
            // Found path to sink
            let mut result = level.iter().map(|(&v, &l)| (v, l)).collect::<Vec<_>>();
            result.sort_by_key(|&(_, l)| l);
            return Some(result.into_iter().map(|(v, _)| v).collect());
        }

        if let Some(edges) = network.adj.get(&current) {
            let current_level = level[&current];

            for edge in edges {
                let remaining = edge.capacity - edge.flow;

                if remaining > 0 && !level.contains_key(&edge.to) {
                    level.insert(edge.to, current_level + 1);
                    queue.push(edge.to);
                }
            }
        }
    }

    None
}

/// Sends blocking flow using level graph.
fn send_blocking_flow(
    network: &mut FlowNetwork,
    level: &[VertexId],
    source: VertexId,
    sink: VertexId,
    min_capacity: u64,
) -> Option<u64> {
    if source == sink {
        return Some(min_capacity);
    }

    let current_level = level.iter().position(|&v| v == source)?;
    let next_level = current_level + 1;

    // Collect edge data to avoid borrow issues
    let edge_data: Vec<_> = network
        .adj
        .get(&source)
        .map(|v| {
            v.iter()
                .filter_map(|e| {
                    // Check if edge.to is at the next level
                    let edge_level = level.iter().position(|&v| v == e.to);
                    edge_level
                        .map_or(false, |l| l == next_level)
                        .then_some((e.to, e.capacity, e.flow))
                })
                .collect()
        })
        .unwrap_or_default();

    for (to, capacity, flow) in edge_data {
        let remaining = capacity - flow;

        if remaining > 0 {
            let flow = send_blocking_flow(network, level, to, sink, min_capacity.min(remaining))?;

            if flow > 0 {
                // Update edge flow
                update_edge_flow(network, source, to, flow);
                return Some(flow);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_network_creation() {
        let mut network = FlowNetwork::new();
        network.add_edge(0, 1, 10);
        network.add_edge(0, 2, 10);
        network.add_edge(1, 2, 2);
        network.add_edge(1, 3, 4);
        network.add_edge(2, 3, 10);

        // Only vertices with outgoing edges are in the adjacency map
        assert_eq!(network.adj.len(), 3);
    }

    #[test]
    fn test_ford_fulkerson() {
        let mut network = FlowNetwork::new();
        // Simple graph: 0 -> 1 -> 2, with edge weights as capacities
        network.add_edge(0, 1, 10);
        network.add_edge(1, 2, 5);

        let flow = ford_fulkerson(&mut network, 0, 2);
        assert_eq!(flow, 5);
    }

    #[test]
    fn test_min_st_cut() {
        let mut network = FlowNetwork::new();
        network.add_edge(0, 1, 10);
        network.add_edge(0, 2, 10);
        network.add_edge(1, 2, 2);

        let flow = ford_fulkerson(&mut network, 0, 2);
        assert!(flow > 0);

        let cut = min_st_cut(&network, 0, 2);
        assert!(cut.contains(&0));
    }
}
