//! Core graph algorithms.


extern crate alloc;

use alloc::collections::{BTreeSet, VecDeque};
use alloc::vec::Vec;

use super::{Graph, VertexId};

/// Performs depth-first search starting from a vertex.
///
/// Returns a vector of vertex IDs in the order they were visited.
///
/// # Examples
///
/// ```
/// use abseil::absl_graph::{Graph, algorithms::dfs};
///
/// let mut graph = Graph::new();
/// let v1 = graph.add_vertex(1);
/// let v2 = graph.add_vertex(2);
/// let v3 = graph.add_vertex(3);
/// graph.add_edge(v1, v2, None);
/// graph.add_edge(v2, v3, None);
///
/// let visited = dfs(&graph, v1);
/// assert!(visited.contains(&v1));
/// assert!(visited.contains(&v2));
/// assert!(visited.contains(&v3));
/// ```
pub fn dfs<T>(graph: &Graph<T>, start: VertexId) -> Vec<VertexId> {
    let mut visited = Vec::new();
    let mut seen = BTreeSet::new();
    dfs_recursive(graph, start, &mut visited, &mut seen);
    visited
}

fn dfs_recursive<T>(
    graph: &Graph<T>,
    current: VertexId,
    visited: &mut Vec<VertexId>,
    seen: &mut BTreeSet<VertexId>,
) {
    if !seen.insert(current) {
        return;
    }
    visited.push(current);

    for neighbor in graph.neighbors(current) {
        dfs_recursive(graph, neighbor.id, visited, seen);
    }
}

/// Performs iterative depth-first search.
pub fn dfs_iterative<T>(graph: &Graph<T>, start: VertexId) -> Vec<VertexId> {
    let mut visited = Vec::new();
    let mut seen = BTreeSet::new();
    let mut stack = vec![start];

    while let Some(current) = stack.pop() {
        if !seen.insert(current) {
            continue;
        }
        visited.push(current);

        // Push neighbors in reverse order to maintain natural order
        let neighbors: Vec<_> = graph.neighbors(current).map(|v| v.id).collect();
        for neighbor in neighbors.into_iter().rev() {
            if !seen.contains(&neighbor) {
                stack.push(neighbor);
            }
        }
    }

    visited
}

/// Performs breadth-first search starting from a vertex.
///
/// Returns a vector of vertex IDs in BFS order.
///
/// # Examples
///
/// ```
/// use abseil::absl_graph::{Graph, algorithms::bfs};
///
/// let mut graph = Graph::new();
/// let v1 = graph.add_vertex(1);
/// let v2 = graph.add_vertex(2);
/// let v3 = graph.add_vertex(3);
/// graph.add_edge(v1, v2, None);
/// graph.add_edge(v2, v3, None);
///
/// let visited = bfs(&graph, v1);
/// assert_eq!(visited[0], v1);
/// assert_eq!(visited[1], v2);
/// assert_eq!(visited[2], v3);
/// ```
pub fn bfs<T>(graph: &Graph<T>, start: VertexId) -> Vec<VertexId> {
    let mut visited = Vec::new();
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        if !seen.insert(current) {
            continue;
        }
        visited.push(current);

        for neighbor in graph.neighbors(current) {
            if !seen.contains(&neighbor.id) {
                queue.push_back(neighbor.id);
            }
        }
    }

    visited
}

/// Performs topological sort on a directed acyclic graph.
///
/// Returns vertices in topological order, or None if the graph contains a cycle.
///
/// # Examples
///
/// ```
/// use abseil::absl_graph::{Graph, algorithms::topological_sort};
///
/// let mut graph = Graph::new();
/// let v1 = graph.add_vertex(1);
/// let v2 = graph.add_vertex(2);
/// let v3 = graph.add_vertex(3);
/// graph.add_edge(v1, v2, None);
/// graph.add_edge(v2, v3, None);
///
/// let sorted = topological_sort(&graph);
/// assert!(sorted.is_some());
/// let order = sorted.unwrap();
/// assert!(order.iter().position(|&v| v == v1) < order.iter().position(|&v| v == v2));
/// ```
pub fn topological_sort<T>(graph: &Graph<T>) -> Option<Vec<VertexId>> {
    let n = graph.vertex_count();
    let mut in_degree = vec![0; n];

    // Calculate in-degrees
    for v in 0..n {
        for edge in graph.outgoing_edges(v) {
            in_degree[edge.to] += 1;
        }
    }

    // Find all vertices with in-degree 0
    let mut queue: VecDeque<VertexId> = in_degree
        .iter()
        .enumerate()
        .filter_map(|(i, &d)| if d == 0 { Some(i) } else { None })
        .collect();

    let mut result = Vec::with_capacity(n);

    while let Some(current) = queue.pop_front() {
        result.push(current);

        for neighbor in graph.neighbors(current) {
            in_degree[neighbor.id] -= 1;
            if in_degree[neighbor.id] == 0 {
                queue.push_back(neighbor.id);
            }
        }
    }

    if result.len() != n {
        None // Cycle detected
    } else {
        Some(result)
    }
}

/// Detects if a graph contains a cycle.
pub fn has_cycle<T>(graph: &Graph<T>) -> bool {
    topological_sort(graph).is_none()
}

/// Computes the transpose of a directed graph (reverses all edges).
pub fn transpose<T>(graph: &Graph<T>) -> Graph<T>
where
    T: Clone,
{
    let mut result = Graph::new();

    // Clone all vertices
    for vertex in 0..graph.vertex_count() {
        if let Some(v) = graph.vertex(vertex) {
            result.add_vertex(v.data.clone());
        }
    }

    // Add reversed edges
    for v in 0..graph.vertex_count() {
        for edge in graph.outgoing_edges(v) {
            result.add_edge(edge.to, edge.from, edge.weight);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfs() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);

        let visited = dfs(&graph, v1);
        assert_eq!(visited.len(), 3);
        assert!(visited.contains(&v1));
        assert!(visited.contains(&v2));
        assert!(visited.contains(&v3));
    }

    #[test]
    fn test_dfs_iterative() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v1, v3, None);

        let visited = dfs_iterative(&graph, v1);
        assert_eq!(visited.len(), 3);
        assert_eq!(visited[0], v1);
    }

    #[test]
    fn test_bfs() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);

        let visited = bfs(&graph, v1);
        assert_eq!(visited, vec![v1, v2, v3]);
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);

        let sorted = topological_sort(&graph);
        assert!(sorted.is_some());
        let order = sorted.unwrap();
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn test_topological_sort_cycle() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);
        graph.add_edge(v3, v1, None); // Creates a cycle

        let sorted = topological_sort(&graph);
        assert!(sorted.is_none());
    }

    #[test]
    fn test_has_cycle() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);
        graph.add_edge(v3, v1, None);

        assert!(has_cycle(&graph));
    }

    #[test]
    fn test_transpose() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        graph.add_edge(v1, v2, None);

        let transposed = transpose(&graph);
        assert!(transposed.has_edge(v2, v1));
        assert!(!transposed.has_edge(v1, v2));
    }
}
