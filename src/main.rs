mod modules;

use std::env;

enum Mode {
    Server,
    Benchmark,
}

struct CliConfig {
    mode: Mode,
    protocol_port: Option<u16>,
    nodes: Option<usize>,
    threads: Option<usize>,
    silent: bool,
}

impl CliConfig {
    fn from_args(args: Vec<String>) -> Self {
        let mut mode = Mode::Server;
        let mut protocol_port = None;
        let mut nodes = None;
        let mut threads = None;
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
                            Ok(num) => protocol_port = Some(num),
                            Err(_) => eprintln!(
                                "⚠️  Invalid value for --port: {} (using default).",
                                value
                            ),
                        }
                    } else {
                        eprintln!("⚠️  Missing value for --port (using default).");
                    }
                }
                "--protocol-port" => {
                    if let Some(value) = iter.next() {
                        match value.parse::<u16>() {
                            Ok(num) => protocol_port = Some(num),
                            Err(_) => eprintln!(
                                "⚠️  Invalid value for --protocol-port: {} (using default).",
                                value
                            ),
                        }
                    } else {
                        eprintln!("⚠️  Missing value for --protocol-port (using default).");
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
                "--threads" => {
                    if let Some(value) = iter.next() {
                        match value.parse::<usize>() {
                            Ok(num) if num > 0 => threads = Some(num),
                            Ok(_) => eprintln!(
                                "⚠️  --threads must be greater than 0 (using default)."
                            ),
                            Err(_) => eprintln!(
                                "⚠️  Invalid value for --threads: {} (using default).",
                                value
                            ),
                        }
                    } else {
                        eprintln!("⚠️  Missing value for --threads (using default).");
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
            protocol_port,
            nodes,
            threads,
            silent,
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let config = CliConfig::from_args(args);

    // Configure thread pool if specified
    if let Some(threads) = config.threads {
        use modules::search::configure_thread_pool;
        configure_thread_pool(Some(threads));
        if !config.silent {
            println!("⚙️  Configured thread pool with {} threads", threads);
        }
    }

    match config.mode {
        Mode::Benchmark => run_benchmark_mode(config.nodes, config.silent).await,
        Mode::Server => run_server_mode(config.protocol_port, config.silent).await,
    }
}

async fn run_server_mode(protocol_port_override: Option<u16>, silent: bool) {
    let protocol_port = protocol_port_override
        .or_else(|| {
            env::var("SARYCHDB_PROTOCOL_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
        })
        .or_else(|| {
            env::var("PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
        })
        .unwrap_or(4040);

    if !silent {
        println!("🌟 SarychDB - Parallel Database System");
        println!("======================================");
        println!("🛰️  Starting SarychDB protocol on port {}", protocol_port);
    }

    modules::server::SarychServer::start_protocol_server(protocol_port).await;
}

async fn run_benchmark_mode(nodes_override: Option<usize>, silent: bool) {
    use std::time::Instant;
    use modules::search::{Item, load_json, split_nodes, centralized_search, sequential_search, parallel_search, smart_search, get_optimal_node_count};
    
    let optimal_nodes = get_optimal_node_count();
    let num_nodes = nodes_override.unwrap_or(optimal_nodes);
    
    if !silent {
        println!("🔧 CPU has {} optimal cores available", optimal_nodes);
        println!("Running benchmark with {} nodes", num_nodes);
    }

    let data: Vec<Item> = match load_json("500MB.json") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("❌ Benchmark data error: {}", e);
            return;
        }
    };
    let nodes = split_nodes(data, num_nodes);

    let queries = ["T206", "id", "TensorFlow"];

    for &query in &queries {
        if !silent {
            println!("\n🔎 Benchmark for query: \"{}\"", query);
        }

        let start = Instant::now();
        let r1 = centralized_search(&nodes, query);
        let t1 = start.elapsed().as_millis();

        let start = Instant::now();
        let r2 = sequential_search(&nodes, query);
        let t2 = start.elapsed().as_millis();

        let start = Instant::now();
        let r3 = parallel_search(&nodes, query);
        let t3 = start.elapsed().as_millis();

        let start = Instant::now();
        let r4 = smart_search(&nodes, query);
        let t4 = start.elapsed().as_millis();

        if !silent {
            println!("Centralized: {} results in {} ms", r1.len(), t1);
            println!("Sequential multi-node: {} results in {} ms", r2.len(), t2);
            println!("Parallel multi-node: {} results in {} ms", r3.len(), t3);
            println!("Smart search (auto): {} results in {} ms ⭐", r4.len(), t4);
        }
    }
}