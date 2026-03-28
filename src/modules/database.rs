use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use std::time::Instant;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use crate::modules::search::{
    load_json, split_nodes,
    get_optimal_node_count,
    invalidate_cache_for_path, cached_parallel_search,
    search_in_json_value, search_in_json_value_ci,
};
use uuid::Uuid;
use chrono::Utc;

static DB_CACHE: Lazy<Mutex<HashMap<String, (Vec<Value>, Instant)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

const CACHE_TTL_SECS: u64 = 300; // 5 minutes

#[derive(Debug, Clone)]
pub struct DatabaseManager;

impl DatabaseManager {
    pub fn new() -> Self {
        Self
    }

    fn data_root() -> PathBuf {
        env::var("SARYCHDB_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::document_dir()
                    .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
                    .join("SarychDB")
            })
    }

    pub fn get_db_path(username: &str, db_name: &str) -> String {
        let path = Self::data_root()
            .join("users")
            .join(username)
            .join(format!("{}.json", db_name));
        path.to_string_lossy().to_string()
    }

    pub fn database_exists(username: &str, db_name: &str) -> bool {
        let filepath = Self::get_db_path(username, db_name);
        Path::new(&filepath).exists()
    }

    pub fn count_records(&self, username: &str, db_name: &str) -> Result<usize, String> {
        let data = Self::read_database_cached(username, db_name)?;
        Ok(data.len())
    }

    pub fn read_database(username: &str, db_name: &str) -> Result<Vec<Value>, String> {
        let filepath = Self::get_db_path(username, db_name);
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }
        load_json(&filepath)
    }

    /// Read database with 5-minute in-memory cache.
    pub fn read_database_cached(username: &str, db_name: &str) -> Result<Vec<Value>, String> {
        let cache_key = format!("{}:{}", username, db_name);

        {
            let cache = DB_CACHE.lock().unwrap();
            if let Some((data, timestamp)) = cache.get(&cache_key) {
                if timestamp.elapsed().as_secs() < CACHE_TTL_SECS {
                    return Ok(data.clone());
                }
            }
        }

        let data = Self::read_database(username, db_name)?;

        {
            let mut cache = DB_CACHE.lock().unwrap();
            cache.insert(cache_key, (data.clone(), Instant::now()));
        }

        Ok(data)
    }

    pub fn invalidate_cache(username: &str, db_name: &str) {
        let cache_key = format!("{}:{}", username, db_name);
        let mut cache = DB_CACHE.lock().unwrap();
        cache.remove(&cache_key);
    }

    /// Returns true if this database is currently in the warm cache.
    fn is_cached(username: &str, db_name: &str) -> bool {
        let cache_key = format!("{}:{}", username, db_name);
        let cache = DB_CACHE.lock().unwrap();
        cache
            .get(&cache_key)
            .map(|(_, ts)| ts.elapsed().as_secs() < CACHE_TTL_SECS)
            .unwrap_or(false)
    }

    pub fn write_database(username: &str, db_name: &str, data: &[Value]) -> Result<(), String> {
        let filepath = Self::get_db_path(username, db_name);
        let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
        fs::write(&filepath, json).map_err(|e| e.to_string())?;

        Self::invalidate_cache(username, db_name);
        invalidate_cache_for_path(&filepath);
        Ok(())
    }

    // GET - Search records with optional queryType
    pub fn search_records(
        &self,
        username: &str,
        db_name: &str,
        query: Option<&str>,
        query_type: Option<&str>,
    ) -> Result<Vec<Value>, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let data = Self::read_database_cached(username, db_name)?;

        match query {
            Some(q) if !q.is_empty() => {
                let results = match query_type {
                    Some("key") => self.search_by_key(&data, q),
                    Some("value") => data
                        .iter()
                        .filter(|item| search_in_json_value(item, q))
                        .cloned()
                        .collect(),
                    Some("icontains") => {
                        let q_lower = q.to_lowercase();
                        data.iter()
                            .filter(|item| search_in_json_value_ci(item, &q_lower))
                            .cloned()
                            .collect()
                    }
                    _ => {
                        // Full-text parallel search with caching
                        let node_count = get_optimal_node_count();
                        let nodes = split_nodes(data, node_count);
                        let filepath = Self::get_db_path(username, db_name);
                        cached_parallel_search(&filepath, &nodes, q, 300)
                    }
                };
                Ok(results)
            }
            _ => Ok(data),
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

    // POST - Insert new record
    pub fn insert_record(
        &self,
        username: &str,
        db_name: &str,
        mut record: Value,
    ) -> Result<String, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let mut data = Self::read_database_cached(username, db_name)?;

        if let Value::Object(ref mut obj) = record {
            obj.insert("_id".to_string(), Value::String(Uuid::new_v4().to_string()));
            obj.insert(
                "_created_at".to_string(),
                Value::String(Utc::now().to_rfc3339()),
            );
        }

        data.push(record);
        Self::write_database(username, db_name, &data)?;
        Ok("Record inserted successfully".to_string())
    }

    // PUT - Update records by query or by _id
    pub fn update_records(
        &self,
        username: &str,
        db_name: &str,
        query: &str,
        update_data: Value,
        id_update: Option<&str>,
    ) -> Result<String, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let mut data = Self::read_database_cached(username, db_name)?;
        let mut updated_count = 0;

        if let Some(target_id) = id_update {
            for item in &mut data {
                if let Value::Object(obj) = item {
                    if let Some(Value::String(id)) = obj.get("_id") {
                        if id == target_id {
                            if let Value::Object(source) = &update_data {
                                for (key, value) in source {
                                    obj.insert(key.clone(), value.clone());
                                }
                                obj.insert(
                                    "_updated_at".to_string(),
                                    Value::String(Utc::now().to_rfc3339()),
                                );
                                updated_count += 1;
                            }
                            break;
                        }
                    }
                }
            }
        } else {
            for item in &mut data {
                if search_in_json_value(item, query) {
                    if let Value::Object(source) = &update_data {
                        if let Value::Object(target) = item {
                            for (key, value) in source {
                                target.insert(key.clone(), value.clone());
                            }
                            target.insert(
                                "_updated_at".to_string(),
                                Value::String(Utc::now().to_rfc3339()),
                            );
                            updated_count += 1;
                        }
                    }
                }
            }
        }

        Self::write_database(username, db_name, &data)?;
        Ok(format!("Updated {} records", updated_count))
    }

    // DELETE - Delete a record by _id
    pub fn delete_record_by_id(
        &self,
        username: &str,
        db_name: &str,
        target_id: &str,
    ) -> Result<String, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let mut data = Self::read_database_cached(username, db_name)?;
        let initial_count = data.len();

        data.retain(|item| {
            if let Value::Object(obj) = item {
                if let Some(Value::String(id)) = obj.get("_id") {
                    return id != target_id;
                }
            }
            true
        });

        let deleted_count = initial_count - data.len();
        Self::write_database(username, db_name, &data)?;
        Ok(format!("Deleted {} records", deleted_count))
    }

    // DELETE - Delete records matching query
    pub fn delete_records(
        &self,
        username: &str,
        db_name: &str,
        query: &str,
    ) -> Result<String, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let mut data = Self::read_database_cached(username, db_name)?;
        let initial_count = data.len();

        data.retain(|item| !search_in_json_value(item, query));

        let deleted_count = initial_count - data.len();
        Self::write_database(username, db_name, &data)?;
        Ok(format!("Deleted {} records", deleted_count))
    }

    // BROWSE - Paginate all records without filtering
    pub fn browse_records(
        &self,
        username: &str,
        db_name: &str,
        page: Option<usize>,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let data = Self::read_database_cached(username, db_name)?;
        let total_records = data.len();

        match (page, limit) {
            (None, Some(lim)) => {
                let paginated_data: Vec<Value> = data.into_iter().take(lim).collect();
                let returned = paginated_data.len();
                Ok(serde_json::json!({
                    "data": paginated_data,
                    "pagination": {
                        "limit": lim,
                        "returned": returned,
                        "total_records": total_records,
                        "mode": "limit_only"
                    }
                }))
            }
            (Some(p), Some(lim)) => {
                let page_num = p.max(1);
                let offset = (page_num - 1) * lim;
                let paginated_data: Vec<Value> =
                    data.into_iter().skip(offset).take(lim).collect();
                let total_pages = if lim > 0 {
                    (total_records as f64 / lim as f64).ceil() as usize
                } else {
                    0
                };
                let returned = paginated_data.len();
                Ok(serde_json::json!({
                    "data": paginated_data,
                    "pagination": {
                        "page": page_num,
                        "limit": lim,
                        "returned": returned,
                        "total_records": total_records,
                        "total_pages": total_pages,
                        "has_next": page_num < total_pages,
                        "has_prev": page_num > 1,
                        "mode": "paginated"
                    }
                }))
            }
            (Some(_), None) => Err(
                "Cannot use 'page' without 'limit'. Please provide both parameters.".to_string(),
            ),
            (None, None) => {
                let default_limit = 10;
                let paginated_data: Vec<Value> =
                    data.into_iter().take(default_limit).collect();
                let returned = paginated_data.len();
                let total_pages =
                    (total_records as f64 / default_limit as f64).ceil() as usize;
                Ok(serde_json::json!({
                    "data": paginated_data,
                    "pagination": {
                        "page": 1,
                        "limit": default_limit,
                        "returned": returned,
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

    // LIST - Advanced query: filter + sort + paginate
    pub fn list_records(
        &self,
        username: &str,
        db_name: &str,
        page: Option<usize>,
        limit: Option<usize>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
        filters: Option<&Value>,
    ) -> Result<Value, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        let mut data = Self::read_database_cached(username, db_name)?;
        let total_records = data.len();

        if let Some(Value::Object(filters_map)) = filters {
            data.retain(|item| self.matches_filters(item, filters_map));
        }

        let filtered_count = data.len();

        if let Some(field) = sort_by {
            let order = sort_order.unwrap_or("asc");
            data.sort_by(|a, b| self.compare_values(a, b, field, order));
        }

        let page_num = page.unwrap_or(1);
        let page_size = limit.unwrap_or(10);
        let offset = page_num.saturating_sub(1) * page_size;

        let paginated_data: Vec<Value> =
            data.into_iter().skip(offset).take(page_size).collect();
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

    fn matches_filters(
        &self,
        item: &Value,
        filters: &serde_json::Map<String, Value>,
    ) -> bool {
        if let Value::Object(obj) = item {
            for (key, filter_value) in filters {
                match obj.get(key) {
                    Some(item_value) => {
                        if !self.value_matches_filter(item_value, filter_value) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
        } else {
            false
        }
    }

    fn value_matches_filter(&self, item_value: &Value, filter_value: &Value) -> bool {
        match filter_value {
            // Array filter = OR logic: item value must match any element
            Value::Array(arr) => arr.iter().any(|fv| item_value == fv),
            _ => item_value == filter_value,
        }
    }

    fn compare_values(
        &self,
        a: &Value,
        b: &Value,
        field: &str,
        order: &str,
    ) -> std::cmp::Ordering {
        let a_val = self.get_field_value(a, field);
        let b_val = self.get_field_value(b, field);

        let cmp = match (a_val, b_val) {
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

        if order == "desc" { cmp.reverse() } else { cmp }
    }

    fn get_field_value<'a>(&self, item: &'a Value, field: &str) -> Option<&'a Value> {
        if let Value::Object(obj) = item {
            obj.get(field)
        } else {
            None
        }
    }

    pub fn get_stats(&self, username: &str, db_name: &str) -> Result<Value, String> {
        if !Self::database_exists(username, db_name) {
            return Err("Database does not exist".to_string());
        }

        // Check cache state before reading (read may populate cache)
        let was_cached = Self::is_cached(username, db_name);

        let read_start = Instant::now();
        let data = Self::read_database_cached(username, db_name)?;
        let read_time_ms = read_start.elapsed().as_millis();

        let filepath = Self::get_db_path(username, db_name);
        Ok(serde_json::json!({
            "database": db_name,
            "username": username,
            "total_records": data.len(),
            "size_bytes": fs::metadata(&filepath).map(|m| m.len()).unwrap_or(0),
            "read_time_ms": read_time_ms,
            "cached": was_cached,
            "timestamp": Utc::now().to_rfc3339()
        }))
    }
}
