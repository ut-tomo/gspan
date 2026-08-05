use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt;

use thiserror::Error;

use crate::graph::{EdgeId, EdgeLabel, Graph, VertexId, VertexLabel};

//DFS traverseによる発見順
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DfsVertexId(pub usize);

//DFS codeの1 edge
/*
TODO Definition 2
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dfs5Tuple {
    pub from: DfsVertexId,
    pub to: DfsVertexId,
    pub from_label: VertexLabel,
    pub edge_label: EdgeLabel,
    pub to_label: VertexLabel,
}
impl Dfs5Tuple {
    pub fn is_forward(self) -> bool {
        self.from < self.to
    }
}

//DFS tupleの列としてDFS codeを定義
/*
TODO(canonical-order): Dfs5Tupleをedge列の辞書順で比較
*/
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct DfsCode {
    edges: Vec<Dfs5Tuple>,
}
impl DfsCode {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn edges(&self) -> &[Dfs5Tuple] {
        &self.edges
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    fn push(&mut self, edge: Dfs5Tuple) {
        self.edges.push(edge);
    }
    /// vertex数はid+1
    pub fn vertex_count(&self) -> usize {
        if self.edges.is_empty() {
            0
        } else {
            self.edges
                .iter()
                .flat_map(|edge| [edge.from.0, edge.to.0])
                .max()
                .map_or(0, |max_id| max_id + 1)
        }
    }
}
impl fmt::Display for DfsCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, edge) in self.edges.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(
                f,
                "({},{},{},{},{})",
                edge.from.0, edge.to.0, edge.from_label.0, edge.edge_label.0, edge.to_label.0,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MinDfsError {
    #[error("DFS code requires a connected graph")]
    DisconnectedGraph,
}

/*
現在のDFSコードの、元のグラフ上での埋め込み位置を復元する
graph_vertexは、DfsVertexId(DFS traverseによる発見順)とgraphのvertex idに対する対応。
extensionを作る際のhelper funcとして使用済みvertex, edgeも保存する。

このembeddingを複数保持し、各embeddingに対してrightmost pathからの1 edge growthを探索する形
*/
#[derive(Debug, Clone, PartialEq, Eq)]
struct Embedding {
    dfs_to_graph: Vec<VertexId>,
    used_edges: Vec<bool>,
}

impl Embedding {
    fn graph_vertex(&self, dfs_vertex: DfsVertexId) -> VertexId {
        self.dfs_to_graph[dfs_vertex.0]
    }

    fn used_edge(&self, edge_id: EdgeId) -> bool {
        self.used_edges[edge_id.0]
    }

    fn contains_vertex(&self, vertex_id: VertexId) -> bool {
        self.dfs_to_graph.contains(&vertex_id)
    }

    fn extend_forward(&self, edge_id: EdgeId, new_vertex: VertexId) -> Self {
        assert!(
            !self.used_edge(edge_id),
            "forward extension must use an unused edge"
        );
        assert!(
            !self.contains_vertex(new_vertex),
            "forward extension must discover a new vertex"
        );
        let mut next = self.clone();
        next.used_edges[edge_id.0] = true;
        next.dfs_to_graph.push(new_vertex);
        next
    }
    fn extend_backward(&self, edge_id: EdgeId) -> Self {
        assert!(
            !self.used_edge(edge_id),
            "backward extension must use an unused edge"
        );

        let mut next = self.clone();
        next.used_edges[edge_id.0] = true;
        next
    }
}

//一番最初のone edge graphを選ぶ

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct InitialEdgeKey {
    from_label: VertexLabel,
    edge_label: EdgeLabel,
    to_label: VertexLabel,
}

// TODO: initial candidateのgrouping
#[allow(dead_code)]
struct InitialGroup {
    dfs_edge: Dfs5Tuple,
    embeddings: Vec<Embedding>,
}

#[derive(Debug, Clone)]
struct Extension {
    key: ExtensionKey,
    dfs_edge: Dfs5Tuple,
    embedding: Embedding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtensionKey {
    Backward {
        to: DfsVertexId,
        edge_label: EdgeLabel,
    },
    Forward {
        from: DfsVertexId,
        edge_label: EdgeLabel,
        to_label: VertexLabel,
    },
}

// TODO: 辞書順の実装
impl Ord for ExtensionKey {
    fn cmp(&self, other: &Self) -> Ordering {
        use ExtensionKey::*;
        match (self, other) {
            (
                Backward {
                    to: self_to,
                    edge_label: self_label,
                },
                Backward {
                    to: other_to,
                    edge_label: other_label,
                },
            ) => self_to
                .cmp(other_to)
                .then_with(|| self_label.cmp(other_label)),

            (
                Forward {
                    from: self_from,
                    edge_label: self_edge,
                    to_label: self_to,
                },
                Forward {
                    from: other_from,
                    edge_label: other_edge,
                    to_label: other_to,
                },
            ) => other_from
                .cmp(self_from)
                .then_with(|| self_edge.cmp(other_edge))
                .then_with(|| self_to.cmp(other_to)),

            (Backward { .. }, Forward { .. }) => Ordering::Less,
            (Forward { .. }, Backward { .. }) => Ordering::Greater,
        }
    }
}

impl PartialOrd for ExtensionKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn other_endpoint(graph: &Graph, edge_id: EdgeId, vertex_id: VertexId) -> VertexId {
    let edge = &graph.edges()[edge_id.0];

    if edge.endpoints[0] == vertex_id {
        edge.endpoints[1]
    } else if edge.endpoints[1] == vertex_id {
        edge.endpoints[0]
    } else {
        panic!("vertex must be an endpoint of the edge")
    }
}

fn rightmost_path(code: &DfsCode) -> Vec<DfsVertexId> {
    let mut parent = vec![None; code.vertex_count()];

    for edge in code.edges() {
        if edge.is_forward() {
            parent[edge.to.0] = Some(edge.from);
        }
    }

    let mut current = DfsVertexId(code.vertex_count() - 1);
    let mut path = vec![current];

    while let Some(p) = parent[current.0] {
        path.push(p);
        current = p;
    }

    path.reverse();
    path
}

fn initial_candidates(graph: &Graph) -> Vec<(InitialEdgeKey, Dfs5Tuple, Embedding)> {
    let mut candidates = Vec::new();

    for edge_index in 0..graph.edge_count() {
        let edge_id = EdgeId(edge_index);
        let edge = &graph.edges()[edge_index];

        // 両方向保持
        for (from, to) in [
            (edge.endpoints[0], edge.endpoints[1]),
            (edge.endpoints[1], edge.endpoints[0]),
        ] {
            let from_label = graph.vertex_labels()[from.0];
            let to_label = graph.vertex_labels()[to.0];

            let key = InitialEdgeKey {
                from_label,
                edge_label: edge.label,
                to_label,
            };

            let dfs_edge = Dfs5Tuple {
                from: DfsVertexId(0),
                to: DfsVertexId(1),
                from_label,
                edge_label: edge.label,
                to_label,
            };

            let mut used_edges = vec![false; graph.edge_count()];
            used_edges[edge_id.0] = true;

            let embedding = Embedding {
                dfs_to_graph: vec![from, to],
                used_edges,
            };

            candidates.push((key, dfs_edge, embedding));
        }
    }

    candidates
}

fn collect_extensions(
    graph: &Graph,
    code: &DfsCode,
    embedding: &Embedding,
    rightmost_path: &[DfsVertexId],
) -> Vec<Extension> {
    let mut extensions = Vec::new();

    let rightmost_dfs = *rightmost_path
        .last()
        .expect("rightmost path must not be empty");

    let rightmost_graph = embedding.graph_vertex(rightmost_dfs);

    // backward extensions
    // TODO: ordering
    // rightmost vertexからrightmost path上の祖先へ戻るvalid childだけを列挙
    for &edge_id in graph.adjacency(rightmost_graph) {
        if embedding.used_edge(edge_id) {
            continue;
        }

        let target_graph = other_endpoint(graph, edge_id, rightmost_graph);

        let Some(target_dfs) = rightmost_path
            .iter()
            .copied()
            .find(|&dfs_vertex| embedding.graph_vertex(dfs_vertex) == target_graph)
        else {
            continue;
        };

        if target_dfs == rightmost_dfs {
            continue;
        }
        let edge = &graph.edges()[edge_id.0];

        extensions.push(Extension {
            key: ExtensionKey::Backward {
                to: target_dfs,
                edge_label: edge.label,
            },
            dfs_edge: Dfs5Tuple {
                from: rightmost_dfs,
                to: target_dfs,
                from_label: graph.vertex_labels()[rightmost_graph.0],
                edge_label: edge.label,
                to_label: graph.vertex_labels()[target_graph.0],
            },
            embedding: embedding.extend_backward(edge_id),
        });
    }

    // forward extensions
    // TODO: canonical orderingに従うminimum labelとrightmost pathのlabel条件を追加
    let new_dfs_vertex = DfsVertexId(code.vertex_count());

    for &from_dfs in rightmost_path.iter().rev() {
        let from_graph = embedding.graph_vertex(from_dfs);

        for &edge_id in graph.adjacency(from_graph) {
            if embedding.used_edge(edge_id) {
                continue;
            }

            let to_graph = other_endpoint(graph, edge_id, from_graph);

            if embedding.contains_vertex(to_graph) {
                continue;
            }

            let edge = &graph.edges()[edge_id.0];
            let to_label = graph.vertex_labels()[to_graph.0];

            extensions.push(Extension {
                key: ExtensionKey::Forward {
                    from: from_dfs,
                    edge_label: edge.label,
                    to_label,
                },
                dfs_edge: Dfs5Tuple {
                    from: from_dfs,
                    to: new_dfs_vertex,
                    from_label: graph.vertex_labels()[from_graph.0],
                    edge_label: edge.label,
                    to_label,
                },
                embedding: embedding.extend_forward(edge_id, to_graph),
            });
        }
    }

    extensions
}

pub fn minimum_dfs_code(graph: &Graph) -> Result<DfsCode, MinDfsError> {
    if !is_connected(graph) {
        return Err(MinDfsError::DisconnectedGraph);
    }

    if graph.edge_count() == 0 {
        return Ok(DfsCode::new());
    }

    let initial = initial_candidates(graph);

    let minimum_key = initial
        .iter()
        .map(|(key, _, _)| *key)
        .min()
        .expect("graph with edges must have an initial candidate");

    let mut initial_iter = initial
        .into_iter()
        .filter(|(key, _, _)| *key == minimum_key);

    let (_, first_edge, first_embedding) = initial_iter
        .next()
        .expect("minimum initial candidate must exist");

    let mut embeddings = vec![first_embedding];

    for (_, _, embedding) in initial_iter {
        embeddings.push(embedding);
    }

    let mut code = DfsCode::new();
    code.push(first_edge);

    // 同じminimum prefixを生成するembeddingをすべて残しながら1 edgeずつ拡張する。
    while code.edge_count() < graph.edge_count() {
        let path = rightmost_path(&code);

        let extensions: Vec<Extension> = embeddings
            .iter()
            .flat_map(|embedding| collect_extensions(graph, &code, embedding, &path))
            .collect();

        let minimum_key = extensions
            .iter()
            .map(|extension| extension.key)
            .min()
            .expect("connected graph with unused edges must have an extension");

        let mut minimum_extensions = extensions
            .into_iter()
            .filter(|extension| extension.key == minimum_key);

        let first = minimum_extensions
            .next()
            .expect("minimum extension must exist");

        code.push(first.dfs_edge);

        embeddings = vec![first.embedding];
        embeddings.extend(minimum_extensions.map(|extension| extension.embedding));
    }

    Ok(code)
}

fn is_connected(graph: &Graph) -> bool {
    if graph.vertex_count() == 0 {
        return true;
    }

    let mut visited = vec![false; graph.vertex_count()];
    let mut queue = VecDeque::new();
    visited[0] = true;
    queue.push_back(VertexId(0));

    while let Some(vertex) = queue.pop_front() {
        for &edge_id in graph.adjacency(vertex) {
            let neighbor = other_endpoint(graph, edge_id, vertex);

            if !visited[neighbor.0] {
                visited[neighbor.0] = true;
                queue.push_back(neighbor);
            }
        }
    }

    visited.into_iter().all(|visited| visited)
}
