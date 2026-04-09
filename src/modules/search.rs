use once_cell::sync::Lazy;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub type Item = Value;

// ==================== CACHE SYSTEM ====================

#[derive(Clone)]
struct CacheEntry {
    results: Vec<Value>,
    timestamp: u64,
    ttl_seconds: u64,
}

impl CacheEntry {
    fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now < self.timestamp + self.ttl_seconds
    }
}

static SEARCH_CACHE: Lazy<Arc<Mutex<HashMap<String, CacheEntry>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

fn cache_key(path: &str, query: &str) -> String {
    format!("{}:{}", path, query)
}

pub fn get_cached_search(path: &str, query: &str) -> Option<Vec<Value>> {
    let cache = SEARCH_CACHE.lock().unwrap();
    let key = cache_key(path, query);
    cache
        .get(&key)
        .filter(|e| e.is_valid())
        .map(|e| e.results.clone())
}

pub fn cache_search_results(path: &str, query: &str, results: Vec<Value>, ttl_seconds: u64) {
    let mut cache = SEARCH_CACHE.lock().unwrap();
    let key = cache_key(path, query);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    cache.insert(
        key,
        CacheEntry {
            results,
            timestamp,
            ttl_seconds,
        },
    );

    // Auto cleanup when cache grows too large
    if cache.len() > 100 {
        cache.retain(|_, entry| entry.is_valid());
    }
}

pub fn invalidate_cache_for_path(path: &str) {
    let mut cache = SEARCH_CACHE.lock().unwrap();
    let prefix = format!("{}:", path);
    cache.retain(|key, _| !key.starts_with(&prefix));
}

pub fn clear_search_cache() {
    let mut cache = SEARCH_CACHE.lock().unwrap();
    cache.clear();
}

// ==================== DATA LOADING ====================

/// Load a JSON array from disk. Returns Err instead of panicking on failure.
pub fn load_json(path: &str) -> Result<Vec<Item>, String> {
    let data =
        fs::read_to_string(path).map_err(|e| format!("Cannot read file '{}': {}", path, e))?;
    serde_json::from_str::<Vec<Value>>(&data)
        .map_err(|e| format!("JSON parse error in '{}': {}", path, e))
}

// ==================== NODE SPLITTING ====================

/// Split data into chunks optimized for parallel processing.
pub fn split_nodes(items: Vec<Item>, num_nodes: usize) -> Vec<Vec<Item>> {
    if items.is_empty() {
        return vec![];
    }
    let optimal_nodes = if num_nodes == 0 {
        rayon::current_num_threads()
    } else {
        num_nodes
    };

    // Integer ceiling division, clamped to at least 1
    let chunk_size = ((items.len() + optimal_nodes - 1) / optimal_nodes).max(1);
    items.chunks(chunk_size).map(|c| c.to_vec()).collect()
}

// ==================== SEARCH FUNCTIONS ====================

/// Recursive case-sensitive search across any JSON value.
pub fn search_in_json_value(value: &Value, query: &str) -> bool {
    match value {
        Value::String(s) => s.contains(query),
        Value::Number(n) => n.to_string().contains(query),
        Value::Bool(b) => b.to_string().contains(query),
        Value::Array(arr) => arr.iter().any(|item| search_in_json_value(item, query)),
        Value::Object(obj) => obj.values().any(|v| search_in_json_value(v, query)),
        Value::Null => false,
    }
}

/// Recursive case-insensitive search across any JSON value.
/// `query_lower` must already be lowercased by the caller to avoid redundant allocations.
pub fn search_in_json_value_ci(value: &Value, query_lower: &str) -> bool {
    match value {
        Value::String(s) => s.to_lowercase().contains(query_lower),
        Value::Number(n) => n.to_string().contains(query_lower),
        Value::Bool(b) => b.to_string().contains(query_lower),
        Value::Array(arr) => arr
            .iter()
            .any(|item| search_in_json_value_ci(item, query_lower)),
        Value::Object(obj) => obj
            .values()
            .any(|v| search_in_json_value_ci(v, query_lower)),
        Value::Null => false,
    }
}

pub fn search_node<'a>(node: &'a [Item], query: &str) -> Vec<&'a Item> {
    node.iter()
        .filter(|item| search_in_json_value(item, query))
        .collect()
}

// ==================== SEARCH MODES ====================

/// Centralized search: flattens all nodes into one pass (small datasets).
pub fn centralized_search<'a>(nodes: &'a [Vec<Item>], query: &str) -> Vec<&'a Item> {
    nodes
        .iter()
        .flat_map(|n| n.iter())
        .filter(|item| search_in_json_value(item, query))
        .collect()
}

/// Sequential multi-node search (small datasets, no threading overhead).
pub fn sequential_search<'a>(nodes: &'a [Vec<Item>], query: &str) -> Vec<&'a Item> {
    nodes.iter().flat_map(|n| search_node(n, query)).collect()
}

/// Parallel multi-node search using all available CPU cores.
pub fn parallel_search<'a>(nodes: &'a [Vec<Item>], query: &str) -> Vec<&'a Item> {
    nodes
        .par_iter()
        .flat_map(|n| search_node(n, query))
        .collect()
}

// ==================== CACHED SEARCH ====================

/// Parallel search with automatic caching. Cache miss triggers a full parallel
/// search and stores the result for `ttl_seconds`.
pub fn cached_parallel_search(
    path: &str,
    nodes: &[Vec<Item>],
    query: &str,
    ttl_seconds: u64,
) -> Vec<Value> {
    if let Some(cached) = get_cached_search(path, query) {
        return cached;
    }

    let results: Vec<Value> = parallel_search(nodes, query).into_iter().cloned().collect();

    cache_search_results(path, query, results.clone(), ttl_seconds);
    results
}

/// Auto-selects sequential or parallel search based on dataset size.
pub fn smart_search<'a>(nodes: &'a [Vec<Item>], query: &str) -> Vec<&'a Item> {
    let total_items: usize = nodes.iter().map(|n| n.len()).sum();
    if total_items < 1000 {
        sequential_search(nodes, query)
    } else {
        parallel_search(nodes, query)
    }
}

// ==================== UTILITIES ====================

pub fn get_optimal_node_count() -> usize {
    rayon::current_num_threads()
}

pub fn configure_thread_pool(num_threads: Option<usize>) {
    if let Some(threads) = num_threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .unwrap_or_else(|_| {
                eprintln!("Warning: Could not configure thread pool");
            });
    }
}
