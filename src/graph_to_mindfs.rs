use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt;

use thiserror::Error;

use crate::graph::{EdgeId, EdgeLabel, Graph, VertexId, VertexLabel};

//DFS traverseによる発見順
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DfsVertexId(pub usize);

//DFS codeの1 edge
/// DFS上の両端IDと、両端および辺のlabelを持つ5-tuple
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dfs5Tuple {
    pub from: DfsVertexId,
    pub to: DfsVertexId,
    pub from_label: VertexLabel,
    pub edge_label: EdgeLabel,
    pub to_label: VertexLabel,
}
impl Dfs5Tuple {
    /// DFSで新しいvertexを発見するtree edgeならforward、既発見vertexへ戻るならbackward
    pub fn is_forward(self) -> bool {
        self.from < self.to
    }
}
impl Ord for Dfs5Tuple {
    fn cmp(&self, other: &Self) -> Ordering {
        // gSpanのedge comparisonを位置関係とlabelの比較に分けて実装する
        let position_order = match (self.is_forward(), other.is_forward()) {
            // backward extensionの方が先
            (false, true) => Ordering::Less,    // backward < forward
            (true, false) => Ordering::Greater, // forward > backward

            // backward同士
            // backwardはrightmost vertexのみから出る
            // より小さいDFS vertex idに戻る
            // まず toを比較->from->label
            (false, false) => self
                .to
                .cmp(&other.to)
                .then_with(|| self.from.cmp(&other.from)),

            // forward 同士
            // rightmost vertexに近いところから伸びている方が早い
            // from label -> edge label -> to label の順で比較
            (true, true) => other
                .from
                .cmp(&self.from)
                .then_with(|| self.to.cmp(&other.to)),
        };

        // positionが同じときは3 labelを比較し、Dfs5Tuple全体として全順序にする。
        position_order.then_with(|| {
            (self.from_label, self.edge_label, self.to_label).cmp(&(
                other.from_label,
                other.edge_label,
                other.to_label,
            ))
        })
    }
}
impl PartialOrd for Dfs5Tuple {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

//DFS tupleの列としてDFS codeを定義
/// graphをDFS edgeの列として表したcode。minimum codeはgraphのcanonical labelになる。
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

