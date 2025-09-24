use warp::{Filter, Reply, Rejection};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;
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
    auth_service: AuthService,
    db_manager: DatabaseManager,
}

impl SarychServer {
    pub fn new() -> Self {
        Self {
            auth_service: AuthService::new(),
            db_manager: DatabaseManager::new(),
        }
    }

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

    // Manejar operaciones del protocolo SarychDB
    pub async fn handle_sarych_request(url_str: String, body: Option<Value>) -> Result<impl Reply, Rejection> {
        let auth_service = AuthService::new();
        let db_manager = DatabaseManager::new();
        let protocol = match Self::parse_sarych_url(&url_str) {
            Ok(p) => p,
            Err(e) => return Ok(warp::reply::with_status(e, warp::http::StatusCode::BAD_REQUEST)),
        };

        // Verify authentication
        if let Err(e) = auth_service.authenticate(&protocol.username, &protocol.password) {
            return Ok(warp::reply::with_status(
                format!("Authentication error: {}", e),
                warp::http::StatusCode::UNAUTHORIZED,
            ));
        }

        // Verify user has access to database
        if let Err(e) = auth_service.user_has_database(&protocol.username, &protocol.password, &protocol.database) {
            return Ok(warp::reply::with_status(
                format!("Database access denied: {}", e),
                warp::http::StatusCode::FORBIDDEN,
            ));
        }

        // Process operation
        let result = match protocol.operation.to_lowercase().as_str() {
            "get" => Self::handle_get(&db_manager, &protocol).await,
            "post" => Self::handle_post(&db_manager, &protocol, body).await,
            "put" => Self::handle_put(&db_manager, &protocol, body).await,
            "delete" => Self::handle_delete(&db_manager, &protocol).await,
            "stats" => Self::handle_stats(&db_manager, &protocol).await,
            _ => Err("Unsupported operation. Use: get, post, put, delete, stats".to_string()),
        };

        match result {
            Ok(response) => Ok(warp::reply::with_status(
                serde_json::to_string(&response).unwrap_or_default(),
                warp::http::StatusCode::OK,
            )),
            Err(e) => Ok(warp::reply::with_status(
                e,
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            )),
        }
    }

    async fn handle_get(db_manager: &DatabaseManager, protocol: &SarychProtocol) -> Result<Value, String> {
        let results = db_manager.search_records(&protocol.username, &protocol.database, protocol.query.as_deref())?;
        Ok(serde_json::json!({
            "operation": "get",
            "database": protocol.database,
            "query": protocol.query,
            "results": results,
            "count": results.len()
        }))
    }

    async fn handle_post(db_manager: &DatabaseManager, protocol: &SarychProtocol, body: Option<Value>) -> Result<Value, String> {
        let record = body.ok_or("Body required for POST operation")?;
        let message = db_manager.insert_record(&protocol.username, &protocol.database, record)?;
        Ok(serde_json::json!({
            "operation": "post",
            "database": protocol.database,
            "message": message
        }))
    }

    async fn handle_put(db_manager: &DatabaseManager, protocol: &SarychProtocol, body: Option<Value>) -> Result<Value, String> {
        let update_data = body.ok_or("Body required for PUT operation")?;
        let query = protocol.query.as_deref().ok_or("Query required for PUT operation")?;
        let message = db_manager.update_records(&protocol.username, &protocol.database, query, update_data)?;
        Ok(serde_json::json!({
            "operation": "put",
            "database": protocol.database,
            "query": query,
            "message": message
        }))
    }

    async fn handle_delete(db_manager: &DatabaseManager, protocol: &SarychProtocol) -> Result<Value, String> {
        let query = protocol.query.as_deref().ok_or("Query required for DELETE operation")?;
        let message = db_manager.delete_records(&protocol.username, &protocol.database, query)?;
        Ok(serde_json::json!({
            "operation": "delete",
            "database": protocol.database,
            "query": query,
            "message": message
        }))
    }

    async fn handle_stats(db_manager: &DatabaseManager, protocol: &SarychProtocol) -> Result<Value, String> {
        db_manager.get_stats(&protocol.username, &protocol.database)
    }

    // Crear usuario
    pub async fn create_user(request: CreateUserRequest) -> Result<impl Reply, Rejection> {
        let auth_service = AuthService::new();
        match auth_service.create_user(request) {
            Ok(message) => Ok(warp::reply::with_status(
                serde_json::json!({"message": message}).to_string(),
                warp::http::StatusCode::CREATED,
            )),
            Err(e) => Ok(warp::reply::with_status(
                serde_json::json!({"error": e}).to_string(),
                warp::http::StatusCode::BAD_REQUEST,
            )),
        }
    }

    // Crear base de datos
    pub async fn create_database(request: CreateDbRequest) -> Result<impl Reply, Rejection> {
        let auth_service = AuthService::new();
        match auth_service.create_database(request) {
            Ok(message) => Ok(warp::reply::with_status(
                serde_json::json!({"message": message}).to_string(),
                warp::http::StatusCode::CREATED,
            )),
            Err(e) => Ok(warp::reply::with_status(
                serde_json::json!({"error": e}).to_string(),
                warp::http::StatusCode::BAD_REQUEST,
            )),
        }
    }

    // Listar bases de datos de un usuario
    pub async fn list_databases(username: String, password: String) -> Result<impl Reply, Rejection> {
        let auth_service = AuthService::new();
        match auth_service.get_user_databases(&username, &password) {
            Ok(databases) => Ok(warp::reply::with_status(
                serde_json::json!({
                    "user": username,
                    "databases": databases
                }).to_string(),
                warp::http::StatusCode::OK,
            )),
            Err(e) => Ok(warp::reply::with_status(
                serde_json::json!({"error": e}).to_string(),
                warp::http::StatusCode::UNAUTHORIZED,
            )),
        }
    }

    // Configurar rutas del servidor
    pub fn routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        // Ruta para el protocolo SarychDB
        let sarych_route = warp::path("sarych")
            .and(warp::query::<HashMap<String, String>>())
            .and(warp::body::bytes())
            .and_then(|params: HashMap<String, String>, body: bytes::Bytes| async move {
                let url = params.get("url").ok_or_else(|| warp::reject::custom(RequestError::MissingUrl))?;
                let json_body = if !body.is_empty() {
                    serde_json::from_slice(&body).ok()
                } else {
                    None
                };
                SarychServer::handle_sarych_request(url.clone(), json_body).await
            });

        // Ruta para crear usuarios
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

        sarych_route
            .or(create_user_route)
            .or(create_db_route)
            .or(list_db_route)
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

    println!("🚀 SarychDB Server iniciado en puerto {}", port);
    println!("📖 Documentación de la API:");
    println!("  POST /api/users - Crear usuario");
    println!("  POST /api/databases - Crear base de datos");
    println!("  GET /api/databases - Listar bases de datos");
    println!("  GET /sarych?url=sarychdb://user@pass/db/operation - Protocolo SarychDB");

    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;
}