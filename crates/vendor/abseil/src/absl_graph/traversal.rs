//! Graph traversal iterators.

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::{Graph, VertexId};

/// Depth-first traversal iterator.
pub struct DFSIterator<'a, T> {
    graph: &'a Graph<T>,
    stack: Vec<VertexId>,
    visited: Vec<bool>,
}

impl<'a, T> DFSIterator<'a, T> {
    pub fn new(graph: &'a Graph<T>, start: VertexId) -> Self {
        let visited = vec![false; graph.vertex_count()];
        Self {
            graph,
            stack: vec![start],
            visited,
        }
    }
}

impl<'a, T> Iterator for DFSIterator<'a, T> {
    type Item = VertexId;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current) = self.stack.pop() {
            if self.visited[current] {
                continue;
            }
            self.visited[current] = true;

            for neighbor in self.graph.neighbors(current) {
                if !self.visited[neighbor.id] {
                    self.stack.push(neighbor.id);
                }
            }

            return Some(current);
        }
        None
    }
}

/// Breadth-first traversal iterator.
pub struct BFSIterator<'a, T> {
    graph: &'a Graph<T>,
    queue: VecDeque<VertexId>,
    visited: Vec<bool>,
}

impl<'a, T> BFSIterator<'a, T> {
    pub fn new(graph: &'a Graph<T>, start: VertexId) -> Self {
        let visited = vec![false; graph.vertex_count()];
        let mut queue = VecDeque::new();
        queue.push_back(start);
        Self {
            graph,
            queue,
            visited,
        }
    }
}

impl<'a, T> Iterator for BFSIterator<'a, T> {
    type Item = VertexId;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current) = self.queue.pop_front() {
            if self.visited[current] {
                continue;
            }
            self.visited[current] = true;

            for neighbor in self.graph.neighbors(current) {
                if !self.visited[neighbor.id] {
                    self.queue.push_back(neighbor.id);
                }
            }

            return Some(current);
        }
        None
    }
}

/// Depth-first traversal trait.
pub trait DepthFirst {
    fn dfs_from(&self, start: VertexId) -> DFSIterator<'_, Self::GraphType>
    where
        Self: Sized;
    type GraphType;
}

/// Breadth-first traversal trait.
pub trait BreadthFirst {
    fn bfs_from(&self, start: VertexId) -> BFSIterator<'_, Self::GraphType>
    where
        Self: Sized;
    type GraphType;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfs_iterator() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);

        let visited: Vec<_> = DFSIterator::new(&graph, v1).collect();
        assert_eq!(visited.len(), 3);
    }

    #[test]
    fn test_bfs_iterator() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);

        let visited: Vec<_> = BFSIterator::new(&graph, v1).collect();
        assert_eq!(visited.len(), 3);
        assert_eq!(visited, vec![v1, v2, v3]);
    }
}
