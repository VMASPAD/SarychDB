use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::Instant;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use crate::modules::search::{
    load_json, split_nodes, 
    get_optimal_node_count,
    invalidate_cache_for_path, cached_parallel_search
};
use uuid::Uuid;
use chrono::Utc;

// Simple cache structure with once_cell
static DB_CACHE: Lazy<Mutex<HashMap<String, (Vec<Value>, Instant)>>> = Lazy::new(|| Mutex::new(HashMap::new()));

const CACHE_TTL_SECS: u64 = 300; // 5 minutes cache

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

    // Read database with cache support
    pub fn read_database_cached(username: &str, db_name: &str) -> Result<Vec<Value>, String> {
        let cache_key = format!("{}:{}", username, db_name);
        
        // Try to get from cache
        {
            let cache = DB_CACHE.lock().unwrap();
            if let Some((data, timestamp)) = cache.get(&cache_key) {
                // Check if cache is still valid (within TTL)
                if timestamp.elapsed().as_secs() < CACHE_TTL_SECS {
                    return Ok(data.clone());
                }
            }
        }
        
        // Cache miss or expired, read from disk
        let data = Self::read_database(username, db_name)?;
        
        // Update cache
        {
            let mut cache = DB_CACHE.lock().unwrap();
            cache.insert(cache_key, (data.clone(), Instant::now()));
        }
        
        Ok(data)
    }

    // Invalidate cache when data is written
    pub fn invalidate_cache(username: &str, db_name: &str) {
        let cache_key = format!("{}:{}", username, db_name);
        let mut cache = DB_CACHE.lock().unwrap();
        cache.remove(&cache_key);
    }

    pub fn write_database(username: &str, db_name: &str, data: &Vec<Value>) -> Result<(), String> {
        let filepath = Self::get_db_path(username, db_name);
        let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
        fs::write(&filepath, json).map_err(|e| e.to_string())?;
        
        // Invalidate both database cache and search cache after write
        Self::invalidate_cache(username, db_name);
        invalidate_cache_for_path(&filepath);
        Ok(())
    }

    // GET - Search records with queryType support and optimized parallel search
    pub fn search_records(&self, username: &str, db_name: &str, query: Option<&str>, query_type: Option<&str>) -> Result<Vec<Value>, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let data = Self::read_database_cached(username, db_name)?;
        
        match query {
            Some(q) if !q.is_empty() => {
                let results = match query_type {
                    Some("key") => self.search_by_key(&data, q),
                    Some("value") => self.search_by_value(&data, q),
                    _ => {
                        // Use intelligent search with cache
                        // Get optimal node count based on CPU cores
                        let node_count = get_optimal_node_count();
                        let nodes = split_nodes(data, node_count);
                        
                        // Use cached parallel search with 5-minute TTL
                        let filepath = Self::get_db_path(username, db_name);
                        cached_parallel_search(&filepath, &nodes, q, 300)
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

        let mut data = Self::read_database_cached(username, db_name)?;
        
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

        let mut data = Self::read_database_cached(username, db_name)?;
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

        let mut data = Self::read_database_cached(username, db_name)?;
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

    // BROWSE - Simple paginated GET of all records
    // Nueva lógica:
    // - Si solo hay limit (sin page): devuelve los primeros N registros
    // - Si hay limit y page: devuelve la página especificada con ese límite por página
    pub fn browse_records(
        &self,
        username: &str,
        db_name: &str,
        page: Option<usize>,
        limit: Option<usize>
    ) -> Result<Value, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }
        
        let data = Self::read_database_cached(username, db_name)?;
        let total_records = data.len();

        match (page, limit) {
            // Case 1: Solo limit, sin page -> Devolver los primeros N registros
            (None, Some(lim)) => {
                let paginated_data: Vec<Value> = data
                    .into_iter()
                    .take(lim)
                    .collect();
                
                let returned_count = paginated_data.len();
                
                Ok(serde_json::json!({
                    "data": paginated_data,
                    "pagination": {
                        "limit": lim,
                        "returned": returned_count,
                        "total_records": total_records,
                        "mode": "limit_only"
                    }
                }))
            },
            // Case 2: limit y page -> Paginación normal
            (Some(p), Some(lim)) => {
                let page_num = p.max(1); // Asegurar que page sea al menos 1
                let offset = (page_num - 1) * lim;
                
                let paginated_data: Vec<Value> = data
                    .into_iter()
                    .skip(offset)
                    .take(lim)
                    .collect();
                
                let total_pages = if lim > 0 { 
                    (total_records as f64 / lim as f64).ceil() as usize 
                } else { 
                    0 
                };
                let returned_count = paginated_data.len();
                
                Ok(serde_json::json!({
                    "data": paginated_data,
                    "pagination": {
                        "page": page_num,
                        "limit": lim,
                        "returned": returned_count,
                        "total_records": total_records,
                        "total_pages": total_pages,
                        "has_next": page_num < total_pages,
                        "has_prev": page_num > 1,
                        "mode": "paginated"
                    }
                }))
            },
            // Case 3: page sin limit -> Error (necesita limit para paginar)
            (Some(_), None) => {
                Err("Cannot use 'page' without 'limit'. Please provide both parameters.".to_string())
            },
            // Case 4: Ni page ni limit -> Devolver primeros 10 registros (default)
            (None, None) => {
                let default_limit = 10;
                let paginated_data: Vec<Value> = data
                    .into_iter()
                    .take(default_limit)
                    .collect();
                
                let returned_count = paginated_data.len();
                let total_pages = (total_records as f64 / default_limit as f64).ceil() as usize;
                
                Ok(serde_json::json!({
                    "data": paginated_data,
                    "pagination": {
                        "page": 1,
                        "limit": default_limit,
                        "returned": returned_count,
                        "total_records": total_records,
                        "total_pages": total_pages,
                        "has_next": total_pages > 1,
                        "has_prev": false,
                        "mode": "default"
                    }
                }))
            }
        }
    }

    // LIST - Advanced search with pagination, sorting, and filtering
    pub fn list_records(
        &self,
        username: &str,
        db_name: &str,
        page: Option<usize>,
        limit: Option<usize>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
        filters: Option<&Value>
    ) -> Result<Value, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let mut data = Self::read_database_cached(username, db_name)?;
        let total_records = data.len();

        // Apply filters if provided
        if let Some(filter_obj) = filters {
            if let Value::Object(filters_map) = filter_obj {
                data.retain(|item| self.matches_filters(item, filters_map));
            }
        }

        let filtered_count = data.len();

        // Apply sorting if provided
        if let Some(field) = sort_by {
            let order = sort_order.unwrap_or("asc");
            data.sort_by(|a, b| self.compare_values(a, b, field, order));
        }

        // Apply pagination
        let page_num = page.unwrap_or(1);
        let page_size = limit.unwrap_or(10);
        let offset = (page_num.saturating_sub(1)) * page_size;
        
        let paginated_data: Vec<Value> = data
            .into_iter()
            .skip(offset)
            .take(page_size)
            .collect();

        let total_pages = (filtered_count as f64 / page_size as f64).ceil() as usize;

        Ok(serde_json::json!({
            "data": paginated_data,
            "pagination": {
                "page": page_num,
                "limit": page_size,
                "total_records": total_records,
                "filtered_records": filtered_count,
                "total_pages": total_pages,
                "has_next": page_num < total_pages,
                "has_prev": page_num > 1
            },
            "sorting": {
                "field": sort_by,
                "order": sort_order.unwrap_or("asc")
            }
        }))
    }

    // Check if item matches all filters
    fn matches_filters(&self, item: &Value, filters: &serde_json::Map<String, Value>) -> bool {
        for (key, filter_value) in filters {
            if let Value::Object(obj) = item {
                match obj.get(key) {
                    Some(item_value) => {
                        if !self.value_matches_filter(item_value, filter_value) {
                            return false;
                        }
                    }
                    None => return false,
                }
            } else {
                return false;
            }
        }
        true
    }

    // Compare filter value with item value (supports exact match and arrays)
    fn value_matches_filter(&self, item_value: &Value, filter_value: &Value) -> bool {
        match filter_value {
            Value::Array(arr) => {
                // If filter is array, item value must be one of the array values (OR logic)
                arr.iter().any(|fv| item_value == fv)
            }
            _ => item_value == filter_value
        }
    }

    // Compare two items by a specific field for sorting
    fn compare_values(&self, a: &Value, b: &Value, field: &str, order: &str) -> std::cmp::Ordering {
        let a_val = self.get_field_value(a, field);
        let b_val = self.get_field_value(b, field);

        let comparison = match (a_val, b_val) {
            (Some(Value::String(s1)), Some(Value::String(s2))) => s1.cmp(s2),
            (Some(Value::Number(n1)), Some(Value::Number(n2))) => {
                let f1 = n1.as_f64().unwrap_or(0.0);
                let f2 = n2.as_f64().unwrap_or(0.0);
                f1.partial_cmp(&f2).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Some(Value::Bool(b1)), Some(Value::Bool(b2))) => b1.cmp(b2),
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        };

        if order == "desc" {
            comparison.reverse()
        } else {
            comparison
        }
    }

    // Get field value from item
    fn get_field_value<'a>(&self, item: &'a Value, field: &str) -> Option<&'a Value> {
        if let Value::Object(obj) = item {
            obj.get(field)
        } else {
            None
        }
    }

    // Get database statistics with read time measurement
    // Get database statistics with read time measurement
    pub fn get_stats(&self, username: &str, db_name: &str) -> Result<Value, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        // Measure file read time (with cache)
        let read_start = Instant::now();
        let data = Self::read_database_cached(username, db_name)?;
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
            "cached": true,
            "timestamp": Utc::now().to_rfc3339()
        });

        Ok(stats)
    }
}