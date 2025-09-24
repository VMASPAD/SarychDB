mod modules;

use modules::server::start_server;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "benchmark" {
        // Modo benchmark (código anterior)
        run_benchmark_mode().await;
    } else {
        // Modo servidor (nuevo)
        run_server_mode().await;
    }
}

async fn run_server_mode() {
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3030".to_string())
        .parse::<u16>()
        .unwrap_or(3030);

    println!("🌟 SarychDB - Parallel Database System");
    println!("======================================");
    
    start_server(port).await;
}

async fn run_benchmark_mode() {
    use std::time::Instant;
    use modules::search::{Item, load_json, split_nodes, centralized_search, sequential_search, parallel_search};
    
    fn run_benchmark(nodes: &Vec<Vec<Item>>, queries: &[&str]) {
        for &query in queries {
            println!("\n🔎 Benchmark para query: \"{}\"", query);

            let start = Instant::now();
            let r1 = centralized_search(nodes, query);
            let t1 = start.elapsed().as_millis();

            let start = Instant::now();
            let r2 = sequential_search(nodes, query);
            let t2 = start.elapsed().as_millis();

            let start = Instant::now();
            let r3 = parallel_search(nodes, query);
            let t3 = start.elapsed().as_millis();

            println!("Centralizado: {} resultados en {} ms", r1.len(), t1);
            println!("Secuencial multinodo: {} resultados en {} ms", r2.len(), t2);
            println!("Paralelo multinodo: {} resultados en {} ms", r3.len(), t3);
        }
    }

    let num_nodes = 8;

    println!("📂 Cargando 500MB.json...");
    let items = load_json("500MB.json");
    println!("Total de registros: {}", items.len());

    let nodes = split_nodes(items, num_nodes);

    // Lista de queries a probar
    let queries = ["P1605"];

    run_benchmark(&nodes, &queries);
}
