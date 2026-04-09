mod modules;

use std::env;

enum Mode {
    Server,
    RestApi,
    Benchmark,
}

struct CliConfig {
    mode: Mode,
    protocol_port: Option<u16>,
    nodes: Option<usize>,
    threads: Option<usize>,
    http_api: bool,
    https: bool,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    silent: bool,
}

impl CliConfig {
    fn from_args(args: Vec<String>) -> Self {
        let mut mode = Mode::Server;
        let mut protocol_port = None;
        let mut nodes = None;
        let mut threads = None;
        let mut http_api = false;
        let mut https = false;
        let mut tls_cert = None;
        let mut tls_key = None;
        let mut silent = false;

        let mut iter = args.into_iter().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "benchmark" => {
                    mode = Mode::Benchmark;
                }
                "--rest" | "--http-api" | "--http" => {
                    mode = Mode::RestApi;
                    http_api = true;
                }
                "--https" => {
                    mode = Mode::RestApi;
                    http_api = true;
                    https = true;
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
                            Ok(_) => {
                                eprintln!("⚠️  --nodes must be greater than 0 (using default).")
                            }
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
                            Ok(_) => {
                                eprintln!("⚠️  --threads must be greater than 0 (using default).")
                            }
                            Err(_) => eprintln!(
                                "⚠️  Invalid value for --threads: {} (using default).",
                                value
                            ),
                        }
                    } else {
                        eprintln!("⚠️  Missing value for --threads (using default).");
                    }
                }
                "--tls-cert" => {
                    if let Some(value) = iter.next() {
                        tls_cert = Some(value);
                    } else {
                        eprintln!("⚠️  Missing value for --tls-cert (using default).\n");
                    }
                }
                "--tls-key" => {
                    if let Some(value) = iter.next() {
                        tls_key = Some(value);
                    } else {
                        eprintln!("⚠️  Missing value for --tls-key (using default).\n");
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
            http_api,
            https,
            tls_cert,
            tls_key,
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
        Mode::RestApi => {
            run_rest_api_mode(
                config.protocol_port,
                config.http_api,
                config.https,
                config.tls_cert,
                config.tls_key,
                config.silent,
            )
            .await
        }
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

async fn run_rest_api_mode(
    port_override: Option<u16>,
    http_api_requested: bool,
    https: bool,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    silent: bool,
) {
    let rest_port = port_override
        .or_else(|| {
            env::var("SARYCHDB_HTTP_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
        })
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

    let tls_cert = tls_cert.or_else(|| env::var("SARYCHDB_TLS_CERT").ok());
    let tls_key = tls_key.or_else(|| env::var("SARYCHDB_TLS_KEY").ok());

    if https && (tls_cert.is_none() || tls_key.is_none()) {
        eprintln!(
            "❌ HTTPS mode requires --tls-cert and --tls-key or the SARYCHDB_TLS_CERT/SARYCHDB_TLS_KEY environment variables."
        );
        return;
    }

    if !silent {
        println!("🌐 SarychDB - REST API mode");
        println!("======================================");
        println!("🛰️  Starting SarychDB REST API on port {}", rest_port);
        if https {
            println!("🔐 HTTPS enabled for the REST API");
        } else if http_api_requested {
            println!("🔓 HTTP enabled for the REST API");
        }
    }

    modules::server::SarychServer::start_rest_server(rest_port, https, tls_cert, tls_key).await;
}

async fn run_benchmark_mode(nodes_override: Option<usize>, silent: bool) {
    use modules::search::{
        Item, centralized_search, get_optimal_node_count, load_json, parallel_search,
        sequential_search, smart_search, split_nodes,
    };
    use std::time::Instant;

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
