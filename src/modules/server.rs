use warp::{Filter, Reply, Rejection};
use serde_json::Value;
use std::collections::HashMap; 
use crate::modules::auth::{AuthService, CreateUserRequest, CreateDbRequest};
use crate::modules::database::DatabaseManager;

#[derive(Debug)]
pub struct SarychProtocol {
    pub username: String,
    pub password: String,
    pub database: String,
    pub operation: String,
    pub query: Option<String>,
}

pub struct SarychServer {
}

impl SarychServer {

    // Parsear el protocolo sarychdb://usuario@password/database/operacion?query=valor
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

        // Parse username@password/database/operation
        let parts: Vec<&str> = main_part.split('/').collect();
        
        if parts.len() < 3 {
            return Err("Invalid format. Use: sarychdb://username@password/database/operation".to_string());
        }

        // Extraer usuario@password
        let auth_part = parts[0];
        let database = parts[1].to_string();
        let operation = parts[2].to_string();

        println!("🔐 Auth part: {}", auth_part);
        println!("🗄️  Database: {}", database);
        println!("⚡ Operation: {}", operation);

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

    // Handle SarychDB protocol operations with header authentication
    pub async fn handle_sarych_request(
        url_str: String, 
        body: Option<Value>, 
        username: String, 
        password: String,
        query_type: Option<String>,
        id_update: Option<String>,
        page: Option<String>,
        limit: Option<String>,
        sort_by: Option<String>,
        sort_order: Option<String>,
        filters: Option<String>
    ) -> Result<impl Reply, Rejection> {
        let operation_start = std::time::Instant::now();
        let auth_service = AuthService::new();
        let db_manager = DatabaseManager::new();
        
        // Parse URL but ignore username/password from URL since we use headers
        let protocol = match Self::parse_sarych_url(&url_str) {
            Ok(p) => p,
            Err(e) => return Ok(warp::reply::with_status(e, warp::http::StatusCode::BAD_REQUEST)),
        };

        // Verify authentication using headers
        if let Err(e) = auth_service.authenticate(&username, &password) {
            return Ok(warp::reply::with_status(
                format!("Authentication error: {}", e),
                warp::http::StatusCode::UNAUTHORIZED,
            ));
        }

        // Verify user has access to database
        if let Err(e) = auth_service.user_has_database(&username, &password, &protocol.database) {
            return Ok(warp::reply::with_status(
                format!("Database access denied: {}", e),
                warp::http::StatusCode::FORBIDDEN,
            ));
        }

        // Process operation with new parameters
        let result = match protocol.operation.to_lowercase().as_str() {
            "get" => Self::handle_get(&db_manager, &protocol, query_type.as_deref()).await,
            "browse" => Self::handle_browse(&db_manager, &protocol, page.as_deref(), limit.as_deref()).await,
            "list" => Self::handle_list(&db_manager, &protocol, page.as_deref(), limit.as_deref(), sort_by.as_deref(), sort_order.as_deref(), filters.as_deref()).await,
            "post" => Self::handle_post(&db_manager, &protocol, body, &username).await,
            "put" => Self::handle_put(&db_manager, &protocol, body, &username, id_update.as_deref()).await,
            "delete" => Self::handle_delete(&db_manager, &protocol, &username).await,
            "stats" => Self::handle_stats(&db_manager, &protocol, &username).await,
            "health" => Self::health().await,
            _ => Err("Unsupported operation. Use: get, browse, list, post, put, delete, stats".to_string()),
        };

        let operation_time = operation_start.elapsed().as_millis();

        match result {
            Ok(mut response) => {
                // Add operation time to all responses
                if let Some(obj) = response.as_object_mut() {
                    obj.insert("time".to_string(), serde_json::Value::Number((operation_time as u64).into()));
                }
                Ok(warp::reply::with_status(
                    serde_json::to_string(&response).unwrap_or_default(),
                    warp::http::StatusCode::OK,
                ))
            },
            Err(e) => {
                let error_response = serde_json::json!({
                    "error": e,
                    "time": operation_time
                });
                Ok(warp::reply::with_status(
                    serde_json::to_string(&error_response).unwrap_or_default(),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            },
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

    async fn handle_browse(
        db_manager: &DatabaseManager,
        protocol: &SarychProtocol,
        page: Option<&str>,
        limit: Option<&str>
    ) -> Result<Value, String> {
        // Parse pagination parameters
        let limit_num = limit.and_then(|l| l.parse::<usize>().ok());
        let page_num = page.and_then(|p| p.parse::<usize>().ok());

        let result = db_manager.browse_records(
            &protocol.username,
            &protocol.database,
            page_num,
            limit_num
        )?;

        Ok(serde_json::json!({
            "operation": "browse",
            "database": protocol.database,
            "data": result.get("data"),
            "pagination": result.get("pagination")
        }))
    }

    async fn handle_list(
        db_manager: &DatabaseManager,
        protocol: &SarychProtocol,
        page: Option<&str>,
        limit: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
        filters: Option<&str>
    ) -> Result<Value, String> {
        // Parse pagination parameters
        let page_num = page.and_then(|p| p.parse::<usize>().ok());
        let limit_num = limit.and_then(|l| l.parse::<usize>().ok());
        
        // Parse filters JSON
        let filters_obj = filters.and_then(|f| {
            serde_json::from_str::<Value>(f).ok()
        });

        let result = db_manager.list_records(
            &protocol.username,
            &protocol.database,
            page_num,
            limit_num,
            sort_by,
            sort_order,
            filters_obj.as_ref()
        )?;

        Ok(serde_json::json!({
            "operation": "list",
            "database": protocol.database,
            "data": result.get("data"),
            "pagination": result.get("pagination"),
            "sorting": result.get("sorting")
        }))
    }

    async fn handle_post(db_manager: &DatabaseManager, protocol: &SarychProtocol, body: Option<Value>, username: &str) -> Result<Value, String> {
        let record = body.ok_or("Body required for POST operation")?;
        let message = db_manager.insert_record(username, &protocol.database, record)?;
        Ok(serde_json::json!({
            "operation": "post",
            "database": protocol.database,
            "message": message
        }))
    }

    async fn handle_put(db_manager: &DatabaseManager, protocol: &SarychProtocol, body: Option<Value>, username: &str, id_update: Option<&str>) -> Result<Value, String> {
        let update_data = body.ok_or("Body required for PUT operation")?;
        
        let message = if let Some(id) = id_update {
            // Update by ID
            db_manager.update_records(username, &protocol.database, "", update_data, Some(id))?
        } else {
            // Update by query (existing behavior)
            let query = protocol.query.as_deref().ok_or("Query or idUpdate header required for PUT operation")?;
            db_manager.update_records(username, &protocol.database, query, update_data, None)?
        };
        
        Ok(serde_json::json!({
            "operation": "put",
            "database": protocol.database,
            "query": protocol.query,
            "id_update": id_update,
            "message": message
        }))
    }

    async fn handle_delete(db_manager: &DatabaseManager, protocol: &SarychProtocol, username: &str) -> Result<Value, String> {
        let query = protocol.query.as_deref().ok_or("Query required for DELETE operation")?;
        let message = db_manager.delete_records(username, &protocol.database, query)?;
        Ok(serde_json::json!({
            "operation": "delete",
            "database": protocol.database,
            "query": query,
            "message": message
        }))
    }

    async fn handle_stats(db_manager: &DatabaseManager, protocol: &SarychProtocol, username: &str) -> Result<Value, String> {
        db_manager.get_stats(username, &protocol.database)
    }
    async fn health() -> Result<Value, String> {
        Ok(serde_json::json!({
            "operation": "health",
            "status": "ok",
            "message": "SarychDB is healthy"
        }))
    }

    // Public health check endpoint (no authentication required)
    pub async fn public_health() -> Result<impl Reply, Rejection> {
        let start_time = std::time::Instant::now();
        let operation_time = start_time.elapsed().as_millis();
        
        Ok(warp::reply::with_status(
            serde_json::json!({
                "status": "ok",
                "message": "SarychDB is healthy and running",
                "service": "SarychDB",
                "version": "2.0",
                "time": operation_time as u64
            }).to_string(),
            warp::http::StatusCode::OK,
        ))
    } 
    // Create user
    pub async fn create_user(request: CreateUserRequest) -> Result<impl Reply, Rejection> {
        let start_time = std::time::Instant::now();
        let auth_service = AuthService::new();
        match auth_service.create_user(request) {
            Ok(message) => {
                let operation_time = start_time.elapsed().as_millis();
                Ok(warp::reply::with_status(
                    serde_json::json!({
                        "message": message,
                        "time": operation_time as u64
                    }).to_string(),
                    warp::http::StatusCode::CREATED,
                ))
            },
            Err(e) => {
                let operation_time = start_time.elapsed().as_millis();
                Ok(warp::reply::with_status(
                    serde_json::json!({
                        "error": e,
                        "time": operation_time as u64
                    }).to_string(),
                    warp::http::StatusCode::BAD_REQUEST,
                ))
            },
        }
    }

    // Create database
    pub async fn create_database(request: CreateDbRequest) -> Result<impl Reply, Rejection> {
        let start_time = std::time::Instant::now();
        let auth_service = AuthService::new();
        match auth_service.create_database(request) {
            Ok(message) => {
                let operation_time = start_time.elapsed().as_millis();
                Ok(warp::reply::with_status(
                    serde_json::json!({
                        "message": message,
                        "time": operation_time as u64
                    }).to_string(),
                    warp::http::StatusCode::CREATED,
                ))
            },
            Err(e) => {
                let operation_time = start_time.elapsed().as_millis();
                Ok(warp::reply::with_status(
                    serde_json::json!({
                        "error": e,
                        "time": operation_time as u64
                    }).to_string(),
                    warp::http::StatusCode::BAD_REQUEST,
                ))
            },
        }
    }

    // List user databases
    pub async fn list_databases(username: String, password: String) -> Result<impl Reply, Rejection> {
        let start_time = std::time::Instant::now();
        let auth_service = AuthService::new();
        match auth_service.get_user_databases(&username, &password) {
            Ok(databases) => {
                let operation_time = start_time.elapsed().as_millis();
                Ok(warp::reply::with_status(
                    serde_json::json!({
                        "user": username,
                        "databases": databases,
                        "time": operation_time as u64
                    }).to_string(),
                    warp::http::StatusCode::OK,
                ))
            },
            Err(e) => {
                let operation_time = start_time.elapsed().as_millis();
                Ok(warp::reply::with_status(
                    serde_json::json!({
                        "error": e,
                        "time": operation_time as u64
                    }).to_string(),
                    warp::http::StatusCode::UNAUTHORIZED,
                ))
            },
        }
    }

    // Clear search cache endpoint
    pub async fn clear_cache(username: String, password: String) -> Result<impl Reply, Rejection> {
        let start_time = std::time::Instant::now();
        let auth_service = AuthService::new();
        
        // Verify authentication
        if let Err(e) = auth_service.authenticate(&username, &password) {
            let operation_time = start_time.elapsed().as_millis();
            return Ok(warp::reply::with_status(
                serde_json::json!({
                    "error": format!("Authentication error: {}", e),
                    "time": operation_time as u64
                }).to_string(),
                warp::http::StatusCode::UNAUTHORIZED,
            ));
        }

        // Clear both database and search caches
        use crate::modules::search::clear_search_cache;
        clear_search_cache();
        
        let operation_time = start_time.elapsed().as_millis();
        Ok(warp::reply::with_status(
            serde_json::json!({
                "message": "All caches cleared successfully",
                "time": operation_time as u64
            }).to_string(),
            warp::http::StatusCode::OK,
        ))
    }

    // Configurar rutas del servidor
    pub fn routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        // CORS configuration
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type", "username", "password", "querytype", "idupdate", "page", "limit", "sortby", "sortorder", "filters", "authorization"])
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]);

        // Ruta para el protocolo SarychDB con autenticación por headers
        let sarych_route = warp::path("sarych")
            .and(warp::query::<HashMap<String, String>>())
            .and(warp::body::bytes())
            .and(warp::header::<String>("username"))
            .and(warp::header::<String>("password"))
            .and(warp::header::optional::<String>("queryType"))
            .and(warp::header::optional::<String>("idUpdate"))
            .and(warp::header::optional::<String>("page"))
            .and(warp::header::optional::<String>("limit"))
            .and(warp::header::optional::<String>("sortBy"))
            .and(warp::header::optional::<String>("sortOrder"))
            .and(warp::header::optional::<String>("filters"))
            .and_then(|params: HashMap<String, String>, body: bytes::Bytes, username: String, password: String, query_type: Option<String>, id_update: Option<String>, page: Option<String>, limit: Option<String>, sort_by: Option<String>, sort_order: Option<String>, filters: Option<String>| async move {
                 let url = params.get("url").ok_or_else(|| warp::reject::custom(RequestError::MissingUrl))?;
                 let json_body = if !body.is_empty() {
                     serde_json::from_slice(&body).ok()
                 } else {
                     None
                 };
                 SarychServer::handle_sarych_request(url.clone(), json_body, username, password, query_type, id_update, page, limit, sort_by, sort_order, filters).await
             });

        // Route to create users
        let create_user_route = warp::path("api")
            .and(warp::path("users"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then(|request: CreateUserRequest| async move {
                SarychServer::create_user(request).await
            });

        // Ruta para crear bases de datos
        let create_db_route = warp::path("api")
            .and(warp::path("databases"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then(|request: CreateDbRequest| async move {
                SarychServer::create_database(request).await
            });

        // Ruta para listar bases de datos
        let list_db_route = warp::path("api")
            .and(warp::path("databases"))
            .and(warp::get())
            .and(warp::query::<HashMap<String, String>>())
            .and_then(|params: HashMap<String, String>| async move {
                let username = params.get("username").ok_or_else(|| warp::reject::custom(RequestError::MissingUsername))?.clone();
                let password = params.get("password").ok_or_else(|| warp::reject::custom(RequestError::MissingPassword))?.clone();
                SarychServer::list_databases(username, password).await
            });

        // Public health check endpoint
        let health_route = warp::path("health")
            .and(warp::get())
            .and_then(|| async move {
                SarychServer::public_health().await
            });

        // Clear cache endpoint
        let clear_cache_route = warp::path("api")
            .and(warp::path("cache"))
            .and(warp::path("clear"))
            .and(warp::delete())
            .and(warp::query::<HashMap<String, String>>())
            .and_then(|params: HashMap<String, String>| async move {
                let username = params.get("username").ok_or_else(|| warp::reject::custom(RequestError::MissingUsername))?.clone();
                let password = params.get("password").ok_or_else(|| warp::reject::custom(RequestError::MissingPassword))?.clone();
                SarychServer::clear_cache(username, password).await
            });

        sarych_route
            .or(create_user_route)
            .or(create_db_route)
            .or(list_db_route)
            .or(health_route)
            .or(clear_cache_route)
            .with(cors)
    }
}

// Errores personalizados
#[derive(Debug)]
enum RequestError {
    MissingUrl,
    MissingUsername,
    MissingPassword,
}

impl warp::reject::Reject for RequestError {}

pub async fn start_server(port: u16) {
    let routes = SarychServer::routes();

        println!("🚀 SarychDB server started on port {}", port);
        println!("📖 API documentation:");
        println!("  GET /health - Health check (public)");
        println!("  POST /api/users - Create user");
        println!("  POST /api/databases - Create database");
        println!("  GET /api/databases - List databases");
        println!("  GET /sarych?url=sarychdb://user@pass/db/operation - SarychDB protocol");

    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;
}