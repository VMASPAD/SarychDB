use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::Value;
use crate::modules::auth::AuthService;
use crate::modules::database::DatabaseManager;

#[derive(Debug)]
pub struct SarychProtocol {
    pub username: String,
    pub password: String,
    pub database: String,
    pub operation: Option<String>,
    pub query: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ProtocolMessage {
    url: String,
    op: Option<String>,
    body: Option<Value>,
    #[serde(rename = "queryType")]
    query_type: Option<String>,
    #[serde(rename = "idUpdate")]
    id_update: Option<String>,
    page: Option<usize>,
    limit: Option<usize>,
    #[serde(rename = "sortBy")]
    sort_by: Option<String>,
    #[serde(rename = "sortOrder")]
    sort_order: Option<String>,
    filters: Option<Value>,
}

pub struct SarychServer {
}

impl SarychServer {

    // Parsear el protocolo sarychdb://usuario@password/database/?query=valor
    pub fn parse_sarych_url(url_str: &str) -> Result<SarychProtocol, String> {
        println!("🔍 Parsing URL: {}", url_str);
        
        // Verify it starts with sarychdb://
        if !url_str.starts_with("sarychdb://") {
            return Err("URL must start with sarychdb://".to_string());
        }

        // Remove protocol
        let without_protocol = url_str.strip_prefix("sarychdb://").unwrap();
        println!("📝 Without protocol: {}", without_protocol);

        // Separar query parameters si existen
        let (main_part, query_string) = if let Some(pos) = without_protocol.find('?') {
            let (main, query) = without_protocol.split_at(pos);
            (main, Some(&query[1..])) // [1..] para saltar el '?'
        } else {
            (without_protocol, None)
        };

        println!("🔧 Main part: {}", main_part);
        if let Some(q) = query_string {
            println!("🔍 Query string: {}", q);
        }

        // Parse username@password/database[/operation]
        let parts: Vec<&str> = main_part.split('/').collect();

        if parts.len() < 2 {
            return Err("Invalid format. Use: sarychdb://username@password/database/".to_string());
        }

        // Extraer usuario@password
        let auth_part = parts[0];
        let database = parts[1].to_string();
        let operation = parts.get(2).map(|op| op.to_string()).filter(|op| !op.is_empty());

        println!("🔐 Auth part: {}", auth_part);
        println!("🗄️  Database: {}", database);
        if let Some(ref op) = operation {
            println!("⚡ Operation: {}", op);
        }

        // Separate username and password
        if !auth_part.contains('@') {
            return Err("Invalid authentication format. Use: username@password".to_string());
        }

        let auth_parts: Vec<&str> = auth_part.split('@').collect();
        if auth_parts.len() != 2 {
            return Err("Invalid authentication format. Use: username@password".to_string());
        }

        let username = auth_parts[0].to_string();
        let password = auth_parts[1].to_string();

        if username.is_empty() || password.is_empty() {
            return Err("Username and password cannot be empty".to_string());
        }

        println!("👤 Username: {}", username);
        println!("🔑 Password: [HIDDEN]");

        // Parsear query parameters
        let query = if let Some(query_str) = query_string {
            // Buscar el parámetro "query"
            let mut found_query = None;
            for param in query_str.split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    if key == "query" {
                        found_query = Some(urlencoding::decode(value).map_err(|_| "Error decoding query")?.into_owned());
                        break;
                    }
                }
            }
            found_query
        } else {
            None
        };

        if let Some(ref q) = query {
            println!("🔎 Query: {}", q);
        }

