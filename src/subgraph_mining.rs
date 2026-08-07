use crate::graph::{EdgeId, Graph, VertexId};
use crate::graph_to_mindfs::{
    Dfs5Tuple, DfsCode, DfsVertexId, Embedding, collect_extensions, minimum_dfs_code,
    rightmost_path,
};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrequentPattern {
    pub code: DfsCode,
    pub support: usize,
}

#[derive(Debug, Clone)]
struct Occurrence {
    graph_id: usize,
    embedding: Embedding,
}

pub fn subgraph_mining(graphs: &[Graph], min_support: usize) -> Vec<FrequentPattern> {
    let roots = initial_occurrences(graphs);
    let mut patterns = Vec::new();

    for (root_edge, occurrences) in roots {
        if projected_support(&occurrences) < min_support {
            continue;
        }
        let root_graph = graph_from_dfs_edges(&[root_edge]);

        let root_code = minimum_dfs_code(&root_graph).expect("a one-edge graph must be connected");

        mine_projected(graphs, root_code, occurrences, min_support, &mut patterns);
    }
    patterns
}

fn initial_occurrences(graphs: &[Graph]) -> BTreeMap<Dfs5Tuple, Vec<Occurrence>> {
    let mut roots = BTreeMap::new();

    for (graph_id, graph) in graphs.iter().enumerate() {
        for (edge_index, edge) in graph.edges().iter().enumerate() {
            let edge_id = EdgeId(edge_index);

            let orientations = [
                (edge.endpoints[0], edge.endpoints[1]),
                (edge.endpoints[1], edge.endpoints[0]),
            ];

            let candidates = orientations.map(|(from, to)| Dfs5Tuple {
                from: DfsVertexId(0),
                to: DfsVertexId(1),
                from_label: graph.vertex_labels()[from.0],
                edge_label: edge.label,
                to_label: graph.vertex_labels()[to.0],
            });

            let minimum_edge = *candidates
                .iter()
                .min()
                .expect("an edge must have two orientations");

            // endpoint labelsが同じなら両方向を残す
            for ((from, to), candidate) in orientations.into_iter().zip(candidates) {
                if candidate != minimum_edge {
                    continue;
                }

                let mut used_edges = vec![false; graph.edge_count()];

                used_edges[edge_id.0] = true;

                roots
                    .entry(minimum_edge)
                    .or_insert_with(Vec::new)
                    .push(Occurrence {
                        graph_id,
                        embedding: Embedding {
                            dfs_to_graph: vec![from, to],
                            used_edges,
                        },
                    });
            }
        }
    }

    roots
}

fn mine_projected(
    graphs: &[Graph],
    code: DfsCode,
    occurrences: Vec<Occurrence>,
    min_support: usize,
    patterns: &mut Vec<FrequentPattern>,
) {
    let support = projected_support(&occurrences);

    if support < min_support {
        return;
    }

    patterns.push(FrequentPattern {
        code: code.clone(),
        support,
    });

    let children = enumerate_children(graphs, &code, &occurrences);

    for (next_edge, next_occurrences) in children {
        if projected_support(&next_occurrences) < min_support {
            continue;
        }

        let mut candidate_edges = code.edges().to_vec();
        candidate_edges.push(next_edge);

        // 現在のDFS codeが表現するpatternをGraphへ戻す。
        let candidate_graph = graph_from_dfs_edges(&candidate_edges);

        let canonical_code = minimum_dfs_code(&candidate_graph)
            .expect("a rightmost extension must remain connected");

        // minimum DFS codeでないbranchをcanonical pruning。
        if canonical_code.edges() != candidate_edges {
            continue;
        }

        mine_projected(
            graphs,
            canonical_code,
            next_occurrences,
            min_support,
            patterns,
        );
    }
}

/// 論文のenumerateに相当。
///
/// 現在のpatternの全Occurrenceを1 edgeずつ拡張し、
/// 同じchild DFS edgeごとにまとめる。
fn enumerate_children(
    graphs: &[Graph],
    code: &DfsCode,
    occurrences: &[Occurrence],
) -> BTreeMap<Dfs5Tuple, Vec<Occurrence>> {
    let path = rightmost_path(code);
    let mut children = BTreeMap::new();

    for occurrence in occurrences {
        let graph = &graphs[occurrence.graph_id];

        for extension in collect_extensions(graph, code, &occurrence.embedding, &path) {
            children
                .entry(extension.dfs_edge)
                .or_insert_with(Vec::new)
                .push(Occurrence {
                    graph_id: occurrence.graph_id,
                    embedding: extension.embedding,
                });
        }
    }

    children
}

/// 同じGraph内に複数Embeddingがあってもsupportには1回だけ数える。
fn projected_support(occurrences: &[Occurrence]) -> usize {
    occurrences
        .iter()
        .map(|occurrence| occurrence.graph_id)
        .collect::<HashSet<_>>()
        .len()
}

fn graph_from_dfs_edges(edges: &[Dfs5Tuple]) -> Graph {
    let vertex_count = edges
        .iter()
        .flat_map(|edge| [edge.from.0, edge.to.0])
        .max()
        .map_or(0, |maximum| maximum + 1);

    let mut labels = vec![None; vertex_count];

    for edge in edges {
        assign_vertex_label(&mut labels[edge.from.0], edge.from_label);

        assign_vertex_label(&mut labels[edge.to.0], edge.to_label);
    }

    let mut graph = Graph::new();

    for label in labels {
        graph.add_vertex(label.expect("every DFS vertex must have a label"));
    }

    for edge in edges {
        graph.add_edge(VertexId(edge.from.0), VertexId(edge.to.0), edge.edge_label);
    }

    graph
}

fn assign_vertex_label(
    slot: &mut Option<crate::graph::VertexLabel>,
    label: crate::graph::VertexLabel,
) {
    if let Some(existing) = slot {
        assert_eq!(*existing, label, "a DFS vertex must have one label");
    } else {
        *slot = Some(label);
    }
}
