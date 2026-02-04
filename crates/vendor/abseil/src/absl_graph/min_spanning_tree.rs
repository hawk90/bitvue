//! Minimum spanning tree algorithms.


extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use super::{UndirectedGraph, VertexId};

/// Minimum spanning tree edge.
#[derive(Clone, Debug)]
pub struct MSTEdge {
    pub from: VertexId,
    pub to: VertexId,
    pub weight: f64,
}

/// Minimum spanning tree result.
pub struct MinimumSpanningTree {
    pub edges: Vec<MSTEdge>,
    pub total_weight: f64,
}

/// Kruskal's algorithm for minimum spanning tree.
pub fn kruskal<T>(graph: &UndirectedGraph<T>) -> Option<MinimumSpanningTree> {
    let mut edges = Vec::new();
    let mut total_weight = 0.0;

    // Collect all edges
    for v in 0..graph.as_graph().vertex_count() {
        for edge in graph.as_graph().outgoing_edges(v) {
            if edge.from < edge.to { // Avoid duplicates
                edges.push(MSTEdge {
                    from: edge.from,
                    to: edge.to,
                    weight: edge.weight.unwrap_or(1.0),
                });
            }
        }
    }

    // Sort by weight
    edges.sort_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap());

    // Union-Find
    let mut parent: Vec<VertexId> = (0..graph.as_graph().vertex_count()).collect();

    let mut mst_edges = Vec::new();
    for edge in edges {
        let root_from = find(&parent, edge.from);
        let root_to = find(&parent, edge.to);

        if root_from != root_to {
            parent[root_from] = root_to;
            mst_edges.push(edge.clone());
            total_weight += edge.weight;
        }
    }

    Some(MinimumSpanningTree {
        edges: mst_edges,
        total_weight,
    })
}

fn find(parent: &[VertexId], x: VertexId) -> VertexId {
    if parent[x] != x {
        // Path compression (simplified)
        find(parent, parent[x])
    } else {
        x
    }
}

/// Prim's algorithm for minimum spanning tree.
pub fn prim<T>(graph: &UndirectedGraph<T>) -> Option<MinimumSpanningTree> {
    if graph.as_graph().vertex_count() == 0 {
        return None;
    }

    let mut in_mst = BTreeSet::new();
    let mut mst_edges = Vec::new();
    let mut total_weight = 0.0;

    let start = 0;
    in_mst.insert(start);

    while in_mst.len() < graph.as_graph().vertex_count() {
        let mut min_edge: Option<MSTEdge> = None;

        for &v in &in_mst {
            for edge in graph.as_graph().outgoing_edges(v) {
                if !in_mst.contains(&edge.to) {
                    let weight = edge.weight.unwrap_or(1.0);
                    match &min_edge {
                        None => min_edge = Some(MSTEdge { from: edge.from, to: edge.to, weight }),
                        Some(min) if weight < min.weight => {
                            min_edge = Some(MSTEdge { from: edge.from, to: edge.to, weight });
                        }
                        _ => {}
                    }
                }
            }
        }

        match min_edge {
            Some(edge) => {
                in_mst.insert(edge.to);
                mst_edges.push(edge.clone());
                total_weight += edge.weight;
            }
            None => break, // Graph is disconnected
        }
    }

    Some(MinimumSpanningTree {
        edges: mst_edges,
        total_weight,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kruskal() {
        let mut graph = UndirectedGraph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, Some(1.0));
        graph.add_edge(v2, v3, Some(2.0));
        graph.add_edge(v1, v3, Some(5.0));

        let mst = kruskal(&graph);
        assert!(mst.is_some());
        let mst = mst.unwrap();
        assert_eq!(mst.edges.len(), 2);
        assert_eq!(mst.total_weight, 3.0);
    }

    #[test]
    fn test_prim() {
        let mut graph = UndirectedGraph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, Some(1.0));
        graph.add_edge(v2, v3, Some(2.0));

        let mst = prim(&graph);
        assert!(mst.is_some());
    }
}