    pub(crate) fn push(&mut self, edge: Dfs5Tuple) {
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
pub(crate) struct Embedding {
    pub(crate) dfs_to_graph: Vec<VertexId>,
    pub(crate) used_edges: Vec<bool>,
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

// 1つの候補は、追加するDFS edgeと、そのedgeを生成したgraph上のembeddingを対で持つ。
#[derive(Debug, Clone)]
pub(crate) struct Extension {
    pub(crate) dfs_edge: Dfs5Tuple,
    pub(crate) embedding: Embedding,
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

/// vertexを発見したtree edgeをDFS vertex idで引けるようにする。backward edgeを持つidはNone。
fn tree_edges(code: &DfsCode) -> Vec<Option<Dfs5Tuple>> {
    let mut edges = vec![None; code.vertex_count()];

    for edge in code.edges() {
        if edge.is_forward() {
            edges[edge.to.0] = Some(*edge);
        }
    }

    edges
}

/// forward edgeから親関係を復元し、rootからrightmost vertexまでのDFS vertex列を返す。
pub(crate) fn rightmost_path(code: &DfsCode) -> Vec<DfsVertexId> {
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

/// 全graph edgeを両方向から見た1-edge DFS codeと、その初期embeddingを列挙する。
/// 同じ最小edgeを持つ候補は、後でembeddingだけをまとめて次の探索へ渡す。
fn initial_candidates(graph: &Graph) -> Vec<Extension> {
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

            candidates.push(Extension {
                dfs_edge,
                embedding,
            });
        }
    }

    candidates
}

// どのグラフに属するかも同時に扱えば subprocedureの enumerate
pub(crate) fn collect_extensions(
    graph: &Graph,
    code: &DfsCode,
    embedding: &Embedding,
    rightmost_path: &[DfsVertexId],
) -> Vec<Extension> {
    // Property 1のneighborhood restrictionに従い、rightmost pathからだけ1-edge growthする。
    let mut extensions = Vec::new();

    let rightmost_dfs = *rightmost_path
        .last()
        .expect("rightmost path must not be empty");

    let rightmost_graph = embedding.graph_vertex(rightmost_dfs);

    // pattern中の最小vertex labelはroot edgeのfrom_label。
    let minimum_label = code
        .edges()
        .first()
        .expect("extension needs a non-empty code")
        .from_label;

    // rightmost path上のvertexから下へ伸びているtree edge。pre-pruningの比較対象。
    let tree_edges = tree_edges(code);

    // backward extensions
    // 列挙後Dfs5Tuple::Ordにより最小値を選択する(論文用再チェック)
    // rightmost vertexからrightmost path上の祖先へ戻るvalid childだけを列挙
    for &edge_id in graph.adjacency(rightmost_graph) {
        if embedding.used_edge(edge_id) {
            continue;
        }

        let target_graph = other_endpoint(graph, edge_id, rightmost_graph);

        let Some(index) = rightmost_path
            .iter()
            .position(|&dfs_vertex| embedding.graph_vertex(dfs_vertex) == target_graph)
        else {
            continue;
        };

        let target_dfs = rightmost_path[index];

        if target_dfs == rightmost_dfs {
            continue;
        }
        let edge = &graph.edges()[edge_id.0];

        // pre-pruning(i) target_dfsから下へ伸びているtree edgeより小さいbackward edgeは、
        // target_dfsからそのedgeを先に辿るcodeの方が小さくなるのでminimumになり得ない。
        let tree_edge = tree_edges[rightmost_path[index + 1].0]
            .expect("a non-root vertex on the rightmost path has a tree edge");

        if (tree_edge.edge_label, tree_edge.to_label)
            > (edge.label, graph.vertex_labels()[rightmost_graph.0])
        {
            continue;
        }

        extensions.push(Extension {
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
    // 未発見vertexへのedgeだけを、rightmost vertexからroot方向の順に候補化する。
    let new_dfs_vertex = DfsVertexId(code.vertex_count());

    for (index, &from_dfs) in rightmost_path.iter().enumerate().rev() {
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

            // pre-pruning(ii) root edgeのfrom_labelより小さいvertex labelを含むpatternは、
            // そのvertexをrootにしたcodeの方が小さいのでminimumになり得ない。
            if to_label < minimum_label {
                continue;
            }

            // pre-pruning(iii) rightmost vertex以外から伸ばす場合、そこから下へ伸びている
            // tree edgeより小さいforward edgeは、そちらを先に辿るcodeの方が小さくなる。
            if let Some(&below) = rightmost_path.get(index + 1) {
                let tree_edge = tree_edges[below.0]
                    .expect("a non-root vertex on the rightmost path has a tree edge");

                if (tree_edge.edge_label, tree_edge.to_label) > (edge.label, to_label) {
                    continue;
                }
            }

            extensions.push(Extension {
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

/// 連結graphのminimum DFS codeを返す。非連結graphはrightmost growthで表せないため拒否する。
pub fn minimum_dfs_code(graph: &Graph) -> Result<DfsCode, MinDfsError> {
    if !is_connected(graph) {
        return Err(MinDfsError::DisconnectedGraph);
    }

    if graph.edge_count() == 0 {
        return Ok(DfsCode::new());
    }

    let initial = initial_candidates(graph);

    // 最初のedgeは両端label、edge labelを含むDfs5Tupleの最小値で決める。
    let minimum_edge = initial
        .iter()
        .map(|extension| extension.dfs_edge)
        .min()
        .expect("graph with edges must have an initial candidate");

    let mut initial_iter = initial
        .into_iter()
        .filter(|extension| extension.dfs_edge == minimum_edge);

    let first = initial_iter
        .next()
        .expect("minimum initial candidate must exist");

    let mut embeddings = vec![first.embedding];
    embeddings.extend(initial_iter.map(|extension| extension.embedding));

    let mut code = DfsCode::new();
    code.push(minimum_edge);

    // 同じminimum prefixを生成するembeddingをすべて残しながら1 edgeずつ拡張する。
    while code.edge_count() < graph.edge_count() {
        let path = rightmost_path(&code);

        let extensions: Vec<Extension> = embeddings
            .iter()
            .flat_map(|embedding| collect_extensions(graph, &code, embedding, &path))
            .collect();

        // 全embeddingから得た候補を一緒に比較し、同じ最小edgeのembeddingをすべて残す。
        let minimum_edge = extensions
            .iter()
            .map(|extension| extension.dfs_edge)
            .min()
            .expect("connected graph with unused edges must have an extension");

        let mut minimum_extensions = extensions
            .into_iter()
            .filter(|extension| extension.dfs_edge == minimum_edge);

        let first = minimum_extensions
            .next()
            .expect("minimum extension must exist");

        code.push(minimum_edge);

        embeddings = vec![first.embedding];
        embeddings.extend(minimum_extensions.map(|extension| extension.embedding));
    }

    Ok(code)
}

pub fn is_min_dfs_code(graph: &Graph, candidate: &DfsCode) -> Result<bool, MinDfsError> {
    if !is_connected(graph) {
        return Err(MinDfsError::DisconnectedGraph);
    }

    if graph.edge_count() == 0 {
        return Ok(candidate.is_empty());
    }

    // candidateはこのgraph全体を表すDFS codeであることを前提にする
    if candidate.edge_count() != graph.edge_count() {
        return Ok(false);
    }

    let candidate_edges = candidate.edges();

    /*
     * 1 edge目のminimum DFS tupleを求める
     */
    let initial = initial_candidates(graph);

    let minimum_first_edge = initial
        .iter()
        .map(|extension| extension.dfs_edge)
        .min()
        .expect("graph with edges must have an initial candidate");

    // 最初のedgeですでにcandidateと違えばminimumではない
    if minimum_first_edge != candidate_edges[0] {
        return Ok(false);
    }

    let mut embeddings: Vec<Embedding> = initial
        .into_iter()
        .filter(|extension| extension.dfs_edge == minimum_first_edge)
        .map(|extension| extension.embedding)
        .collect();

    let mut minimum_prefix = DfsCode::new();
    minimum_prefix.push(minimum_first_edge);

    /*
     * 2 edge目以降をminimum DFS codeと同じ方法で生成する。
     *
     * 各stepでcandidateと比較し、違った瞬間にfalseを返す
     */
    for candidate_edge in &candidate_edges[1..] {
        let path = rightmost_path(&minimum_prefix);

        let extensions: Vec<Extension> = embeddings
            .iter()
            .flat_map(|embedding| collect_extensions(graph, &minimum_prefix, embedding, &path))
            .collect();

        let minimum_edge = extensions
            .iter()
            .map(|extension| extension.dfs_edge)
            .min()
            .expect("connected graph with unused edges must have an extension");

        /*
         * graphから生成されるminimum continuationと
         * candidateの次edgeが違う
         *      ↓
         * candidate != min(candidate)
         *      ↓
         * これ以降を生成する必要なし
         */
        if minimum_edge != *candidate_edge {
            return Ok(false);
        }

        /*
         * 同じminimum prefixを実現するembeddingをすべて残す
         * ここはminimum_dfs_codeと同じはず
         */
        embeddings = extensions
            .into_iter()
            .filter(|extension| extension.dfs_edge == minimum_edge)
            .map(|extension| extension.embedding)
            .collect();

        minimum_prefix.push(minimum_edge);
    }

    Ok(true)
}

/// BFSで全vertexへ到達できるか確認する。空graphは連結として扱う。
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

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;

    fn dfs_edge(
        from: usize,
        to: usize,
        from_label: u32,
        edge_label: u32,
        to_label: u32,
    ) -> Dfs5Tuple {
        Dfs5Tuple {
            from: DfsVertexId(from),
            to: DfsVertexId(to),
            from_label: VertexLabel(from_label),
            edge_label: EdgeLabel(edge_label),
            to_label: VertexLabel(to_label),
        }
    }

    fn assert_less(left: Dfs5Tuple, right: Dfs5Tuple) {
        assert_eq!(left.cmp(&right), Ordering::Less);
        assert_eq!(right.cmp(&left), Ordering::Greater);
        assert_eq!(left.partial_cmp(&right), Some(Ordering::Less));
    }

    // backward candidateが存在するときは、forward candidateより先に選ばれる
    #[test]
    fn dfs_tuple_orders_backward_before_forward() {
        let backward = dfs_edge(2, 0, 3, 1, 1);
        let forward = dfs_edge(1, 3, 2, 0, 4);

        assert_less(backward, forward);
    }

    // backward同士では、戻り先のDFS IDを優先し、同じならedge labelを比較する
    #[test]
    fn dfs_tuple_orders_backward_by_to_then_edge_label() {
        let smaller_to = dfs_edge(3, 0, 4, 9, 1);
        let larger_to = dfs_edge(3, 1, 4, 0, 1);
        assert_less(smaller_to, larger_to);

        let smaller_label = dfs_edge(3, 1, 4, 1, 2);
        let larger_label = dfs_edge(3, 1, 4, 2, 2);
        assert_less(smaller_label, larger_label);
    }

    // forward同士では、rightmost vertexに近い大きなfrom IDを先にする
    #[test]
    fn dfs_tuple_orders_forward_from_rightmost_to_leftmost() {
        let from_right = dfs_edge(2, 4, 3, 9, 5);
        let from_left = dfs_edge(1, 4, 3, 0, 5);

        assert_less(from_right, from_left);
    }

    // 同じ位置からのforward edgeは、edge label、発見先labelの順に比較
    #[test]
    fn dfs_tuple_orders_forward_by_edge_label_then_to_label() {
        let smaller_edge_label = dfs_edge(2, 4, 3, 1, 9);
        let larger_edge_label = dfs_edge(2, 4, 3, 2, 0);
        assert_less(smaller_edge_label, larger_edge_label);

        let smaller_to_label = dfs_edge(2, 4, 3, 1, 4);
        let larger_to_label = dfs_edge(2, 4, 3, 1, 5);
        assert_less(smaller_to_label, larger_to_label);
    }

    // 初期edgeは位置が常に(0, 1)なので、3 labelの辞書順だけで向きと辺を決める
    #[test]
    fn dfs_tuple_orders_initial_edge_labels_lexicographically() {
        let smaller_from_label = dfs_edge(0, 1, 1, 9, 9);
        let larger_from_label = dfs_edge(0, 1, 2, 0, 0);
        assert_less(smaller_from_label, larger_from_label);

        let smaller_edge_label = dfs_edge(0, 1, 1, 1, 9);
        let larger_edge_label = dfs_edge(0, 1, 1, 2, 0);
        assert_less(smaller_edge_label, larger_edge_label);

        let smaller_to_label = dfs_edge(0, 1, 1, 1, 2);
        let larger_to_label = dfs_edge(0, 1, 1, 1, 3);
        assert_less(smaller_to_label, larger_to_label);
    }

    // OrdとPartialOrdが同一tupleをEqualとして扱うことを固定する
    #[test]
    fn identical_dfs_tuples_compare_equal() {
        let edge = dfs_edge(2, 0, 3, 1, 1);

        assert_eq!(edge.cmp(&edge), Ordering::Equal);
        assert_eq!(edge.partial_cmp(&edge), Some(Ordering::Equal));
    }

    // 1-edge graphでは、小さいvertex labelをfrom側にした向きがminimumになる
    #[test]
    fn minimum_code_orients_one_edge_by_labels() {
        let mut graph = Graph::new();
        let high = graph.add_vertex(VertexLabel(2));
        let low = graph.add_vertex(VertexLabel(1));
        graph.add_edge(high, low, EdgeLabel(7));

        let code = minimum_dfs_code(&graph).unwrap();

        assert_eq!(code.edges(), &[dfs_edge(0, 1, 1, 7, 2)]);
    }

    // pathでは全edgeの両方向候補から最小の1-edge prefixを選べることを確認する
    #[test]
    fn minimum_code_for_small_path_chooses_smallest_first_edge() {
        let mut graph = Graph::new();
        let first = graph.add_vertex(VertexLabel(2));
        let middle = graph.add_vertex(VertexLabel(1));
        let last = graph.add_vertex(VertexLabel(3));
        graph.add_edge(first, middle, EdgeLabel(5));
        graph.add_edge(middle, last, EdgeLabel(4));

        let code = minimum_dfs_code(&graph).unwrap();

        assert_eq!(
            code.edges(),
            &[dfs_edge(0, 1, 1, 4, 3), dfs_edge(0, 2, 1, 5, 2)]
        );
    }

    // graph上のvertex IDが違っても、同型なtriangleのcanonical labelは一致する
    #[test]
    fn isomorphic_triangles_have_the_same_minimum_code() {
        let mut first = Graph::new();
        let a = first.add_vertex(VertexLabel(1));
        let b = first.add_vertex(VertexLabel(2));
        let c = first.add_vertex(VertexLabel(3));
        first.add_edge(a, b, EdgeLabel(3));
        first.add_edge(b, c, EdgeLabel(1));
        first.add_edge(c, a, EdgeLabel(2));

        let mut second = Graph::new();
        let c2 = second.add_vertex(VertexLabel(3));
        let a2 = second.add_vertex(VertexLabel(1));
        let b2 = second.add_vertex(VertexLabel(2));
        second.add_edge(c2, a2, EdgeLabel(2));
        second.add_edge(a2, b2, EdgeLabel(3));
        second.add_edge(b2, c2, EdgeLabel(1));

        let first_code = minimum_dfs_code(&first).unwrap();
        let second_code = minimum_dfs_code(&second).unwrap();

        assert_eq!(first_code.edge_count(), 3);
        assert_eq!(first_code, second_code);
    }

    #[test]
    fn disconnected_graph_is_rejected() {
        let mut graph = Graph::new();
        graph.add_vertex(VertexLabel(1));
        graph.add_vertex(VertexLabel(2));

        assert_eq!(
            minimum_dfs_code(&graph),
            Err(MinDfsError::DisconnectedGraph)
        );
    }
}
