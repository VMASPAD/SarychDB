use rayon::prelude::*;
use serde_json::Value;
use std::fs;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use once_cell::sync::Lazy;

// Alias para el tipo de datos flexible
pub type Item = Value;

// ==================== CACHE SYSTEM ====================

/// Cache entry with TTL
#[derive(Clone)]
struct CacheEntry {
    query: String,
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

/// Global search cache with automatic cleanup
static SEARCH_CACHE: Lazy<Arc<Mutex<HashMap<String, CacheEntry>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Generate cache key from path and query
fn cache_key(path: &str, query: &str) -> String {
    format!("{}:{}", path, query)
}

/// Get cached search results if valid
pub fn get_cached_search(path: &str, query: &str) -> Option<Vec<Value>> {
    let cache = SEARCH_CACHE.lock().unwrap();
    let key = cache_key(path, query);
    
    if let Some(entry) = cache.get(&key) {
        if entry.is_valid() {
            return Some(entry.results.clone());
        }
    }
    None
}

/// Store search results in cache
pub fn cache_search_results(path: &str, query: &str, results: Vec<Value>, ttl_seconds: u64) {
    let mut cache = SEARCH_CACHE.lock().unwrap();
    let key = cache_key(path, query);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    cache.insert(key, CacheEntry {
        query: query.to_string(),
        results: results.clone(),
        timestamp,
        ttl_seconds,
    });
    
    // Auto cleanup: remove expired entries if cache is too large
    if cache.len() > 100 {
        cache.retain(|_, entry| entry.is_valid());
    }
}

/// Clear all cache entries for a specific database file
pub fn invalidate_cache_for_path(path: &str) {
    let mut cache = SEARCH_CACHE.lock().unwrap();
    cache.retain(|key, _| !key.starts_with(&format!("{}:", path)));
}

/// Clear entire search cache
pub fn clear_search_cache() {
    let mut cache = SEARCH_CACHE.lock().unwrap();
    cache.clear();
}

// ==================== DATA LOADING ====================

pub fn load_json(path: &str) -> Vec<Item> {
    let data = fs::read_to_string(path).expect("No se pudo leer el archivo JSON");
    serde_json::from_str::<Vec<Value>>(&data).expect("Error al parsear JSON")
}


// ==================== NODE SPLITTING ====================

/// Divide datos en chunks optimizados para procesamiento paralelo
/// Usa el número de CPUs disponibles para maximizar el uso del procesador
pub fn split_nodes(items: Vec<Item>, num_nodes: usize) -> Vec<Vec<Item>> {
    // Si num_nodes es 0, usar el número de CPUs lógicos disponibles
    let optimal_nodes = if num_nodes == 0 {
        rayon::current_num_threads()
    } else {
        num_nodes
    };
    
    let chunk_size = (items.len() as f64 / optimal_nodes as f64).ceil() as usize;
    items.chunks(chunk_size).map(|c| c.to_vec()).collect()
}

// ==================== SEARCH FUNCTIONS ====================

/// Función recursiva optimizada para buscar en cualquier valor JSON
/// Usa early returns para mejorar performance
fn search_in_json_value(value: &Value, query: &str) -> bool {
    match value {
        Value::String(s) => s.contains(query),
        Value::Number(n) => n.to_string().contains(query),
        Value::Bool(b) => b.to_string().contains(query),
        Value::Array(arr) => {
            // Early return: detiene en el primer match encontrado
            for item in arr {
                if search_in_json_value(item, query) {
                    return true;
                }
            }
            false
        },
        Value::Object(obj) => {
            // Early return: detiene en el primer match encontrado
            for value in obj.values() {
                if search_in_json_value(value, query) {
                    return true;
                }
            }
            false
        },
        Value::Null => false,
    }
}

/// Buscar en un item completo
fn item_contains_value(item: &Item, query: &str) -> bool {
    search_in_json_value(item, query)
}

/// Búsqueda en un solo nodo (secuencial dentro del nodo)
pub fn search_node<'a>(node: &'a Vec<Item>, query: &str) -> Vec<&'a Item> {
    node.iter()
        .filter(|item| item_contains_value(item, query))
        .collect()
}

// ==================== SEARCH MODES ====================

/// Centralizado: todos los datos en un vector (para datasets pequeños)
pub fn centralized_search<'a>(nodes: &'a Vec<Vec<Item>>, query: &str) -> Vec<&'a Item> {
    let all: Vec<&Item> = nodes.iter().flat_map(|n| n.iter()).collect();
    all.into_iter()
        .filter(|item| item_contains_value(item, query))
        .collect()
}

/// Secuencial multinodo (para datasets pequeños sin overhead de threading)
pub fn sequential_search<'a>(nodes: &'a Vec<Vec<Item>>, query: &str) -> Vec<&'a Item> {
    nodes.iter()
        .flat_map(|n| search_node(n, query))
        .collect()
}

/// Paralelo multinodo optimizado (usa todos los cores del CPU)
/// Esta es la opción recomendada para datasets grandes
pub fn parallel_search<'a>(nodes: &'a Vec<Vec<Item>>, query: &str) -> Vec<&'a Item> {
    nodes.par_iter()
        .flat_map(|n| search_node(n, query))
        .collect()
}

// ==================== CACHED SEARCH (HIGH LEVEL) ====================

/// Búsqueda con cache automático
/// Primero busca en cache, si no existe realiza búsqueda paralela y cachea el resultado
pub fn cached_parallel_search(
    path: &str,
    nodes: &Vec<Vec<Item>>,
    query: &str,
    ttl_seconds: u64
) -> Vec<Value> {
    // Intenta obtener del cache
    if let Some(cached) = get_cached_search(path, query) {
        return cached;
    }
    
    // Si no está en cache, realiza búsqueda paralela
    let results: Vec<Value> = parallel_search(nodes, query)
        .into_iter()
        .cloned()
        .collect();
    
    // Cachea los resultados
    cache_search_results(path, query, results.clone(), ttl_seconds);
    
    results
}

/// Búsqueda inteligente que elige el mejor método según el tamaño del dataset
pub fn smart_search<'a>(nodes: &'a Vec<Vec<Item>>, query: &str) -> Vec<&'a Item> {
    let total_items: usize = nodes.iter().map(|n| n.len()).sum();
    
    // Para datasets pequeños (<1000 items), usar secuencial
    // Para datasets medianos (1000-10000), usar paralelo simple
    // Para datasets grandes (>10000), usar paralelo con optimizaciones
    if total_items < 1000 {
        sequential_search(nodes, query)
    } else {
        parallel_search(nodes, query)
    }
}

// ==================== UTILITIES ====================

/// Obtiene el número óptimo de nodos basado en el CPU
pub fn get_optimal_node_count() -> usize {
    rayon::current_num_threads()
}

/// Configura el thread pool de Rayon para uso óptimo del CPU
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
