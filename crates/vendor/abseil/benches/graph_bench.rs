// Comprehensive benchmarks for graph algorithms
#![cfg(bench)]

#[cfg(test)]
use abseil::absl_graph::{bfs, dfs, dijkstra, topological_sort, Graph};
#[cfg(test)]
use test::{black_box, Bencher};

// Create a small graph (10 vertices, 15 edges)
fn small_graph() -> Graph<i32> {
    let mut graph = Graph::new();
    let vertices: Vec<_> = (0..10).map(|i| graph.add_vertex(i)).collect();

    for i in 0..8 {
        graph.add_edge(vertices[i], vertices[i + 1], Some(1.0));
        graph.add_edge(vertices[i], vertices[i + 2], Some(2.0));
    }

    graph
}

// Create a medium graph (100 vertices, ~300 edges)
fn medium_graph() -> Graph<i32> {
    let mut graph = Graph::new();
    let vertices: Vec<_> = (0..100).map(|i| graph.add_vertex(i)).collect();

    for i in 0..98 {
        graph.add_edge(vertices[i], vertices[i + 1], Some(1.0));
        if i < 97 {
            graph.add_edge(vertices[i], vertices[i + 2], Some(2.0));
        }
        if i < 95 {
            graph.add_edge(vertices[i], vertices[i + 5], Some(3.0));
        }
    }

    graph
}

// Create a large graph (1000 vertices, ~3000 edges)
fn large_graph() -> Graph<i32> {
    let mut graph = Graph::new();
    let vertices: Vec<_> = (0..1000).map(|i| graph.add_vertex(i)).collect();

    for i in 0..998 {
        graph.add_edge(vertices[i], vertices[i + 1], Some(1.0));
        if i < 997 {
            graph.add_edge(vertices[i], vertices[i + 2], Some(2.0));
        }
        if i < 995 {
            graph.add_edge(vertices[i], vertices[i + 5], Some(3.0));
        }
    }

    graph
}

// Create an extra large graph (5000 vertices, ~15000 edges)
fn xlarge_graph() -> Graph<i32> {
    let mut graph = Graph::new();
    let vertices: Vec<_> = (0..5000).map(|i| graph.add_vertex(i)).collect();

    for i in 0..4998 {
        graph.add_edge(vertices[i], vertices[i + 1], Some(1.0));
        if i < 4997 {
            graph.add_edge(vertices[i], vertices[i + 2], Some(2.0));
        }
        if i < 4995 {
            graph.add_edge(vertices[i], vertices[i + 5], Some(3.0));
        }
    }

    graph
}

// ========== Traversal Algorithms ==========

#[bench]
fn bench_bfs_small(b: &mut Bencher) {
    let graph = small_graph();
    b.iter(|| {
        black_box(bfs(black_box(&graph), black_box(0)));
    });
}

#[bench]
fn bench_bfs_medium(b: &mut Bencher) {
    let graph = medium_graph();
    b.iter(|| {
        black_box(bfs(black_box(&graph), black_box(0)));
    });
}

#[bench]
fn bench_bfs_large(b: &mut Bencher) {
    let graph = large_graph();
    b.iter(|| {
        black_box(bfs(black_box(&graph), black_box(0)));
    });
}

#[bench]
fn bench_bfs_xlarge(b: &mut Bencher) {
    let graph = xlarge_graph();
    b.iter(|| {
        black_box(bfs(black_box(&graph), black_box(0)));
    });
}

#[bench]
fn bench_dfs_small(b: &mut Bencher) {
    let graph = small_graph();
    b.iter(|| {
        black_box(dfs(black_box(&graph), black_box(0)));
    });
}

#[bench]
fn bench_dfs_medium(b: &mut Bencher) {
    let graph = medium_graph();
    b.iter(|| {
        black_box(dfs(black_box(&graph), black_box(0)));
    });
}

#[bench]
fn bench_dfs_large(b: &mut Bencher) {
    let graph = large_graph();
    b.iter(|| {
        black_box(dfs(black_box(&graph), black_box(0)));
    });
}

// ========== Shortest Path Algorithms ==========

