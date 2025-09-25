use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::Instant;
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
            return Err("Database does not exist".to_string());
        }
        
        Ok(load_json(&filepath))
    }

    pub fn write_database(username: &str, db_name: &str, data: &Vec<Value>) -> Result<(), String> {
        let filepath = Self::get_db_path(username, db_name);
        let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
        fs::write(&filepath, json).map_err(|e| e.to_string())
    }

    // GET - Search records with queryType support  
    pub fn search_records(&self, username: &str, db_name: &str, query: Option<&str>, query_type: Option<&str>) -> Result<Vec<Value>, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let data = Self::read_database(username, db_name)?;
        
        match query {
            Some(q) if !q.is_empty() => {
                let results = match query_type {
                    Some("key") => self.search_by_key(&data, q),
                    Some("value") => self.search_by_value(&data, q),
                    _ => {
                        // Default behavior - search in entire structure
                        let nodes = split_nodes(data, 4);
                        parallel_search(&nodes, q).into_iter().cloned().collect()
                    }
                };
                Ok(results)
            },
            _ => {
                // No query, return all records
                Ok(data)
            }
        }
    }

    // Search by specific key name
    fn search_by_key(&self, data: &[Value], key_name: &str) -> Vec<Value> {
        data.iter()
            .filter(|item| {
                if let Value::Object(obj) = item {
                    obj.contains_key(key_name)
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    // Search by value in any part of the structure
    fn search_by_value(&self, data: &[Value], search_value: &str) -> Vec<Value> {
        data.iter()
            .filter(|item| self.search_in_json_value(item, search_value))
            .cloned()
            .collect()
    }

    // POST - Insert new record
    pub fn insert_record(&self, username: &str, db_name: &str, mut record: Value) -> Result<String, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let mut data = Self::read_database(username, db_name)?;
        
        // Add metadata to record
        if let Value::Object(ref mut obj) = record {
            obj.insert("_id".to_string(), Value::String(Uuid::new_v4().to_string()));
            obj.insert("_created_at".to_string(), Value::String(Utc::now().to_rfc3339()));
        }

        data.push(record);
        Self::write_database(username, db_name, &data)?;
        
        Ok("Record inserted successfully".to_string())
    }

    // PUT - Update records with ID support
    pub fn update_records(&self, username: &str, db_name: &str, query: &str, update_data: Value, id_update: Option<&str>) -> Result<String, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let mut data = Self::read_database(username, db_name)?;
        let mut updated_count = 0;

        // Update by specific ID if provided
        if let Some(target_id) = id_update {
            for item in &mut data {
                if let &mut Value::Object(ref obj) = item {
                    if let Some(Value::String(id)) = obj.get("_id") {
                        if id == target_id {
                            if let (Value::Object(target), Value::Object(source)) = (item, &update_data) {
                                // Update fields from update_data
                                for (key, value) in source {
                                    target.insert(key.clone(), value.clone());
                                }
                                // Add update timestamp
                                target.insert("_updated_at".to_string(), Value::String(Utc::now().to_rfc3339()));
                                updated_count += 1;
                                break; // Only update one record when using ID
                            }
                        }
                    }
                }
            }
        } else {
            // Update by query (existing behavior)
            for item in &mut data {
                if self.item_matches_query(item, query) {
                    if let (Value::Object(target), Value::Object(source)) = (item, &update_data) {
                        // Update fields
                        for (key, value) in source {
                            target.insert(key.clone(), value.clone());
                        }
                        // Add update timestamp
                        target.insert("_updated_at".to_string(), Value::String(Utc::now().to_rfc3339()));
                        updated_count += 1;
                    }
                }
            }
        }

        Self::write_database(username, db_name, &data)?;
        Ok(format!("Updated {} records", updated_count))
    }

    // DELETE - Delete records matching query
    pub fn delete_records(&self, username: &str, db_name: &str, query: &str) -> Result<String, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let mut data = Self::read_database(username, db_name)?;
        let initial_count = data.len();
        
        // Filter records that DON'T match the query (delete those that DO match)
        data.retain(|item| !self.item_matches_query(item, query));
        
        let deleted_count = initial_count - data.len();
        Self::write_database(username, db_name, &data)?;
        
        Ok(format!("Deleted {} records", deleted_count))
    }

    // Helper function to check if an item matches the query
    fn item_matches_query(&self, item: &Value, query: &str) -> bool {
        self.search_in_json_value(item, query)
    }

    // Recursive function to search in JSON values
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

    // Get database statistics with read time measurement
    pub fn get_stats(&self, username: &str, db_name: &str) -> Result<Value, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        // Measure file read time
        let read_start = Instant::now();
        let data = Self::read_database(username, db_name)?;
        let read_time_ms = read_start.elapsed().as_millis();

        let filepath = Self::get_db_path(username, db_name);
        let stats = serde_json::json!({
            "database": db_name,
            "username": username,
            "total_records": data.len(),
            "size_bytes": fs::metadata(&filepath)
                .map(|m| m.len())
                .unwrap_or(0),
            "read_time_ms": read_time_ms,
            "timestamp": Utc::now().to_rfc3339()
        });

        Ok(stats)
    }
}