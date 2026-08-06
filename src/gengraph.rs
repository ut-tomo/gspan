/*
ラベル+1になるやつP個
ラベル-1になるやつQ個

positive feature N個, negative feature M個をそれぞれ決めて、
正例transactionにはpositive featureを確率pで追加、negative featureを確率rで混入
負例transactionにはnegative featureを確率qで追加、positive featureを確率rで混入

各transactionについて、追加するfeatureが決まったら(確率的に選択したら)
それらをつなぎ合わせる形で生成。
生成されたグラフの重複をminimum DFS codeでみる



該当実装↓↓

 // output the transactions

  for(int i=0; i<P; ++i){
    Graph out_g;
    for(int j=0; j<N; ++j){
      if(runif(0,1)>p) continue;
      //std::ostringstream os;
      //os << "pos_comp_" << i << "-" << j << ".sif";
      //output_sif(g_pos[j],os.str());
      graph_append(out_g, g_pos[j], num_elab);
    }
    for(int j=0; j<M; ++j){
      if(runif(0,1)>r) continue;
      //std::ostringstream os;
      //os << "pos_ctrl_comp_" << i << "-" << j << ".sif";
      //output_sif(g_pos[j],os.str());
      graph_append(out_g, g_neg[j], num_elab);
    }
    //print_graph(out_g);
    std::ostringstream os2;
    //os2 << "pos_" << i << ".sif";
    //output_sif(out_g,os2.str());
    os2.str("");
    os2 << "pos_" << i << "_edge_" << out_g.edgeline;
    output_gspan(out_g,ofs1,os2.str(),1);

    std::cout << "pos[" << i << "] edge=" << out_g.edgeline << std::endl;
  }

  for(int i=0; i<Q; ++i){
    Graph out_g;
    for(int j=0; j<M; ++j){
      if(runif(0,1)>q) continue;
      //std::ostringstream os;
      //os << "neg_comp_" << i << "-" << j << ".sif";
      //output_sif(g_neg[j],os.str());
      graph_append(out_g, g_neg[j], num_elab);
    }
    for(int j=0; j<N; ++j){
      if(runif(0,1)>s) continue;
      //std::ostringstream os;
      //os << "pos_comp_" << i << "-" << j << ".sif";
      //output_sif(g_pos[j],os.str());
      graph_append(out_g, g_pos[j], num_elab);
    }
    //print_graph(out_g);
    std::ostringstream os2;
    //os2 << "neg_" << i << ".sif";
    //output_sif(out_g,os2.str());
    os2.str("");
    os2 << "neg_" << i << "_edge_" << out_g.edgeline;
    output_gspan(out_g,ofs1,os2.str(),-1);

    std::cout << "neg[" << i << "] edge=" << out_g.edgeline << std::endl;
  }
*/

//! C++版`gengraph`と同じ流れで、特徴グラフと正負transactionを生成する。
//! 乱数生成、minimum DFS codeによる重複排除、特徴の混合、gSpan形式への出力を
//! それぞれ分離し、同じseedから同じRust側データセットを再生成できるようにしている。

use std::collections::HashSet;
use std::io::{self, Write};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use thiserror::Error;

use crate::graph::{EdgeLabel, Graph, VertexId, VertexLabel};
use crate::graph_to_mindfs::{DfsCode, MinDfsError, minimum_dfs_code};