#[bench]
fn bench_dijkstra_small(b: &mut Bencher) {
    let graph = small_graph();
    b.iter(|| {
        black_box(dijkstra(black_box(&graph), black_box(0), black_box(9)));
    });
}

#[bench]
fn bench_dijkstra_medium(b: &mut Bencher) {
    let graph = medium_graph();
    b.iter(|| {
        black_box(dijkstra(black_box(&graph), black_box(0), black_box(99)));
    });
}

#[bench]
fn bench_dijkstra_large(b: &mut Bencher) {
    let graph = large_graph();
    b.iter(|| {
        black_box(dijkstra(black_box(&graph), black_box(0), black_box(999)));
    });
}

// ========== Topological Sort ==========

#[bench]
fn bench_topological_sort_small(b: &mut Bencher) {
    let graph = small_graph();
    b.iter(|| {
        black_box(topological_sort(black_box(&graph)));
    });
}

#[bench]
fn bench_topological_sort_medium(b: &mut Bencher) {
    let graph = medium_graph();
    b.iter(|| {
        black_box(topological_sort(black_box(&graph)));
    });
}

#[bench]
fn bench_topological_sort_large(b: &mut Bencher) {
    let graph = large_graph();
    b.iter(|| {
        black_box(topological_sort(black_box(&graph)));
    });
}

// ========== Graph Construction ==========

#[bench]
fn bench_graph_add_vertex_small(b: &mut Bencher) {
    b.iter(|| {
        let mut graph = Graph::new();
        for i in 0..100 {
            black_box(graph.add_vertex(i));
        }
    });
}

#[bench]
fn bench_graph_add_vertex_medium(b: &mut Bencher) {
    b.iter(|| {
        let mut graph = Graph::new();
        for i in 0..1000 {
            black_box(graph.add_vertex(i));
        }
    });
}

#[bench]
fn bench_graph_add_edge_small(b: &mut Bencher) {
    b.iter(|| {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        for _ in 0..100 {
            black_box(graph.add_edge(v1, v2, Some(1.0)));
        }
    });
}

#[bench]
fn bench_graph_add_edge_medium(b: &mut Bencher) {
    b.iter(|| {
        let mut graph = Graph::new();
        let v1 = graph.add_vertex(1);
        let v2 = graph.add_vertex(2);
        for _ in 0..1000 {
            black_box(graph.add_edge(v1, v2, Some(1.0)));
        }
    });
}

#[bench]
fn bench_graph_construction_small(b: &mut Bencher) {
    b.iter(|| {
        let mut graph = Graph::new();
        let vertices: Vec<_> = (0..10).map(|i| graph.add_vertex(i)).collect();
        for i in 0..8 {
            graph.add_edge(vertices[i], vertices[i + 1], Some(1.0));
            graph.add_edge(vertices[i], vertices[i + 2], Some(2.0));
        }
        black_box(&graph);
    });
}

#[bench]
fn bench_graph_construction_medium(b: &mut Bencher) {
    b.iter(|| {
        let mut graph = Graph::new();
        let vertices: Vec<_> = (0..100).map(|i| graph.add_vertex(i)).collect();
        for i in 0..98 {
            graph.add_edge(vertices[i], vertices[i + 1], Some(1.0));
            if i < 97 {
                graph.add_edge(vertices[i], vertices[i + 2], Some(2.0));
            }
        }
        black_box(&graph);
    });
}

// ========== Graph Queries ==========

#[bench]
fn bench_graph_neighbors_small(b: &mut Bencher) {
    let graph = small_graph();
    b.iter(|| {
        black_box(graph.neighbors(black_box(0)).count());
    });
}

#[bench]
fn bench_graph_outgoing_edges_small(b: &mut Bencher) {
    let graph = small_graph();
    b.iter(|| {
        black_box(graph.outgoing_edges(black_box(0)).count());
    });
}

#[bench]
fn bench_graph_has_edge_small(b: &mut Bencher) {
    let graph = small_graph();
    b.iter(|| {
        black_box(graph.has_edge(black_box(0), black_box(1)));
    });
}

#[bench]
fn bench_graph_edge_weight_small(b: &mut Bencher) {
    let graph = small_graph();
    b.iter(|| {
        black_box(graph.edge_weight(black_box(0), black_box(1)));
    });
}
