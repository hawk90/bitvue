//! Adjacency list and matrix representations.


extern crate alloc;

use alloc::vec::Vec;
use core::ops::Index;

use super::VertexId;

/// Adjacency list representation of a graph.
#[derive(Clone, Debug)]
pub struct AdjacencyList<T> {
    vertices: Vec<Vec<T>>,
}

impl<T: Clone> AdjacencyList<T> {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
        }
    }

    pub fn add_vertex(&mut self, neighbors: Vec<T>) -> VertexId {
        let id = self.vertices.len();
        self.vertices.push(neighbors);
        id
    }

    /// Returns the neighbors of a vertex.
    ///
    /// # Panics
    ///
    /// Panics if `vertex` is out of bounds.
    pub fn neighbors(&self, vertex: VertexId) -> &[T] {
        // SAFETY: We validate the vertex is within bounds before accessing
        if vertex >= self.vertices.len() {
            panic!(
                "Vertex index {} out of bounds (vertex count: {})",
                vertex,
                self.vertices.len()
            );
        }
        &self.vertices[vertex]
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}

impl<T: Clone> Default for AdjacencyList<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Adjacency matrix representation of a graph.
#[derive(Clone, Debug)]
pub struct AdjacencyMatrix {
    matrix: Vec<Vec<bool>>,
}

impl AdjacencyMatrix {
    pub fn new(size: usize) -> Self {
        Self {
            matrix: vec![vec![false; size]; size],
        }
    }

    pub fn with_capacity(size: usize) -> Self {
        Self::new(size)
    }

    pub fn add_edge(&mut self, from: VertexId, to: VertexId) {
        if from < self.matrix.len() && to < self.matrix.len() {
            self.matrix[from][to] = true;
        }
    }

    pub fn has_edge(&self, from: VertexId, to: VertexId) -> bool {
        if from < self.matrix.len() && to < self.matrix.len() {
            self.matrix[from][to]
        } else {
            false
        }
    }

    pub fn size(&self) -> usize {
        self.matrix.len()
    }
}

impl Index<(VertexId, VertexId)> for AdjacencyMatrix {
    type Output = bool;

    fn index(&self, index: (VertexId, VertexId)) -> &Self::Output {
        // SAFETY: We validate both indices are within bounds before accessing
        let (from, to) = index;
        if from >= self.matrix.len() {
            panic!(
                "Row index {} out of bounds (matrix size: {})",
                from,
                self.matrix.len()
            );
        }
        if to >= self.matrix[from].len() {
            panic!(
                "Column index {} out of bounds for row {} (row size: {})",
                to,
                from,
                self.matrix[from].len()
            );
        }
        &self.matrix[from][to]
    }
}

impl Default for AdjacencyMatrix {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjacency_list() {
        let mut list = AdjacencyList::new();
        let v1 = list.add_vertex(vec![1, 2, 3]);
        let neighbors = list.neighbors(v1);
        assert_eq!(neighbors.len(), 3);
    }

    #[test]
    fn test_adjacency_matrix() {
        let mut matrix = AdjacencyMatrix::new(3);
        matrix.add_edge(0, 1);
        assert!(matrix.has_edge(0, 1));
        assert!(!matrix.has_edge(1, 0));
    }

    #[test]
    fn test_adjacency_matrix_index() {
        let mut matrix = AdjacencyMatrix::new(3);
        matrix.add_edge(0, 1);
        assert_eq!(matrix[(0, 1)], true);
        assert_eq!(matrix[(1, 0)], false);
    }

    // Tests for MEDIUM security fix - bounds checking

    #[test]
    #[should_panic(expected = "Vertex index 5 out of bounds")]
    fn test_adjacency_list_neighbors_out_of_bounds() {
        let list = AdjacencyList::<i32>::new();
        // Accessing a vertex that doesn't exist should panic
        let _ = list.neighbors(5);
    }

    #[test]
    #[should_panic(expected = "Row index 5 out of bounds")]
    fn test_adjacency_matrix_index_row_out_of_bounds() {
        let matrix = AdjacencyMatrix::new(3);
        // Accessing a row that doesn't exist should panic
        let _ = matrix[(5, 0)];
    }

    #[test]
    #[should_panic(expected = "Column index 5 out of bounds")]
    fn test_adjacency_matrix_index_column_out_of_bounds() {
        let matrix = AdjacencyMatrix::new(3);
        // Accessing a column that doesn't exist should panic
        let _ = matrix[(0, 5)];
    }
}
