use serde_json::Value;
use std::fs;
use std::path::Path;
use crate::modules::search::{load_json, split_nodes, parallel_search};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct DatabaseManager;

impl DatabaseManager {
    pub fn new() -> Self {
        Self
    }

    pub fn get_db_path(username: &str, db_name: &str) -> String {
        format!("users/{}/{}.json", username, db_name)
    }

    pub fn database_exists(username: &str, db_name: &str) -> bool {
        let filepath = Self::get_db_path(username, db_name);
        Path::new(&filepath).exists()
    }

    pub fn read_database(username: &str, db_name: &str) -> Result<Vec<Value>, String> {
        let filepath = Self::get_db_path(username, db_name);
        if !Self::database_exists(username, db_name) {
            return Err("Base de datos no existe".to_string());
        }
        
        Ok(load_json(&filepath))
    }

    pub fn write_database(username: &str, db_name: &str, data: &Vec<Value>) -> Result<(), String> {
        let filepath = Self::get_db_path(username, db_name);
        let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
        fs::write(&filepath, json).map_err(|e| e.to_string())
    }



    // GET - Buscar registros
    pub fn search_records(&self, username: &str, db_name: &str, query: Option<&str>) -> Result<Vec<Value>, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Base de datos no existe".to_string());
        }

        let data = Self::read_database(username, db_name)?;
        
        match query {
            Some(q) if !q.is_empty() => {
                // Usar el motor de búsqueda paralelo para queries específicas
                let nodes = split_nodes(data, 4); // 4 nodos para búsqueda
                let results = parallel_search(&nodes, q);
                Ok(results.into_iter().cloned().collect())
            },
            _ => {
                // Sin query, devolver todos los registros
                Ok(data)
            }
        }
    }

    // POST - Insertar nuevo registro
    pub fn insert_record(&self, username: &str, db_name: &str, mut record: Value) -> Result<String, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Base de datos no existe".to_string());
        }

        let mut data = Self::read_database(username, db_name)?;
        
        // Agregar metadatos al registro
        if let Value::Object(ref mut obj) = record {
            obj.insert("_id".to_string(), Value::String(Uuid::new_v4().to_string()));
            obj.insert("_created_at".to_string(), Value::String(Utc::now().to_rfc3339()));
        }

        data.push(record);
        Self::write_database(username, db_name, &data)?;
        
        Ok("Registro insertado exitosamente".to_string())
    }

    // PUT - Actualizar registros que coincidan con query
    pub fn update_records(&self, username: &str, db_name: &str, query: &str, update_data: Value) -> Result<String, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Base de datos no existe".to_string());
        }

        let mut data = Self::read_database(username, db_name)?;
        let mut updated_count = 0;

        // Buscar y actualizar registros
        for item in &mut data {
            if self.item_matches_query(item, query) {
                if let (Value::Object(target), Value::Object(source)) = (item, &update_data) {
                    // Actualizar campos
                    for (key, value) in source {
                        target.insert(key.clone(), value.clone());
                    }
                    // Agregar timestamp de actualización
                    target.insert("_updated_at".to_string(), Value::String(Utc::now().to_rfc3339()));
                    updated_count += 1;
                }
            }
        }

        Self::write_database(username, db_name, &data)?;
        Ok(format!("Se actualizaron {} registros", updated_count))
    }

    // DELETE - Eliminar registros que coincidan con query
    pub fn delete_records(&self, username: &str, db_name: &str, query: &str) -> Result<String, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Base de datos no existe".to_string());
        }

        let mut data = Self::read_database(username, db_name)?;
        let initial_count = data.len();
        
        // Filtrar registros que NO coincidan con la query (eliminar los que sí coincidan)
        data.retain(|item| !self.item_matches_query(item, query));
        
        let deleted_count = initial_count - data.len();
        Self::write_database(username, db_name, &data)?;
        
        Ok(format!("Se eliminaron {} registros", deleted_count))
    }

    // Función auxiliar para verificar si un item coincide con la query
    fn item_matches_query(&self, item: &Value, query: &str) -> bool {
        self.search_in_json_value(item, query)
    }

    // Función recursiva para buscar en valores JSON (copiada del módulo search)
    fn search_in_json_value(&self, value: &Value, query: &str) -> bool {
        match value {
            Value::String(s) => s.contains(query),
            Value::Number(n) => n.to_string().contains(query),
            Value::Bool(b) => b.to_string().contains(query),
            Value::Array(arr) => arr.iter().any(|v| self.search_in_json_value(v, query)),
            Value::Object(obj) => obj.values().any(|v| self.search_in_json_value(v, query)),
            Value::Null => false,
        }
    }

    // Obtener estadísticas de la base de datos
    pub fn get_stats(&self, username: &str, db_name: &str) -> Result<Value, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Base de datos no existe".to_string());
        }

        let data = Self::read_database(username, db_name)?;
        let filepath = Self::get_db_path(username, db_name);
        let stats = serde_json::json!({
            "database": db_name,
            "username": username,
            "total_records": data.len(),
            "size_bytes": fs::metadata(&filepath)
                .map(|m| m.len())
                .unwrap_or(0),
            "timestamp": Utc::now().to_rfc3339()
        });

        Ok(stats)
    }
}