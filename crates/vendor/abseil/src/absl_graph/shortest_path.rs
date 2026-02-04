//! Shortest path algorithms.


extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use super::{Graph, VertexId};

/// Dijkstra's shortest path algorithm.
pub fn dijkstra<T>(graph: &Graph<T>, start: VertexId, goal: VertexId) -> Option<Vec<VertexId>> {
    let mut distances = vec![f64::INFINITY; graph.vertex_count()];
    let mut previous: Vec<Option<VertexId>> = vec![None; graph.vertex_count()];
    let mut unvisited = BTreeSet::new();

    for v in 0..graph.vertex_count() {
        unvisited.insert(v);
    }

    distances[start] = 0.0;

    while !unvisited.is_empty() {
        let current = *unvisited
            .iter()
            .min_by_key(|&&v| {
                // Sort by distance, with f64 comparison trick
                distances[v].to_bits() as i64
            })?;

        if current == goal {
            break;
        }

        unvisited.remove(&current);

        for edge in graph.outgoing_edges(current) {
            if !unvisited.contains(&edge.to) {
                continue;
            }

            let weight = edge.weight.unwrap_or(1.0);
            let alt = distances[current] + weight;

            if alt < distances[edge.to] {
                distances[edge.to] = alt;
                previous[edge.to] = Some(current);
            }
        }
    }

    // Reconstruct path
    if distances[goal] == f64::INFINITY {
        return None;
    }

    let mut path = Vec::new();
    let mut current = Some(goal);

    while let Some(v) = current {
        path.push(v);
        current = previous[v];
    }

    path.reverse();
    Some(path)
}

/// Bellman-Ford shortest path algorithm (handles negative weights).
pub fn bellman_ford<T>(graph: &Graph<T>, start: VertexId) -> Option<Vec<f64>> {
    let n = graph.vertex_count();
    let mut distances = vec![f64::INFINITY; n];
    distances[start] = 0.0;

    // Relax edges n-1 times
    for _ in 0..n - 1 {
        for v in 0..n {
            for edge in graph.outgoing_edges(v) {
                let weight = edge.weight.unwrap_or(1.0);
                if distances[v] + weight < distances[edge.to] {
                    distances[edge.to] = distances[v] + weight;
                }
            }
        }
    }

    // Check for negative cycles
    for v in 0..n {
        for edge in graph.outgoing_edges(v) {
            let weight = edge.weight.unwrap_or(1.0);
            if distances[v] + weight < distances[edge.to] {
                return None; // Negative cycle detected
            }
        }
    }

    Some(distances)
}

/// Floyd-Warshall all-pairs shortest path algorithm.
pub fn floyd_warshall<T>(graph: &Graph<T>) -> Vec<Vec<f64>> {
    let n = graph.vertex_count();
    let mut dist = vec![vec![f64::INFINITY; n]; n];

    // Initialize distances
    for i in 0..n {
        dist[i][i] = 0.0;
        for edge in graph.outgoing_edges(i) {
            dist[i][edge.to] = edge.weight.unwrap_or(1.0);
        }
    }

    // Floyd-Warshall algorithm
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if dist[i][k] + dist[k][j] < dist[i][j] {
                    dist[i][j] = dist[i][k] + dist[k][j];
                }
            }
        }
    }

    dist
}

/// Generic shortest path function that uses Dijkstra by default.
pub fn shortest_path<T>(graph: &Graph<T>, start: VertexId, goal: VertexId) -> Option<Vec<VertexId>> {
    dijkstra(graph, start, goal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dijkstra() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_weighted_edge(v1, v2, 5.0);
        graph.add_weighted_edge(v2, v3, 3.0);

        let path = dijkstra(&graph, v1, v3);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec![v1, v2, v3]);
    }

    #[test]
    fn test_bellman_ford() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_weighted_edge(v1, v2, 5.0);
        graph.add_weighted_edge(v2, v3, 3.0);

        let distances = bellman_ford(&graph, v1);
        assert!(distances.is_some());
        let d = distances.unwrap();
        assert_eq!(d[v1], 0.0);
        assert_eq!(d[v2], 5.0);
        assert_eq!(d[v3], 8.0);
    }

    #[test]
    fn test_floyd_warshall() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_weighted_edge(v1, v2, 5.0);
        graph.add_weighted_edge(v2, v3, 3.0);

        let dist = floyd_warshall(&graph);
        assert_eq!(dist[v1][v1], 0.0);
        assert_eq!(dist[v1][v2], 5.0);
        assert_eq!(dist[v1][v3], 8.0);
    }
}
