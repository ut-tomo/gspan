#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VertexLabel(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeLabel(pub u32);

#[derive(Debug, Clone)]
pub struct Edge {
    pub endpoints: [VertexId; 2],
    pub label: EdgeLabel,
}

#[derive(Debug, Clone, Default)]
pub struct Graph {
    vertex_labels: Vec<VertexLabel>,
    edges: Vec<Edge>,
    adjacency: Vec<Vec<EdgeId>>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn vertex_count(&self) -> usize {
        self.vertex_labels.len()
    }
    pub fn vertex_labels(&self) -> &[VertexLabel] {
        &self.vertex_labels
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn adjacency(&self, vertex: VertexId) -> &[EdgeId] {
        &self.adjacency[vertex.0]
    }

    pub fn add_vertex(&mut self, label: VertexLabel) -> VertexId {
        let id = VertexId(self.vertex_labels.len());
        self.vertex_labels.push(label);
        self.adjacency.push(Vec::new());
        id
    }
    pub fn add_edge(&mut self, u: VertexId, v: VertexId, label: EdgeLabel) -> EdgeId {
        assert_ne!(u, v, "No Self Loop");
        assert!(!self.has_edge(u, v), "No Parallel Edge");

        let id = EdgeId(self.edges.len());

        self.edges.push(Edge {
            endpoints: [u, v],
            label,
        });

        self.adjacency[u.0].push(id);
        self.adjacency[v.0].push(id);

        id
    }

    pub fn has_edge(&self, u: VertexId, v: VertexId) -> bool {
        self.adjacency[u.0].iter().any(|edge_id| {
            let edge = &self.edges[edge_id.0];
            edge.endpoints == [u, v] || edge.endpoints == [v, u]
        })
    }
    pub fn append_disjoint(&mut self, other: &Graph) -> std::ops::Range<usize> {
        let vertex_offset = self.vertex_count();

        for &label in other.vertex_labels() {
            self.add_vertex(label);
        }

        for edge in other.edges() {
            let u = VertexId(vertex_offset + edge.endpoints[0].0);
            let v = VertexId(vertex_offset + edge.endpoints[1].0);

            self.add_edge(u, v, edge.label);
        }

        vertex_offset..self.vertex_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_graph_keeps_edges_and_adjacency_consistent() {
        let mut graph = Graph::new();
        let first = graph.add_vertex(VertexLabel(10));
        let second = graph.add_vertex(VertexLabel(20));
        let third = graph.add_vertex(VertexLabel(30));

        let first_edge = graph.add_edge(first, second, EdgeLabel(1));
        let second_edge = graph.add_edge(second, third, EdgeLabel(2));

        assert_eq!(graph.vertex_count(), 3);
        assert_eq!(graph.edge_count(), 2);
        assert_eq!(
            graph.vertex_labels(),
            &[VertexLabel(10), VertexLabel(20), VertexLabel(30)]
        );
        assert_eq!(graph.adjacency(first), &[first_edge]);
        assert_eq!(graph.adjacency(second), &[first_edge, second_edge]);
        assert_eq!(graph.adjacency(third), &[second_edge]);
        assert!(graph.has_edge(first, second));
        assert!(!graph.has_edge(first, third));
    }

    #[test]
    fn append_disjoint_offsets_vertices_and_preserves_labels() {
        let mut graph = Graph::new();
        graph.add_vertex(VertexLabel(1));

        let mut other = Graph::new();
        let first = other.add_vertex(VertexLabel(2));
        let second = other.add_vertex(VertexLabel(3));
        other.add_edge(first, second, EdgeLabel(4));

        let appended = graph.append_disjoint(&other);

        assert_eq!(appended, 1..3);
        assert_eq!(
            graph.vertex_labels(),
            &[VertexLabel(1), VertexLabel(2), VertexLabel(3)]
        );
        assert_eq!(graph.edges()[0].endpoints, [VertexId(1), VertexId(2)]);
        assert_eq!(graph.edges()[0].label, EdgeLabel(4));
    }
}
