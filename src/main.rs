use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Read};

use gspan::graph::{EdgeLabel, Graph, VertexId, VertexLabel};
use gspan::graph_to_mindfs::DfsCode;
use gspan::subgraph_mining::{FrequentPattern, subgraph_mining};

const USAGE: &str = "usage: gspan [-m minsup] [graph-file]";

fn main() -> Result<(), Box<dyn Error>> {
    // 参照実装(gl/gspan/main.cpp)の `-m minsup` に合わせてCLIから受け取る。
    let options = parse_arguments()?;

    let graphs = load_graphs(options.graph_file.as_deref())?;

    let patterns = subgraph_mining(&graphs, options.min_support);

    print_patterns(&patterns);

    Ok(())
}

struct Options {
    min_support: usize,
    graph_file: Option<String>,
}

fn parse_arguments() -> Result<Options, Box<dyn Error>> {
    let mut options = Options {
        min_support: 0,
        graph_file: None,
    };

    let mut arguments = std::env::args().skip(1);

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "-m" => {
                let value = arguments.next().ok_or(USAGE)?;
                options.min_support = value.parse()?;
            }
            // 参照実装が持つ -x maxpat / -i (instance出力) は未実装なので明示的に弾いておく
            "-x" | "-i" => return Err(format!("{argument} is not implemented; {USAGE}").into()),
            _ if options.graph_file.is_none() => options.graph_file = Some(argument),
            _ => return Err(USAGE.into()),
        }
    }

    Ok(options)
}

// (`t # ...` / `v id label` / `e u v label`)読む

fn load_graphs(graph_file: Option<&str>) -> Result<Vec<Graph>, Box<dyn Error>> {
    let text = match graph_file {
        Some(path) => fs::read_to_string(path)?,
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    Ok(parse_gspan_graphs(&text)?)
}

fn parse_gspan_graphs(text: &str) -> Result<Vec<Graph>, ParseError> {
    let mut graphs: Vec<Graph> = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let mut fields = line.split_whitespace();

        let error = |message: &str| ParseError {
            line_number,
            message: message.to_string(),
        };

        match fields.next() {
            // `t # <id> ...` で新しいtransactionが始まる
            Some("t") => graphs.push(Graph::new()),

            Some("v") => {
                let graph = graphs.last_mut().ok_or_else(|| error("v before t"))?;

                let id: usize = parse_field(&mut fields).ok_or_else(|| error("invalid v"))?;
                let label: u32 = parse_field(&mut fields).ok_or_else(|| error("invalid v"))?;

                if id != graph.vertex_count() {
                    return Err(error("vertex ids must start at 0 and increase by one"));
                }

                graph.add_vertex(VertexLabel(label));
            }

            Some("e") => {
                let graph = graphs.last_mut().ok_or_else(|| error("e before t"))?;

                let from: usize = parse_field(&mut fields).ok_or_else(|| error("invalid e"))?;
                let to: usize = parse_field(&mut fields).ok_or_else(|| error("invalid e"))?;
                let label: u32 = parse_field(&mut fields).ok_or_else(|| error("invalid e"))?;

                if from >= graph.vertex_count() || to >= graph.vertex_count() {
                    return Err(error("edge refers to an undeclared vertex"));
                }

                let (from, to) = (VertexId(from), VertexId(to));

                if from == to {
                    return Err(error("self loops are not supported"));
                }
                if graph.has_edge(from, to) {
                    return Err(error("parallel edges are not supported"));
                }

                graph.add_edge(from, to, EdgeLabel(label));
            }

            _ => {}
        }
    }

    Ok(graphs)
}

fn parse_field<'a, T: std::str::FromStr>(fields: &mut impl Iterator<Item = &'a str>) -> Option<T> {
    fields.next()?.parse().ok()
}

fn print_patterns(patterns: &[FrequentPattern]) {
    for pattern in patterns {
        println!("{} {}", pattern.support, ReferenceStyle(&pattern.code));
    }
}

struct ReferenceStyle<'a>(&'a DfsCode);

impl fmt::Display for ReferenceStyle<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(first) = self.0.edges().first() else {
            return Ok(());
        };

        write!(
            f,
            "({}) {} ({}f{})",
            first.from_label.0, first.edge_label.0, first.from.0, first.to_label.0
        )?;

        for edge in &self.0.edges()[1..] {
            if edge.is_forward() {
                write!(
                    f,
                    " {} ({}f{})",
                    edge.edge_label.0, edge.from.0, edge.to_label.0
                )?;
            } else {
                write!(f, " {} (b{})", edge.edge_label.0, edge.to.0)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct ParseError {
    line_number: usize,
    message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line_number, self.message)
    }
}

impl Error for ParseError {}
