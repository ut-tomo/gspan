# gspan

A Rust reimplementation of gSpan (Yan & Han, ICDM'02). It enumerates frequent connected
subgraphs of a graph database, using the minimum DFS code as the canonical label.

## Try it

No input file needed — `-g` generates a dataset you can feed straight back in.

```sh
cargo build --release
./target/release/gspan -g -s 42 > data.gspan   # generate a random dataset
./target/release/gspan -m 9 data.gspan         # mine with minimum support 9
```

A pipe works too:

```sh
./target/release/gspan -g -s 42 | ./target/release/gspan -m 9
```

Output:

```
10 (1) 0 (0f3)
10 (1) 1 (0f2)
10 (1) 1 (0f2) 0 (1f3)
10 (1) 1 (0f2) 0 (1f3) 4 (0f2)
10 (1) 1 (0f2) 4 (0f2)
10 (1) 1 (0f2) 4 (0f2) 1 (2f3)
10 (1) 4 (0f2)
10 (1) 4 (0f2) 1 (1f3)
10 (1) 4 (0f2) 1 (1f3) 0 (2f2)
10 (2) 0 (0f3)
10 (2) 0 (0f3) 1 (1f2)
10 (2) 1 (0f3)
10 (4) 2 (0f4)
```

Lowering `-m` blows the count up quickly: on the same dataset `-m 8` gives 714 patterns and
`-m 7` gives 105,484.

## Reading the output

One pattern per line: the support, followed by its DFS code.

```
6 (0) 0 (0f0) 0 (1f4) 3 (b0)
```

| Fragment | Meaning |
| --- | --- |
| `6` | support — the number of graphs containing this pattern |
| `(0)` | the label of the first vertex |
| `0 (0f0)` | vertex 0 has a forward edge with label `0`; the new vertex has label `0` |
| `0 (1f4)` | vertex 1 has a forward edge with label `0`; the new vertex has label `4` |
| `3 (b0)` | the most recent vertex has a backward edge with label `3` to vertex 0 |

Vertices are numbered in DFS discovery order. This is the same format as the C++ reference
implementation in `gl/gspan`, so sorting both outputs makes them directly comparable with `diff`.

## Usage

```
gspan [-m minsup] [graph-file]   reads stdin when graph-file is omitted
gspan -g [-s seed]               writes a random dataset in gSpan format to stdout
```

- `-m` defaults to 0 (no pruning).
- `-s` defaults to the seed in `GeneratorConfig::default()`; the same seed yields the same data.

## Input format

```
t # 0 1 pos_0    start of a graph; the id, class and name are ignored
v 0 1            vertex: v <id> <label>   ids start at 0 and increase by one
v 1 2
e 0 1 0          edge:   e <from> <to> <label>
                 blank line separator, kept for compatibility with the C++ reference
t # 1 -1 neg_0   (it is optional here)
...
```

Self loops and parallel edges are rejected. Disconnected graphs are fine as input.

## Layout

| File | Role |
| --- | --- |
| `src/graph.rs` | undirected labeled graph |
| `src/graph_to_mindfs.rs` | DFS codes, minimum DFS code, rightmost-path extension |
| `src/subgraph_mining.rs` | the frequent subgraph search itself |
| `src/gengraph.rs` | dataset generation, mixing feature graphs into positive/negative transactions |
| `src/main.rs` | CLI and gSpan-format I/O |
