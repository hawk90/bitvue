//! Graph topology and structural analysis.

extern crate alloc;

use alloc::vec::Vec;

use super::{Graph, VertexId};

/// Topological sort result with additional utilities.
pub struct TopologicalSort {
    order: Vec<VertexId>,
}

impl TopologicalSort {
    pub fn new(order: Vec<VertexId>) -> Self {
        Self { order }
    }

    pub fn order(&self) -> &[VertexId] {
        &self.order
    }

    pub fn position_of(&self, vertex: VertexId) -> Option<usize> {
        self.order.iter().position(|&v| v == vertex)
    }

    pub fn comes_before(&self, a: VertexId, b: VertexId) -> bool {
        match (self.position_of(a), self.position_of(b)) {
            (Some(pos_a), Some(pos_b)) => pos_a < pos_b,
            _ => false,
        }
    }
}

/// Topological order trait.
pub trait TopologicalOrder {
    fn topological_order(&self) -> Option<TopologicalSort>;
}

impl<T> TopologicalOrder for Graph<T> {
    fn topological_order(&self) -> Option<TopologicalSort> {
        super::algorithms::topological_sort(self).map(TopologicalSort::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);

        let order = graph.topological_order();
        assert!(order.is_some());
        let topo = order.unwrap();
        assert!(topo.comes_before(v1, v2));
        assert!(topo.comes_before(v2, v3));
    }
}
