#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexLabel(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
/*
gengraph
*/

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
