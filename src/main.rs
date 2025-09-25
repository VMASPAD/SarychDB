mod modules;

use modules::server::start_server;
use std::env;

enum Mode {
    Server,
    Benchmark,
}

struct CliConfig {
    mode: Mode,
    port: Option<u16>,
    nodes: Option<usize>,
    silent: bool,
}

impl CliConfig {
    fn from_args(args: Vec<String>) -> Self {
        let mut mode = Mode::Server;
        let mut port = None;
        let mut nodes = None;
        let mut silent = false;

        let mut iter = args.into_iter().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "benchmark" => {
                    mode = Mode::Benchmark;
                }
                "--port" => {
                    if let Some(value) = iter.next() {
                        match value.parse::<u16>() {
                            Ok(num) => port = Some(num),
                            Err(_) => eprintln!(
                                "⚠️  Invalid value for --port: {} (using default).",
                                value
                            ),
                        }
                    } else {
                        eprintln!("⚠️  Missing value for --port (using default).");
                    }
                }
                "--nodes" => {
                    if let Some(value) = iter.next() {
                        match value.parse::<usize>() {
                            Ok(num) if num > 0 => nodes = Some(num),
                            Ok(_) => eprintln!(
                                "⚠️  --nodes must be greater than 0 (using default)."
                            ),
                            Err(_) => eprintln!(
                                "⚠️  Invalid value for --nodes: {} (using default).",
                                value
                            ),
                        }
                    } else {
                        eprintln!("⚠️  Missing value for --nodes (using default).");
                    }
                }
                "--background" | "--silent" => {
                    silent = true;
                }

                "--foreground" => {
                    silent = false;
                }
                other => {
                    eprintln!("⚠️  Unrecognized argument '{}' - ignoring.", other);
                }
            }
        }

        CliConfig {
            mode,
            port,
            nodes,
            silent,
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let config = CliConfig::from_args(args);

    match config.mode {
        Mode::Benchmark => run_benchmark_mode(config.nodes, config.silent).await,
        Mode::Server => run_server_mode(config.port, config.silent).await,
    }
}

async fn run_server_mode(port_override: Option<u16>, silent: bool) {
    let port = port_override
        .or_else(|| {
            env::var("PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
        })
        .unwrap_or(3030);

    if !silent {
        println!("🌟 SarychDB - Parallel Database System");
        println!("======================================");
        println!("🚀 Starting server on port {}", port);
    }
    
    start_server(port).await;
}

async fn run_benchmark_mode(nodes_override: Option<usize>, silent: bool) {
    use std::time::Instant;
    use modules::search::{Item, load_json, split_nodes, centralized_search, sequential_search, parallel_search};
    
    fn run_benchmark(nodes: &Vec<Vec<Item>>, queries: &[&str], silent: bool) {
        for &query in queries {
            if !silent {
                println!("\n🔎 Benchmark for query: \"{}\"", query);
            }

            let start = Instant::now();
            let r1 = centralized_search(nodes, query);
            let t1 = start.elapsed().as_millis();

            let start = Instant::now();
            let r2 = sequential_search(nodes, query);
            let t2 = start.elapsed().as_millis();

            let start = Instant::now();
            let r3 = parallel_search(nodes, query);
            let t3 = start.elapsed().as_millis();

            if !silent {
                println!("Centralized: {} results in {} ms", r1.len(), t1);
                println!("Sequential multi-node: {} results in {} ms", r2.len(), t2);
                println!("Parallel multi-node: {} results in {} ms", r3.len(), t3);
            }
        }
    }

    let num_nodes = nodes_override.unwrap_or(8);
    if !silent {
        println!("Running benchmark with {} nodes", num_nodes);
    }

    if !silent {
        println!("📂 Loading 500MB.json...");
    }
    let items = load_json("500MB.json");
    if !silent {
        println!("Total records: {}", items.len());
    }

    let nodes = split_nodes(items, num_nodes);

    // Lista de queries a probar
    let queries = ["P1605"];

    run_benchmark(&nodes, &queries, silent);
}
