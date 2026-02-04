//! Graph connectivity algorithms.

extern crate alloc;

use alloc::vec::Vec;

use super::{Graph, UndirectedGraph, VertexId};

/// Checks if an undirected graph is connected.
pub fn is_connected<T>(graph: &UndirectedGraph<T>) -> bool {
    if graph.vertex_count() == 0 {
        return true;
    }

    let visited = super::algorithms::bfs(graph.as_graph(), 0);
    visited.len() == graph.vertex_count()
}

/// Finds connected components in an undirected graph.
pub fn components<T>(graph: &UndirectedGraph<T>) -> Vec<Vec<VertexId>> {
    let mut visited = Vec::new();
    let mut result = Vec::new();

    for v in 0..graph.vertex_count() {
        if !visited.contains(&v) {
            let component = super::algorithms::bfs(graph.as_graph(), v);
            for &vertex in &component {
                visited.push(vertex);
            }
            result.push(component);
        }
    }

    result
}

/// Finds strongly connected components in a directed graph.
pub fn strongly_connected_components<T>(graph: &Graph<T>) -> Vec<Vec<VertexId>>
where
    T: Clone,
{
    // Kosaraju's algorithm
    let mut order = Vec::new();
    let mut visited = vec![false; graph.vertex_count()];

    // First DFS to get ordering
    for v in 0..graph.vertex_count() {
        if !visited[v] {
            dfs_order(graph, v, &mut visited, &mut order);
        }
    }

    // Transpose the graph
    let transposed = super::algorithms::transpose(graph);

    // Second DFS on transposed graph
    let mut visited2 = vec![false; transposed.vertex_count()];
    let mut sccs = Vec::new();

    for &v in order.iter().rev() {
        if !visited2[v] {
            let mut component = Vec::new();
            dfs_collect(&transposed, v, &mut visited2, &mut component);
            sccs.push(component);
        }
    }

    sccs
}

fn dfs_order<T>(graph: &Graph<T>, v: VertexId, visited: &mut [bool], order: &mut Vec<VertexId>) {
    visited[v] = true;
    for neighbor in graph.neighbors(v) {
        if !visited[neighbor.id] {
            dfs_order(graph, neighbor.id, visited, order);
        }
    }
    order.push(v);
}

fn dfs_collect<T>(
    graph: &Graph<T>,
    v: VertexId,
    visited: &mut [bool],
    component: &mut Vec<VertexId>,
) {
    visited[v] = true;
    component.push(v);
    for neighbor in graph.neighbors(v) {
        if !visited[neighbor.id] {
            dfs_collect(graph, neighbor.id, visited, component);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_connected() {
        let mut graph = UndirectedGraph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        graph.add_edge(v1, v2, None);

        assert!(is_connected(&graph));
    }

    #[test]
    fn test_components() {
        let mut graph = UndirectedGraph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);

        let comps = components(&graph);
        assert_eq!(comps.len(), 2);
    }

    #[test]
    fn test_strongly_connected_components() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);
        graph.add_edge(v3, v1, None);

        let sccs = strongly_connected_components(&graph);
        assert_eq!(sccs.len(), 1);
    }
}
