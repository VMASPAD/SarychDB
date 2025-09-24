use rayon::prelude::*;
use serde_json::Value;
use std::fs;

// Alias para el tipo de datos flexible
pub type Item = Value;

pub fn load_json(path: &str) -> Vec<Item> {
    let data = fs::read_to_string(path).expect("No se pudo leer el archivo JSON");
    serde_json::from_str::<Vec<Value>>(&data).expect("Error al parsear JSON")
}

// Dividir datos en nodos simulados
pub fn split_nodes(items: Vec<Item>, num_nodes: usize) -> Vec<Vec<Item>> {
    let chunk_size = (items.len() as f64 / num_nodes as f64).ceil() as usize;
    items.chunks(chunk_size).map(|c| c.to_vec()).collect()
}

// Función para buscar en todo el objeto JSON
fn item_contains_value(item: &Item, query: &str) -> bool {
    search_in_json_value(item, query)
}

// Función recursiva para buscar en cualquier valor JSON
fn search_in_json_value(value: &Value, query: &str) -> bool {
    match value {
        Value::String(s) => s.contains(query),
        Value::Number(n) => n.to_string().contains(query),
        Value::Bool(b) => b.to_string().contains(query),
        Value::Array(arr) => arr.iter().any(|v| search_in_json_value(v, query)),
        Value::Object(obj) => obj.values().any(|v| search_in_json_value(v, query)),
        Value::Null => false,
    }
}

// Búsqueda en un nodo
pub fn search_node<'a>(node: &'a Vec<Item>, query: &str) -> Vec<&'a Item> {
    node.iter().filter(|item| item_contains_value(item, query)).collect()
}

// Centralizado: todos los datos en un vector
pub fn centralized_search<'a>(nodes: &'a Vec<Vec<Item>>, query: &str) -> Vec<&'a Item> {
    let all: Vec<&Item> = nodes.iter().flat_map(|n| n.iter()).collect();
    all.into_iter().filter(|item| item_contains_value(item, query)).collect()
}

// Secuencial multinodo
pub fn sequential_search<'a>(nodes: &'a Vec<Vec<Item>>, query: &str) -> Vec<&'a Item> {
    nodes.iter().flat_map(|n| search_node(n, query)).collect()
}

// Paralelo multinodo
pub fn parallel_search<'a>(nodes: &'a Vec<Vec<Item>>, query: &str) -> Vec<&'a Item> {
    nodes.par_iter().flat_map(|n| search_node(n, query)).collect()
}
