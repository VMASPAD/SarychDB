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
    query: Option<Value>,
    #[serde(default, alias = "updateData")]
    update_data: Option<Value>,
    #[serde(rename = "queryType")]
    query_type: Option<String>,
    #[serde(default, alias = "idUpdate")]
    id_update: Option<String>,
    page: Option<usize>,
    limit: Option<usize>,
    #[serde(rename = "sortBy")]
    sort_by: Option<String>,
    #[serde(rename = "sortOrder")]
    sort_order: Option<String>,
    filters: Option<Value>,
}

pub struct SarychServer {}

impl SarychServer {
    pub fn parse_sarych_url(url_str: &str) -> Result<SarychProtocol, String> {
        if !url_str.starts_with("sarychdb://") {
            return Err("URL must start with sarychdb://".to_string());
        }

        let without_protocol = url_str.strip_prefix("sarychdb://").unwrap();

        let (main_part, query_string) = if let Some(pos) = without_protocol.find('?') {
            let (main, query) = without_protocol.split_at(pos);
            (main, Some(&query[1..]))
        } else {
            (without_protocol, None)
        };

        let parts: Vec<&str> = main_part.split('/').collect();

        if parts.len() < 2 {
            return Err(
                "Invalid format. Use: sarychdb://username@password/database/".to_string(),
            );
        }

        let auth_part = parts[0];
        let database = parts[1].to_string();
        let operation = parts
            .get(2)
            .map(|op| op.to_string())
            .filter(|op| !op.is_empty());

        if !auth_part.contains('@') {
            return Err(
                "Invalid authentication format. Use: username@password".to_string(),
            );
        }

        let auth_parts: Vec<&str> = auth_part.splitn(2, '@').collect();
        if auth_parts.len() != 2 {
            return Err(
                "Invalid authentication format. Use: username@password".to_string(),
            );
        }

        let username = auth_parts[0].to_string();
        let password = auth_parts[1].to_string();

        if username.is_empty() || password.is_empty() {
            return Err("Username and password cannot be empty".to_string());
        }

        let query = if let Some(query_str) = query_string {
            let mut found_query = None;
            for param in query_str.split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    if key == "query" {
                        found_query = Some(
                            urlencoding::decode(value)
                                .map_err(|_| "Error decoding query")?
                                .into_owned(),
                        );
                        break;
                    }
                }
            }
            found_query
        } else {
            None
        };

        Ok(SarychProtocol {
            username,
            password,
            database,
            operation,
            query,
        })
    }

    pub async fn handle_protocol_message(raw: String) -> Result<Value, String> {
        let start_time = std::time::Instant::now();
        let auth_service = AuthService::new();
        let db_manager = DatabaseManager::new();

        let trimmed = raw.trim();
        if trimmed.is_empty() {
            let elapsed_ms = start_time.elapsed().as_millis() as u64;
            return Ok(serde_json::json!({ "error": "Empty request", "time": elapsed_ms }));
        }

        let (
            url,
            op_override,
            body,
            query,
            update_data,
            query_type,
            id_update,
            page,
            limit,
            sort_by,
            sort_order,
            filters,
        ) = if trimmed.starts_with("sarychdb://") {
            (
                trimmed.to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        } else {
            let msg: ProtocolMessage = match serde_json::from_str(trimmed) {
                Ok(msg) => msg,
                Err(e) => {
                    let elapsed_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(serde_json::json!({
                        "error": format!("Invalid JSON message: {}", e),
                        "time": elapsed_ms
                    }));
                }
            };
            (
                msg.url,
                msg.op,
                msg.body,
                msg.query,
                msg.update_data,
                msg.query_type,
                msg.id_update,
                msg.page,
                msg.limit,
                msg.sort_by,
                msg.sort_order,
                msg.filters,
            )
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
                    let db_result =
                        auth_service.create_database(crate::modules::auth::CreateDbRequest {
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
                let message =
                    auth_service.create_database(crate::modules::auth::CreateDbRequest {
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
                    &protocol.database,
                )?;
                Ok(serde_json::json!({
                    "operation": "delete_db",
                    "database": protocol.database,
                    "message": message
                }))
            }

            "rename_db" | "rename_database" | "update_db_name" => {
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;

                let body_obj = body.as_ref().and_then(|b| b.as_object());
                let mut new_name: Option<String> = body_obj
                    .and_then(|obj| {
                        obj.get("new_name")
                            .or_else(|| obj.get("newName"))
                    })
                    .and_then(|v| v.as_str())
                    .map(String::from);

                if new_name.is_none() {
                    new_name = update_data.as_ref().and_then(|ud| match ud {
                        Value::String(s) => Some(s.clone()),
                        Value::Object(obj) => obj
                            .get("new_name")
                            .or_else(|| obj.get("newName"))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        _ => None,
                    });
                }

                if new_name.is_none() {
                    new_name = query.as_ref().and_then(|qv| {
                        if let Value::String(s) = qv { Some(s.clone()) } else { None }
                    });
                }

                if new_name.is_none() {
                    new_name = id_update.clone();
                }

                let new_name =
                    new_name.ok_or("new_name required for RENAME_DB operation")?;

                let message = auth_service.rename_database(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                    &new_name,
                )?;

                Ok(serde_json::json!({
                    "operation": "rename_db",
                    "old_database": protocol.database,
                    "new_database": new_name,
                    "message": message
                }))
            }

            "list_dbs" | "list_databases" => {
                let databases =
                    auth_service.get_user_databases(&protocol.username, &protocol.password)?;
                Ok(serde_json::json!({
                    "operation": "list_dbs",
                    "user": protocol.username,
                    "databases": databases
                }))
            }

            "all_dbs" | "get_all_dbs" => {
                let databases =
                    auth_service.get_user_databases(&protocol.username, &protocol.password)?;
                let mut db_details: Vec<Value> = Vec::new();
                for db in &databases {
                    let count = db_manager
                        .count_records(&protocol.username, &db.namedb)
                        .unwrap_or(0);
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

            "get" => {
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;
                Self::handle_get(&db_manager, &protocol, query_type.as_deref()).await
            }

            "browse" => {
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;
                let result = db_manager.browse_records(
                    &protocol.username,
                    &protocol.database,
                    page,
                    limit,
                )?;
                Ok(serde_json::json!({
                    "operation": "browse",
                    "database": protocol.database,
                    "data": result.get("data"),
                    "pagination": result.get("pagination")
                }))
            }

            "list" => {
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;
                let result = db_manager.list_records(
                    &protocol.username,
                    &protocol.database,
                    page,
                    limit,
                    sort_by.as_deref(),
                    sort_order.as_deref(),
                    filters.as_ref(),
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
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;
                let record = body.ok_or("Body required for POST operation")?;
                let message =
                    db_manager.insert_record(&protocol.username, &protocol.database, record)?;
                Ok(serde_json::json!({
                    "operation": "post",
                    "database": protocol.database,
                    "message": message
                }))
            }

            "put" => {
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;
                let update_data = body.ok_or("Body required for PUT operation")?;
                let message = if let Some(id) = id_update.as_deref() {
                    db_manager.update_records(
                        &protocol.username,
                        &protocol.database,
                        "",
                        update_data,
                        Some(id),
                    )?
                } else {
                    let q = protocol
                        .query
                        .as_deref()
                        .ok_or("Query or idUpdate required for PUT operation")?;
                    db_manager.update_records(
                        &protocol.username,
                        &protocol.database,
                        q,
                        update_data,
                        None,
                    )?
                };
                Ok(serde_json::json!({
                    "operation": "put",
                    "database": protocol.database,
                    "query": protocol.query,
                    "id_update": id_update,
                    "message": message
                }))
            }

            "update_records" => {
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;

                let body_obj = body.as_ref().and_then(|b| b.as_object());

                let final_id_update = id_update
                    .clone()
                    .or_else(|| {
                        body_obj.and_then(|obj| {
                            obj.get("id_update")
                                .or_else(|| obj.get("idUpdate"))
                                .and_then(|v| v.as_str())
                                .map(String::from)
                        })
                    });

                let final_update_data = update_data
                    .clone()
                    .or_else(|| {
                        body_obj.and_then(|obj| {
                            obj.get("update_data")
                                .or_else(|| obj.get("updateData"))
                                .cloned()
                        })
                    })
                    .or_else(|| {
                        body_obj.map(|obj| {
                            let mut update_obj = obj.clone();
                            update_obj.remove("id_update");
                            update_obj.remove("idUpdate");
                            update_obj.remove("query");
                            update_obj.remove("update_data");
                            update_obj.remove("updateData");
                            Value::Object(update_obj)
                        })
                    });

                let update_data_value = final_update_data
                    .ok_or("update_data required for UPDATE_RECORDS operation")?;

                let message = if let Some(id) = final_id_update.as_deref() {
                    db_manager.update_records(
                        &protocol.username,
                        &protocol.database,
                        "",
                        update_data_value,
                        Some(id),
                    )?
                } else {
                    return Err("id_update required for UPDATE_RECORDS operation".to_string());
                };

                Ok(serde_json::json!({
                    "operation": "update_records",
                    "database": protocol.database,
                    "id_update": final_id_update,
                    "message": message
                }))
            }

            "edit" | "edit_by_id" => {
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;
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
                    Some(id_value.as_str()),
                )?;

                Ok(serde_json::json!({
                    "operation": "edit",
                    "database": protocol.database,
                    "id_update": id_value,
                    "message": message
                }))
            }

            "delete" => {
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;
                let q = protocol
                    .query
                    .as_deref()
                    .ok_or("Query required for DELETE operation")?;
                let message =
                    db_manager.delete_records(&protocol.username, &protocol.database, q)?;
                Ok(serde_json::json!({
                    "operation": "delete",
                    "database": protocol.database,
                    "query": q,
                    "message": message
                }))
            }

            "delete_by_id" | "delete_id" => {
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;

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
                    id_value.as_str(),
                )?;

                Ok(serde_json::json!({
                    "operation": "delete_by_id",
                    "database": protocol.database,
                    "id_update": id_value,
                    "message": message
                }))
            }

            "stats" => {
                auth_service.authenticate_and_check_db(
                    &protocol.username,
                    &protocol.password,
                    &protocol.database,
                )?;
                db_manager.get_stats(&protocol.username, &protocol.database)
            }

            "health" => Self::health().await,

            _ => Err(
                "Unsupported operation. Use: get, browse, list, post, put, edit, \
                 delete, delete_by_id, delete_db, rename_db, stats"
                    .to_string(),
            ),
        };

        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        match result {
            Ok(mut response) => {
                if let Some(obj) = response.as_object_mut() {
                    obj.insert(
                        "time".to_string(),
                        serde_json::Value::Number(elapsed_ms.into()),
                    );
                }
                Ok(response)
            }
            Err(e) => Ok(serde_json::json!({ "error": e, "time": elapsed_ms })),
        }
    }

    async fn handle_get(
        db_manager: &DatabaseManager,
        protocol: &SarychProtocol,
        query_type: Option<&str>,
    ) -> Result<Value, String> {
        let results = db_manager.search_records(
            &protocol.username,
            &protocol.database,
            protocol.query.as_deref(),
            query_type,
        )?;
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

    pub async fn start_protocol_server(port: u16) {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .expect("Failed to bind SarychDB protocol server");
        println!("🛰️  SarychDB protocol server started on port {}", port);

        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => continue,
            };

            tokio::spawn(async move {
                // 4 MB buffer — handles large JSON payloads
                let mut buffer = vec![0u8; 4 * 1024 * 1024];
                let read = match socket.read(&mut buffer).await {
                    Ok(n) => n,
                    Err(_) => 0,
                };

                if read == 0 {
                    return;
                }

                let request =
                    String::from_utf8_lossy(&buffer[..read]).trim().to_string();
                let response = match SarychServer::handle_protocol_message(request).await {
                    Ok(json) => {
                        serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_string())
                    }
                    Err(e) => serde_json::json!({ "error": e }).to_string(),
                };

                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.write_all(b"\n").await;
            });
        }
    }
}