#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// 特徴グラフの辺数を決めるPoisson分布の平均
    pub mean_edge_count: f64,
    /// edge labelの種類数(-e)
    pub edge_label_count: u32,
    pub vertex_label_count: u32,
    pub positive_feature_count: usize,
    pub negative_feature_count: usize,
    pub positive_transaction_count: usize,
    pub negative_transaction_count: usize,
    pub positive_feature_probability: f64,
    pub negative_feature_probability: f64,
    pub negative_in_positive_probability: f64,
    pub positive_in_negative_probability: f64,
    pub seed: u64,
    pub max_feature_attempts: usize,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            mean_edge_count: 10.0,
            edge_label_count: 5,
            vertex_label_count: 5,
            positive_feature_count: 3,
            negative_feature_count: 2,
            positive_transaction_count: 5,
            negative_transaction_count: 5,
            positive_feature_probability: 0.9,
            negative_feature_probability: 0.9,
            negative_in_positive_probability: 0.2,
            positive_in_negative_probability: 0.2,
            seed: 1977,
            max_feature_attempts: 100_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FeatureGraph {
    pub graph: Graph,
    pub minimum_dfs_code: DfsCode,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub graph: Graph,
    pub class_label: i8,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct GeneratedDataset {
    pub positive_features: Vec<FeatureGraph>,
    pub negative_features: Vec<FeatureGraph>,
    pub positive_transactions: Vec<Transaction>,
    pub negative_transactions: Vec<Transaction>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GenerateError {
    #[error("invalid generator configuration: {0}")]
    InvalidConfig(&'static str),
    #[error("could not generate {requested} unique features in {attempts} attempts")]
    FeatureAttemptsExceeded { requested: usize, attempts: usize },
    #[error(transparent)]
    MinimumDfs(#[from] MinDfsError),
}

/// feature生成とtransaction生成を同じ乱数列上で順番に実行
///
/// 正例ではpositive featureを`p`、negative featureを`r`で選び、
/// 負例ではnegative featureを`q`、positive featureを`s`で選ぶ
pub fn generate_dataset(config: &GeneratorConfig) -> Result<GeneratedDataset, GenerateError> {
    validate_config(config)?;

    let mut rng = StdRng::seed_from_u64(config.seed);
    let (positive_features, negative_features) = generate_features(config, &mut rng)?;
    let positive_transactions = generate_transactions(
        config.positive_transaction_count,
        1,
        "pos",
        &positive_features,
        config.positive_feature_probability,
        &negative_features,
        config.negative_in_positive_probability,
        config.edge_label_count,
        &mut rng,
    );
    let negative_transactions = generate_transactions(
        config.negative_transaction_count,
        -1,
        "neg",
        &negative_features,
        config.negative_feature_probability,
        &positive_features,
        config.positive_in_negative_probability,
        config.edge_label_count,
        &mut rng,
    );

    Ok(GeneratedDataset {
        positive_features,
        negative_features,
        positive_transactions,
        negative_transactions,
    })
}

pub fn write_feature_codes<W: Write>(dataset: &GeneratedDataset, mut writer: W) -> io::Result<()> {
    for feature in dataset
        .positive_features
        .iter()
        .chain(&dataset.negative_features)
    {
        writeln!(writer, "{}", feature.minimum_dfs_code)?;
    }

    Ok(())
}

pub fn write_transactions<W: Write>(dataset: &GeneratedDataset, mut writer: W) -> io::Result<()> {
    for transaction in dataset
        .positive_transactions
        .iter()
        .chain(&dataset.negative_transactions)
    {
        write_gspan_graph(transaction, &mut writer)?;
    }

    Ok(())
}

fn validate_config(config: &GeneratorConfig) -> Result<(), GenerateError> {
    if !config.mean_edge_count.is_finite()
        || config.mean_edge_count <= 0.0
        || config.mean_edge_count > 700.0
    {
        return Err(GenerateError::InvalidConfig(
            "mean_edge_count must be finite and in 0 < mean_edge_count <= 700",
        ));
    }
    if config.edge_label_count == 0 {
        return Err(GenerateError::InvalidConfig(
            "edge_label_count must be greater than zero",
        ));
    }
    if config.vertex_label_count == 0 {
        return Err(GenerateError::InvalidConfig(
            "vertex_label_count must be greater than zero",
        ));
    }
    if config.max_feature_attempts == 0
        && config.positive_feature_count + config.negative_feature_count > 0
    {
        return Err(GenerateError::InvalidConfig(
            "max_feature_attempts must be greater than zero when features are requested",
        ));
    }

    for probability in [
        config.positive_feature_probability,
        config.negative_feature_probability,
        config.negative_in_positive_probability,
        config.positive_in_negative_probability,
    ] {
        if !(0.0..=1.0).contains(&probability) {
            return Err(GenerateError::InvalidConfig(
                "all probabilities must be in 0 <= probability <= 1",
            ));
        }
    }

    Ok(())
}

fn generate_features<R: Rng>(
    config: &GeneratorConfig,
    rng: &mut R,
) -> Result<(Vec<FeatureGraph>, Vec<FeatureGraph>), GenerateError> {
    let requested = config.positive_feature_count + config.negative_feature_count;
    let mut seen_codes = HashSet::new();
    let mut positive = Vec::with_capacity(config.positive_feature_count);
    let mut negative = Vec::with_capacity(config.negative_feature_count);

    if requested == 0 {
        return Ok((positive, negative));
    }

    for attempts in 1..=config.max_feature_attempts {
        let edge_count = loop {
            let edge_count = sample_poisson(config.mean_edge_count, rng);
            if edge_count >= 2 {
                break edge_count;
            }
        };
        let graph = generate_feature_graph(
            edge_count,
            config.vertex_label_count,
            config.edge_label_count,
            rng,
        );
        let minimum_dfs_code = minimum_dfs_code(&graph)?;

        // 同型は非採用
        if !seen_codes.insert(minimum_dfs_code.clone()) {
            continue;
        }

        let feature = FeatureGraph {
            graph,
            minimum_dfs_code,
        };
        if positive.len() < config.positive_feature_count {
            positive.push(feature);
        } else {
            negative.push(feature);
        }

        if positive.len() + negative.len() == requested {
            return Ok((positive, negative));
        }

        if attempts == config.max_feature_attempts {
            break;
        }
    }

    Err(GenerateError::FeatureAttemptsExceeded {
        requested,
        attempts: config.max_feature_attempts,
    })
}

fn sample_poisson<R: Rng>(mean: f64, rng: &mut R) -> usize {
    let threshold = (-mean).exp();
    let mut product = 1.0;
    let mut count = 0;

    loop {
        product *= rng.random::<f64>();
        if product <= threshold {
            return count;
        }
        count += 1;
    }
}

fn generate_feature_graph<R: Rng>(
    edge_count: usize,
    vertex_label_count: u32,
    edge_label_count: u32,
    rng: &mut R,
) -> Graph {
    assert!(edge_count >= 2, "a feature must have at least two edges");

    // 1辺から開始し、既存vertex間または新規vertexへの辺を足すので、常に連結になる
    let mut graph = Graph::new();
    let first = graph.add_vertex(random_vertex_label(vertex_label_count, rng));
    let second = graph.add_vertex(random_vertex_label(vertex_label_count, rng));
    graph.add_edge(first, second, random_edge_label(edge_label_count, rng));

    for _ in 1..edge_count {
        // choose a node from V(g)
        let selected = VertexId(rng.random_range(0..graph.vertex_count()));

        // all = { a set of node indices }
        let available: Vec<VertexId> = (0..graph.vertex_count())
            .map(VertexId)
            .filter(|&candidate| candidate != selected && !graph.has_edge(selected, candidate))
            .collect();

        // generate a random integer between 0 and all.size()
        let go = rng.random_range(0..=available.len());

        if go == 0 {
            // generate a new node and a edge from i=selec to j=newnode
            let new_vertex = graph.add_vertex(random_vertex_label(vertex_label_count, rng));
            graph.add_edge(
                selected,
                new_vertex,
                random_edge_label(edge_label_count, rng),
            );
        } else {
            // create a edge from i=selec to j=all[go-1]
            graph.add_edge(
                selected,
                available[go - 1],
                random_edge_label(edge_label_count, rng),
            );
        }
    }

    graph
}

#[allow(clippy::too_many_arguments)]
fn generate_transactions<R: Rng>(
    count: usize,
    class_label: i8,
    name_prefix: &str,
    primary_features: &[FeatureGraph],
    primary_probability: f64,
    control_features: &[FeatureGraph],
    control_probability: f64,
    edge_label_count: u32,
    rng: &mut R,
) -> Vec<Transaction> {
    let mut transactions = Vec::with_capacity(count);

    for index in 0..count {
        let mut graph = Graph::new();

        for feature in primary_features {
            if rng.random::<f64>() > primary_probability {
                continue;
            }
            append_feature(&mut graph, &feature.graph, edge_label_count, rng);
        }
        for feature in control_features {
            if rng.random::<f64>() > control_probability {
                continue;
            }
            append_feature(&mut graph, &feature.graph, edge_label_count, rng);
        }

        let name = format!("{name_prefix}_{index}_edge_{}", graph.edge_count());
        transactions.push(Transaction {
            graph,
            class_label,
            name,
        });
    }

    transactions
}

fn append_feature<R: Rng>(output: &mut Graph, feature: &Graph, edge_label_count: u32, rng: &mut R) {
    if feature.vertex_count() == 0 {
        return;
    }
    if output.vertex_count() == 0 {
        *output = feature.clone();
        return;
    }

    // 2個目以降はdisjointに追加した後、両成分から1vertexずつ選んで1辺で接続
    let previous_vertex_count = output.vertex_count();
    let appended_vertices = output.append_disjoint(feature);
    let first = VertexId(rng.random_range(0..previous_vertex_count));
    let second = VertexId(rng.random_range(appended_vertices));

    output.add_edge(first, second, random_edge_label(edge_label_count, rng));
}

fn random_vertex_label<R: Rng>(label_count: u32, rng: &mut R) -> VertexLabel {
    VertexLabel(rng.random_range(0..label_count))
}

fn random_edge_label<R: Rng>(label_count: u32, rng: &mut R) -> EdgeLabel {
    EdgeLabel(rng.random_range(0..label_count))
}

fn write_gspan_graph<W: Write>(transaction: &Transaction, writer: &mut W) -> io::Result<()> {
    writeln!(
        writer,
        "t # 0 {} {}",
        transaction.class_label, transaction.name
    )?;
    for (vertex_id, label) in transaction.graph.vertex_labels().iter().enumerate() {
        writeln!(writer, "v {vertex_id} {}", label.0)?;
    }
    for edge in transaction.graph.edges() {
        writeln!(
            writer,
            "e {} {} {}",
            edge.endpoints[0].0, edge.endpoints[1].0, edge.label.0
        )?;
    }
    writeln!(writer)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;

    #[test]
    fn generated_feature_is_connected_and_simple_with_requested_edge_count() {
        let mut rng = StdRng::seed_from_u64(1);
        let graph = generate_feature_graph(8, 4, 3, &mut rng);

        assert_eq!(graph.edge_count(), 8);
        assert!(graph.vertex_count() >= 2);

        let mut endpoint_pairs = HashSet::new();
        for edge in graph.edges() {
            let mut endpoints = [edge.endpoints[0].0, edge.endpoints[1].0];
            endpoints.sort_unstable();
            assert!(endpoint_pairs.insert(endpoints));
            assert!(edge.label.0 < 3);
        }
        assert!(graph.vertex_labels().iter().all(|label| label.0 < 4));

        let mut reached = vec![false; graph.vertex_count()];
        let mut stack = vec![VertexId(0)];
        reached[0] = true;
        while let Some(vertex) = stack.pop() {
            for &edge_id in graph.adjacency(vertex) {
                let edge = &graph.edges()[edge_id.0];
                let next = if edge.endpoints[0] == vertex {
                    edge.endpoints[1]
                } else {
                    edge.endpoints[0]
                };
                if !reached[next.0] {
                    reached[next.0] = true;
                    stack.push(next);
                }
            }
        }
        assert!(reached.into_iter().all(|vertex| vertex));
    }

    #[test]
    fn appending_feature_adds_one_connecting_edge() {
        let mut first = Graph::new();
        let first_a = first.add_vertex(VertexLabel(0));
        let first_b = first.add_vertex(VertexLabel(1));
        first.add_edge(first_a, first_b, EdgeLabel(0));

        let mut second = Graph::new();
        let second_a = second.add_vertex(VertexLabel(2));
        let second_b = second.add_vertex(VertexLabel(3));
        second.add_edge(second_a, second_b, EdgeLabel(1));

        let mut rng = StdRng::seed_from_u64(2);
        append_feature(&mut first, &second, 3, &mut rng);

        assert_eq!(first.vertex_count(), 4);
        assert_eq!(first.edge_count(), 3);
        assert!(
            first
                .edges()
                .iter()
                .any(|edge| edge.endpoints[0].0 < 2 && edge.endpoints[1].0 >= 2)
        );
    }

    #[test]
    fn dataset_generation_is_seeded_and_follows_mixture_probabilities() {
        let config = GeneratorConfig {
            mean_edge_count: 3.0,
            edge_label_count: 3,
            vertex_label_count: 4,
            positive_feature_count: 1,
            negative_feature_count: 1,
            positive_transaction_count: 2,
            negative_transaction_count: 2,
            positive_feature_probability: 1.0,
            negative_feature_probability: 1.0,
            negative_in_positive_probability: 1.0,
            positive_in_negative_probability: 1.0,
            seed: 42,
            max_feature_attempts: 1_000,
        };

        let first = generate_dataset(&config).unwrap();
        let second = generate_dataset(&config).unwrap();

        assert_eq!(first.positive_features.len(), 1);
        assert_eq!(first.negative_features.len(), 1);
        assert_eq!(first.positive_transactions.len(), 2);
        assert_eq!(first.negative_transactions.len(), 2);
        assert_ne!(
            first.positive_features[0].minimum_dfs_code,
            first.negative_features[0].minimum_dfs_code
        );

        let component_edge_count = first.positive_features[0].graph.edge_count()
            + first.negative_features[0].graph.edge_count();
        for transaction in first
            .positive_transactions
            .iter()
            .chain(&first.negative_transactions)
        {
            assert_eq!(transaction.graph.edge_count(), component_edge_count + 1);
        }

        let mut first_output = Vec::new();
        let mut second_output = Vec::new();
        write_transactions(&first, &mut first_output).unwrap();
        write_transactions(&second, &mut second_output).unwrap();
        assert_eq!(first_output, second_output);

        let output = String::from_utf8(first_output).unwrap();
        assert!(output.contains("t # 0 1 pos_0_edge_"));
        assert!(output.contains("t # 0 -1 neg_0_edge_"));
        assert!(output.contains("\nv 0 "));
        assert!(output.contains("\ne "));
    }

    #[test]
    fn zero_features_produce_empty_transactions() {
        let config = GeneratorConfig {
            positive_feature_count: 0,
            negative_feature_count: 0,
            positive_transaction_count: 1,
            negative_transaction_count: 1,
            ..GeneratorConfig::default()
        };

        let dataset = generate_dataset(&config).unwrap();

        assert!(dataset.positive_features.is_empty());
        assert!(dataset.negative_features.is_empty());
        assert_eq!(dataset.positive_transactions[0].graph.edge_count(), 0);
        assert_eq!(dataset.negative_transactions[0].graph.edge_count(), 0);
    }
}
