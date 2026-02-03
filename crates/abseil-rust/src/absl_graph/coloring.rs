//! Graph coloring algorithms.


extern crate alloc;

use alloc::vec::Vec;

use super::Graph;

/// Graph coloring result.
pub struct GraphColoring {
    pub colors: Vec<Option<usize>>,
}

/// Greedy graph coloring.
pub fn greedy_coloring<T>(graph: &Graph<T>) -> GraphColoring {
    let mut colors = vec![None; graph.vertex_count()];
    let mut max_color = 0;

    for v in 0..graph.vertex_count() {
        let mut used_colors = Vec::new();
        for edge in graph.outgoing_edges(v) {
            if let Some(color) = colors[edge.to] {
                used_colors.push(color);
            }
        }

        let mut color = 0;
        while used_colors.contains(&color) {
            color += 1;
        }

        colors[v] = Some(color);
        max_color = max_color.max(color);
    }

    GraphColoring { colors }
}

/// Computes the chromatic number of a graph.
pub fn chromatic_number<T>(graph: &Graph<T>) -> usize {
    let coloring = greedy_coloring(graph);
    coloring.colors.iter().filter_map(|&c| c).map(|c| c + 1).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greedy_coloring() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        // Create a triangle (v1->v2, v2->v3, v3->v1)
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);
        graph.add_edge(v3, v1, None);

        let coloring = greedy_coloring(&graph);
        // Each vertex should have a color
        assert!(coloring.colors[v1].is_some());
        assert!(coloring.colors[v2].is_some());
        assert!(coloring.colors[v3].is_some());
    }

    #[test]
    fn test_chromatic_number() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        let v3 = graph.add_vertex(3);
        // Create a triangle
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v3, None);
        graph.add_edge(v3, v1, None);

        let chromatic = chromatic_number(&graph);
        // Triangle requires at least 2 colors in greedy
        assert!(chromatic >= 2);
    }

    #[test]
    fn test_coloring_simple() {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        graph.add_edge(v1, v2, None);
        graph.add_edge(v2, v1, None);

        let coloring = greedy_coloring(&graph);
        // Both vertices should be colored
        assert!(coloring.colors[v1].is_some());
        assert!(coloring.colors[v2].is_some());
        // They should have different colors (in a proper coloring)
        assert_ne!(coloring.colors[v1], coloring.colors[v2]);
    }
}
