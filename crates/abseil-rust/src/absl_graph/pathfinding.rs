//! Advanced pathfinding algorithms.


extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use super::{Graph, VertexId};

/// A* search algorithm result.
pub struct AStarResult {
    pub path: Vec<VertexId>,
    pub cost: f64,
}

/// A* pathfinding algorithm with heuristic.
pub fn astar<T, F>(
    graph: &Graph<T>,
    start: VertexId,
    goal: VertexId,
    heuristic: F,
) -> Option<Vec<VertexId>>
where
    F: Fn(VertexId) -> f64,
{
    use alloc::collections::BinaryHeap;
    use core::cmp::Ordering;

    #[derive(Clone, Copy)]
    struct Node {
        vertex: VertexId,
        #[allow(dead_code)]
        g_score: f64,
        f_score: f64,
    }

    impl PartialEq for Node {
        fn eq(&self, other: &Self) -> bool {
            self.vertex == other.vertex
        }
    }

    impl Eq for Node {}

    impl PartialOrd for Node {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for Node {
        fn cmp(&self, other: &Self) -> Ordering {
            // Reverse ordering for min-heap behavior
            match self.f_score.partial_cmp(&other.f_score) {
                Some(Ordering::Equal) => {}
                Some(ord) => return ord.reverse(),
                None => return Ordering::Equal,
            }
            self.vertex.cmp(&other.vertex)
        }
    }

    let mut open_set = BinaryHeap::new();
    let mut came_from: Vec<Option<VertexId>> = vec![None; graph.vertex_count()];
    let mut g_score = vec![f64::INFINITY; graph.vertex_count()];
    let mut in_open_set = BTreeSet::new();

    g_score[start] = 0.0;
    open_set.push(Node {
        vertex: start,
        g_score: 0.0,
        f_score: heuristic(start),
    });
    in_open_set.insert(start);

    while let Some(current) = open_set.pop() {
        if current.vertex == goal {
            // Reconstruct path
            let mut path = vec![goal];
            let mut current = Some(goal);
            while let Some(v) = current {
                current = came_from[v];
                if let Some(prev) = current {
                    path.push(prev);
                }
            }
            path.reverse();
            return Some(path);
        }

        in_open_set.remove(&current.vertex);

        for edge in graph.outgoing_edges(current.vertex) {
            let tentative_g = g_score[current.vertex] + edge.weight.unwrap_or(1.0);

            if tentative_g < g_score[edge.to] {
                came_from[edge.to] = Some(current.vertex);
                g_score[edge.to] = tentative_g;
                let f_score = tentative_g + heuristic(edge.to);

                if !in_open_set.contains(&edge.to) {
                    open_set.push(Node {
                        vertex: edge.to,
                        g_score: tentative_g,
                        f_score,
                    });
                    in_open_set.insert(edge.to);
                }
            }
        }
    }

    None
}

/// Path finding trait.
pub trait PathFinder {
    fn find_path(&self, start: VertexId, goal: VertexId) -> Option<Vec<VertexId>>;
}

impl<T> PathFinder for Graph<T> {
    fn find_path(&self, start: VertexId, goal: VertexId) -> Option<Vec<VertexId>> {
        astar(self, start, goal, |_| 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_astar() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_weighted_edge(v1, v2, 5.0);
        graph.add_weighted_edge(v2, v3, 3.0);

        // Zero heuristic (same as Dijkstra)
        let path = astar(&graph, v1, v3, |_| 0.0);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec![v1, v2, v3]);
    }

    #[test]
    fn test_path_finder() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        graph.add_edge(v1, v2, None);

        let path = graph.find_path(v1, v2);
        assert!(path.is_some());
    }
}