        Ok(SarychProtocol {
            username,
            password,
            database,
            operation,
            query,
        })
    }

    // Handle raw SarychDB protocol messages over TCP
    pub async fn handle_protocol_message(raw: String) -> Result<Value, String> {
        let start_time = std::time::Instant::now();
        let auth_service = AuthService::new();
        let db_manager = DatabaseManager::new();

        let trimmed = raw.trim();
        if trimmed.is_empty() {
            let elapsed_ms = start_time.elapsed().as_millis() as u64;
            return Ok(serde_json::json!({ "error": "Empty request", "time": elapsed_ms }));
        }

        let (url, op_override, body, query_type, id_update, page, limit, sort_by, sort_order, filters) =
            if trimmed.starts_with("sarychdb://") {
                (trimmed.to_string(), None, None, None, None, None, None, None, None, None)
            } else {
                let msg: ProtocolMessage = match serde_json::from_str(trimmed) {
                    Ok(msg) => msg,
                    Err(_) => {
                        let elapsed_ms = start_time.elapsed().as_millis() as u64;
                        return Ok(serde_json::json!({
                            "error": "Invalid JSON message. Expected { url: \"sarychdb://...\" }",
                            "time": elapsed_ms
                        }));
                    }
                };
                (msg.url, msg.op, msg.body, msg.query_type, msg.id_update, msg.page, msg.limit, msg.sort_by, msg.sort_order, msg.filters)
            };

        let protocol = match Self::parse_sarych_url(&url) {
            Ok(p) => p,
            Err(e) => {
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                return Ok(serde_json::json!({ "error": e, "time": elapsed_ms }));
            }
        };

        let operation = op_override
            .or(protocol.operation.clone())
            .unwrap_or_else(|| "get".to_string())
            .to_lowercase();

        let result = match operation.as_str() {
            "create_user" | "signup" => {
                let user_result = auth_service.create_user(crate::modules::auth::CreateUserRequest {
                    username: protocol.username.clone(),
                    password: protocol.password.clone(),
                });

                let mut responses = Vec::new();

                match user_result {
                    Ok(message) => responses.push(serde_json::json!({
                        "action": "create_user",
                        "message": message
                    })),
                    Err(e) if e == "User already exists" => responses.push(serde_json::json!({
                        "action": "create_user",
                        "message": e
                    })),
                    Err(e) => return Err(e),
                }

                if !protocol.database.is_empty() {
                    let db_result = auth_service.create_database(crate::modules::auth::CreateDbRequest {
                        username: protocol.username.clone(),
                        password: protocol.password.clone(),
                        db_name: protocol.database.clone(),
                    });

                    match db_result {
                        Ok(message) => responses.push(serde_json::json!({
                            "action": "create_db",
                            "database": protocol.database,
                            "message": message
                        })),
                        Err(e) => responses.push(serde_json::json!({
                            "action": "create_db",
                            "database": protocol.database,
                            "error": e
                        })),
                    }
                }

                Ok(serde_json::json!({
                    "operation": "create_user",
                    "results": responses
                }))
            }
            "create_db" | "create_database" | "create_db_only" => {
                let message = auth_service.create_database(crate::modules::auth::CreateDbRequest {
                    username: protocol.username.clone(),
                    password: protocol.password.clone(),
                    db_name: protocol.database.clone(),
                })?;
                Ok(serde_json::json!({
                    "operation": "create_db",
                    "database": protocol.database,
                    "message": message
                }))
            }
            "delete_db" | "delete_database" => {
                let message = auth_service.delete_database(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database
                )?;
                Ok(serde_json::json!({
                    "operation": "delete_db",
                    "database": protocol.database,
                    "message": message
                }))
            }
            "list_dbs" | "list_databases" => {
                let databases = auth_service.get_user_databases(&protocol.username, &protocol.password)?;
                Ok(serde_json::json!({
                    "operation": "list_dbs",
                    "user": protocol.username,
                    "databases": databases
                }))
            }
            "all_dbs" | "get_all_dbs" => {
                let databases = auth_service.get_user_databases(&protocol.username, &protocol.password)?;
                let mut db_details: Vec<Value> = Vec::new();
                for db in &databases {
                    let count = db_manager.count_records(&protocol.username, &db.namedb).unwrap_or(0);
                    db_details.push(serde_json::json!({
                        "name": db.namedb,
                        "count": count
                    }));
                }
                Ok(serde_json::json!({
                    "operation": "all_dbs",
                    "user": protocol.username,
                    "databases": db_details,
                    "total_databases": databases.len()
                }))
            }
            "get" => Self::handle_get(&db_manager, &protocol, query_type.as_deref()).await,
            "browse" => {
                let result = db_manager.browse_records(&protocol.username, &protocol.database, page, limit)?;
                Ok(serde_json::json!({
                    "operation": "browse",
                    "database": protocol.database,
                    "data": result.get("data"),
                    "pagination": result.get("pagination")
                }))
            }
            "list" => {
                // Authenticate using credentials from the URL
                if !auth_service.authenticate(&protocol.username, &protocol.password)? {
                    return Err("Invalid credentials".to_string());
                }

                // Verify user has access to database
                if let Err(e) = auth_service.user_has_database(&protocol.username, &protocol.password, &protocol.database) {
                    return Err(format!("Database access denied: {}", e));
                }
                let result = db_manager.list_records(
                    &protocol.username,
                    &protocol.database,
                    page,
                    limit,
                    sort_by.as_deref(),
                    sort_order.as_deref(),
                    filters.as_ref()
                )?;

                Ok(serde_json::json!({
                    "operation": "list",
                    "database": protocol.database,
                    "data": result.get("data"),
                    "pagination": result.get("pagination"),
                    "sorting": result.get("sorting")
                }))
            }
            "post" => {
                if !auth_service.authenticate(&protocol.username, &protocol.password)? {
                    return Err("Invalid credentials".to_string());
                }
                if let Err(e) = auth_service.user_has_database(&protocol.username, &protocol.password, &protocol.database) {
                    return Err(format!("Database access denied: {}", e));
                }
                let record = body.ok_or("Body required for POST operation")?;
                let message = db_manager.insert_record(&protocol.username, &protocol.database, record)?;
                Ok(serde_json::json!({
                    "operation": "post",
                    "database": protocol.database,
                    "message": message
                }))
            }
            "put" => {
                if !auth_service.authenticate(&protocol.username, &protocol.password)? {
                    return Err("Invalid credentials".to_string());
                }
                if let Err(e) = auth_service.user_has_database(&protocol.username, &protocol.password, &protocol.database) {
                    return Err(format!("Database access denied: {}", e));
                }
                let update_data = body.ok_or("Body required for PUT operation")?;
                let message = if let Some(id) = id_update.as_deref() {
                    db_manager.update_records(&protocol.username, &protocol.database, "", update_data, Some(id))?
                } else {
                    let query = protocol.query.as_deref().ok_or("Query or idUpdate required for PUT operation")?;
                    db_manager.update_records(&protocol.username, &protocol.database, query, update_data, None)?
                };
                Ok(serde_json::json!({
                    "operation": "put",
                    "database": protocol.database,
                    "query": protocol.query,
                    "id_update": id_update,
                    "message": message
                }))
            }
            "edit" | "edit_by_id" => {
                if !auth_service.authenticate(&protocol.username, &protocol.password)? {
                    return Err("Invalid credentials".to_string());
                }
                if let Err(e) = auth_service.user_has_database(&protocol.username, &protocol.password, &protocol.database) {
                    return Err(format!("Database access denied: {}", e));
                }
                let mut update_data = body.ok_or("Body required for EDIT operation")?;

                let body_id = if let Value::Object(ref mut obj) = update_data {
                    match obj.remove("_id") {
                        Some(Value::String(id)) if !id.is_empty() => Some(id),
                        _ => None,
                    }
                } else {
                    None
                };

                let id_value = id_update
                    .as_deref()
                    .map(|id| id.to_string())
                    .or(body_id)
                    .ok_or("idUpdate or _id required for EDIT operation")?;

                let message = db_manager.update_records(
                    &protocol.username,
                    &protocol.database,
                    "",
                    update_data,
                    Some(id_value.as_str())
                )?;

                Ok(serde_json::json!({
                    "operation": "edit",
                    "database": protocol.database,
                    "id_update": id_value,
                    "message": message
                }))
            }
            "delete" => {
                if !auth_service.authenticate(&protocol.username, &protocol.password)? {
                    return Err("Invalid credentials".to_string());
                }
                if let Err(e) = auth_service.user_has_database(&protocol.username, &protocol.password, &protocol.database) {
                    return Err(format!("Database access denied: {}", e));
                }
                let query = protocol.query.as_deref().ok_or("Query required for DELETE operation")?;
                let message = db_manager.delete_records(&protocol.username, &protocol.database, query)?;
                Ok(serde_json::json!({
                    "operation": "delete",
                    "database": protocol.database,
                    "query": query,
                    "message": message
                }))
            }
            "delete_by_id" | "delete_id" => {
                if !auth_service.authenticate(&protocol.username, &protocol.password)? {
                    return Err("Invalid credentials".to_string());
                }
                if let Err(e) = auth_service.user_has_database(&protocol.username, &protocol.password, &protocol.database) {
                    return Err(format!("Database access denied: {}", e));
                }

                let body_id = body.and_then(|value| {
                    if let Value::Object(obj) = value {
                        match obj.get("_id") {
                            Some(Value::String(id)) if !id.is_empty() => Some(id.clone()),
                            _ => None,
                        }
                    } else {
                        None
                    }
                });

                let id_value = id_update
                    .as_deref()
                    .map(|id| id.to_string())
                    .or(body_id)
                    .ok_or("idUpdate or _id required for DELETE_BY_ID operation")?;

                let message = db_manager.delete_record_by_id(
                    &protocol.username,
                    &protocol.database,
                    id_value.as_str()
                )?;

                Ok(serde_json::json!({
                    "operation": "delete_by_id",
                    "database": protocol.database,
                    "id_update": id_value,
                    "message": message
                }))
            }
            "stats" => {
                if !auth_service.authenticate(&protocol.username, &protocol.password)? {
                    return Err("Invalid credentials".to_string());
                }
                if let Err(e) = auth_service.user_has_database(&protocol.username, &protocol.password, &protocol.database) {
                    return Err(format!("Database access denied: {}", e));
                }
                db_manager.get_stats(&protocol.username, &protocol.database)
            }
            "health" => Self::health().await,
            _ => Err("Unsupported operation. Use: get, browse, list, post, put, edit, delete, delete_by_id, delete_db, stats".to_string()),
        };

        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        match result {
            Ok(mut response) => {
                if let Some(obj) = response.as_object_mut() {
                    obj.insert("time".to_string(), serde_json::Value::Number(elapsed_ms.into()));
                }
                Ok(response)
            }
            Err(e) => Ok(serde_json::json!({ "error": e, "time": elapsed_ms })),
        }
    }

    async fn handle_get(db_manager: &DatabaseManager, protocol: &SarychProtocol, query_type: Option<&str>) -> Result<Value, String> {
        let results = db_manager.search_records(&protocol.username, &protocol.database, protocol.query.as_deref(), query_type)?;
        Ok(serde_json::json!({
            "operation": "get",
            "database": protocol.database,
            "query": protocol.query,
            "query_type": query_type,
            "results": results,
            "count": results.len()
        }))
    }

    async fn health() -> Result<Value, String> {
        Ok(serde_json::json!({
            "operation": "health",
            "status": "ok",
            "message": "SarychDB is healthy"
        }))
    }

    // Start raw SarychDB protocol server over TCP
    pub async fn start_protocol_server(port: u16) {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await.expect("Failed to bind SarychDB protocol server");
        println!("🛰️  SarychDB protocol server started on port {}", port);

        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => continue,
            };

            tokio::spawn(async move {
                let mut buffer = vec![0u8; 65536];
                let read = match socket.read(&mut buffer).await {
                    Ok(n) => n,
                    Err(_) => 0,
                };

                if read == 0 {
                    return;
                }

                let request = String::from_utf8_lossy(&buffer[..read]).trim().to_string();
                let response = match SarychServer::handle_protocol_message(request).await {
                    Ok(json) => serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_string()),
                    Err(e) => serde_json::json!({ "error": e }).to_string(),
                };

                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.write_all(b"\n").await;
            });
        }
    }

}