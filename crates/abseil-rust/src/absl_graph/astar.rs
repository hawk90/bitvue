//! A* search algorithm implementation (simplified).
//!
//! A* is a graph traversal and path search algorithm that uses heuristics
//! to find the shortest path between nodes.


extern crate alloc;

use alloc::vec::Vec;

use crate::absl_graph::{Graph, VertexId};

/// Simple A* node for pathfinding.
#[derive(Clone, Debug)]
struct AStarNode {
    vertex: VertexId,
    g_cost: u64,
    f_cost: u64,
    parent: Option<usize>,
}

/// A* search algorithm - finds shortest path using heuristics.
///
/// Simplified version that works without complex data structures.
///
/// # Returns
///
/// Vector of VertexIds representing the path from start to goal, or None if no path exists.
pub fn astar_search<T, F, H>(
    graph: &Graph<T>,
    start: VertexId,
    goal: VertexId,
    mut heuristic: H,
    mut edge_cost: F,
) -> Option<Vec<VertexId>>
where
    F: FnMut(VertexId, VertexId) -> Option<u64>,
    H: FnMut(VertexId, VertexId) -> u64,
{
    if start == goal {
        return Some(vec![start]);
    }

    let h_cost = heuristic(start, goal);
    let mut nodes = vec![AStarNode {
        vertex: start,
        g_cost: 0,
        f_cost: h_cost,
        parent: None,
    }];

    loop {
        // Find node with lowest f_cost in open set
        let current_idx = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.vertex != goal) // Not yet reached goal
            .min_by_key(|(_, n)| n.f_cost)?;

        let current_idx = current_idx.0;
        let current_vertex = nodes[current_idx].vertex;

        if current_vertex == goal {
            // Reconstruct path
            let mut path = Vec::new();
            let mut idx = current_idx;

            loop {
                path.push(nodes[idx].vertex);
                match nodes[idx].parent {
                    Some(parent_idx) => idx = parent_idx,
                    None => {
                        path.reverse();
                        return Some(path);
                    }
                }
            }
        }

        // Explore neighbors
        let neighbors = neighbors_id(graph, current_vertex);
        for neighbor in neighbors {
            // Check if already in nodes
            if nodes.iter().any(|n| n.vertex == neighbor) {
                continue;
            }

            let Some(edge_c) = edge_cost(current_vertex, neighbor) else {
                continue;
            };

            let tentative_g = nodes[current_idx].g_cost + edge_c;
            let h = heuristic(neighbor, goal);
            let f = tentative_g + h;

            nodes.push(AStarNode {
                vertex: neighbor,
                g_cost: tentative_g,
                f_cost: f,
                parent: Some(current_idx),
            });
        }

        // No path found
        if nodes.len() > graph.vertex_count() * 2 {
            break;
        }
    }

    None
}

/// Returns neighbors as VertexIds for pathfinding.
pub fn neighbors_id<T>(graph: &Graph<T>, vertex: VertexId) -> Vec<VertexId> {
    let mut neighbors = Vec::new();
    for edge in graph.outgoing_edges(vertex) {
        neighbors.push(edge.to);
    }
    neighbors
}

/// Manhattan distance heuristic for 2D grid positions.
pub fn manhattan_distance(a: (i32, i32), b: (i32, i32)) -> u64 {
    ((a.0 - b.0).abs() + (a.1 - b.1).abs()) as u64
}

/// Euclidean distance heuristic for 2D positions.
pub fn euclidean_distance(a: (i32, i32), b: (i32, i32)) -> u64 {
    let dx = (a.0 - b.0) as f64;
    let dy = (a.1 - b.1) as f64;
    (dx * dx + dy * dy).sqrt() as u64
}

/// Chebyshev distance heuristic for 2D grid positions (allows 8-directional movement).
pub fn chebyshev_distance(a: (i32, i32), b: (i32, i32)) -> u64 {
    ((a.0 - b.0).abs()).max((a.1 - b.1).abs()) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manhattan_distance() {
        assert_eq!(manhattan_distance((0, 0), (3, 4)), 7);
        assert_eq!(manhattan_distance((10, 10), (10, 10)), 0);
    }

    #[test]
    fn test_euclidean_distance() {
        assert_eq!(euclidean_distance((0, 0), (3, 4)), 5);
        assert_eq!(euclidean_distance((10, 10), (10, 10)), 0);
    }

    #[test]
    fn test_chebyshev_distance() {
        assert_eq!(chebyshev_distance((0, 0), (3, 4)), 4);
        assert_eq!(chebyshev_distance((10, 10), (10, 10)), 0);
    }
}
