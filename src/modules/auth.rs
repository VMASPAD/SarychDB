use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::env;
use bcrypt::{hash, verify, DEFAULT_COST};
use crate::modules::database::DatabaseManager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Database {
    pub namedb: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub user: String,
    pub password: String, // Password hash
    pub db: Vec<Database>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDbRequest {
    pub username: String,
    pub password: String,
    pub db_name: String,
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

fn legacy_data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn migrate_legacy_data_if_needed() {
    if env::var("SARYCHDB_DATA_DIR").is_ok() {
        return;
    }

    let new_root = data_root();
    let legacy_root = legacy_data_root();

    if new_root.exists() {
        return;
    }

    let legacy_users = legacy_root.join("users.json");
    let legacy_users_dir = legacy_root.join("users");
    if !legacy_users.exists() && !legacy_users_dir.exists() {
        return;
    }

    if let Err(e) = fs::create_dir_all(&new_root) {
        eprintln!("⚠️  Failed to create data directory {:?}: {}", new_root, e);
        return;
    }

    let new_users = new_root.join("users.json");
    if legacy_users.exists() && !new_users.exists() {
        if let Err(e) = fs::rename(&legacy_users, &new_users) {
            eprintln!("⚠️  Failed to move users.json to {:?}: {}", new_users, e);
        }
    }

    let new_users_dir = new_root.join("users");
    if legacy_users_dir.exists() && !new_users_dir.exists() {
        if let Err(e) = fs::rename(&legacy_users_dir, &new_users_dir) {
            eprintln!("⚠️  Failed to move users directory to {:?}: {}", new_users_dir, e);
        }
    }
}

fn users_file_path() -> PathBuf {
    data_root().join("users.json")
}

fn user_dir_path(username: &str) -> PathBuf {
    data_root().join("users").join(username)
}

pub struct AuthService;

impl AuthService {
    pub fn new() -> Self {
        migrate_legacy_data_if_needed();
        // Initialize users.json file if it doesn't exist
        let users_file = users_file_path();
        if !users_file.exists() {
            let empty_users: Vec<User> = vec![];
            let json = serde_json::to_string_pretty(&empty_users).unwrap();
            if let Some(parent) = users_file.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(users_file, json).unwrap();
        }
        Self
    }

    pub fn load_users() -> Result<Vec<User>, Box<dyn std::error::Error>> {
        let data = fs::read_to_string(users_file_path())?;
        let users: Vec<User> = serde_json::from_str(&data)?;
        Ok(users)
    }

    pub fn save_users(users: &[User]) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(users)?;
        fs::write(users_file_path(), json)?;
        Ok(())
    }

    pub fn create_user(&self, request: CreateUserRequest) -> Result<String, String> {
        let mut users = Self::load_users().map_err(|e| e.to_string())?;
        
        // Check if user already exists
        if users.iter().any(|u| u.user == request.username) {
            return Err("User already exists".to_string());
        }

        // Validate username (no spaces, special characters)
        if request.username.is_empty() || request.username.contains(' ') || 
           request.username.contains('/') || request.username.contains('\\') {
            return Err("Invalid username. Cannot contain spaces or special characters".to_string());
        }

        // Hash the password
        let password_hash = hash(request.password.as_bytes(), DEFAULT_COST)
            .map_err(|e| e.to_string())?;

        // Create user folder
        let user_dir = user_dir_path(&request.username);
        fs::create_dir_all(&user_dir).map_err(|e| format!("Error creating user folder: {}", e))?;

        // Create new user
        let new_user = User {
            user: request.username.clone(),
            password: password_hash,
            db: vec![],
        };

        users.push(new_user);
        Self::save_users(&users).map_err(|e| e.to_string())?;

        Ok(format!("User '{}' created successfully with folder at: {}", request.username, user_dir.display()))
    }

    pub fn authenticate(&self, username: &str, password: &str) -> Result<bool, String> {
        let users = Self::load_users().map_err(|e| e.to_string())?;
        
        if let Some(user) = users.iter().find(|u| u.user == username) {
            verify(password, &user.password).map_err(|e| e.to_string())
        } else {
            Ok(false)
        }
    }

    pub fn create_database(&self, request: CreateDbRequest) -> Result<String, String> {
        // Verify authentication
        if !self.authenticate(&request.username, &request.password)? {
            return Err("Invalid credentials".to_string());
        }

        // Validate database name
        if request.db_name.is_empty() || request.db_name.contains(' ') || 
           request.db_name.contains('/') || request.db_name.contains('\\') {
            return Err("Invalid database name. Cannot contain spaces or special characters".to_string());
        }

        let mut users = Self::load_users().map_err(|e| e.to_string())?;
        
        // Find the user
        if let Some(user) = users.iter_mut().find(|u| u.user == request.username) {
            // Check if DB already exists
            if user.db.iter().any(|db| db.namedb == request.db_name) {
                return Err("Database already exists for this user".to_string());
            }

            // Create empty JSON file for the DB in user folder
            let user_dir = user_dir_path(&request.username);
            let db_filepath = user_dir.join(format!("{}.json", request.db_name));
            
            // Verify that user folder exists
            if !user_dir.exists() {
                fs::create_dir_all(&user_dir).map_err(|e| format!("Error creating user folder: {}", e))?;
            }

            // Check if file already exists with that name (prevent global duplicates)
            if db_filepath.exists() {
                return Err("File with that name already exists in user folder".to_string());
            }

            let empty_data: Vec<serde_json::Value> = vec![];
            let json = serde_json::to_string_pretty(&empty_data).unwrap();
            fs::write(&db_filepath, json).map_err(|e| format!("Error creating database file: {}", e))?;

            // Add DB to user
            user.db.push(Database {
                namedb: request.db_name.clone(),
            });

            Self::save_users(&users).map_err(|e| e.to_string())?;
            Ok(format!("Database '{}' created successfully at: {}", request.db_name, db_filepath.display()))
        } else {
            Err("User not found".to_string())
        }
    }

    pub fn delete_database(&self, username: &str, password: &str, db_name: &str) -> Result<String, String> {
        if !self.authenticate(username, password)? {
            return Err("Invalid credentials".to_string());
        }

        if db_name.is_empty() || db_name.contains(' ') || db_name.contains('/') || db_name.contains('\\') {
            return Err("Invalid database name. Cannot contain spaces or special characters".to_string());
        }

        let mut users = Self::load_users().map_err(|e| e.to_string())?;

        if let Some(user) = users.iter_mut().find(|u| u.user == username) {
            let initial_len = user.db.len();
            user.db.retain(|db| db.namedb != db_name);

            if user.db.len() == initial_len {
                return Err("Database not found for this user".to_string());
            }

            let db_filepath = user_dir_path(username).join(format!("{}.json", db_name));
            if db_filepath.exists() {
                fs::remove_file(&db_filepath).map_err(|e| format!("Error deleting database file: {}", e))?;
            }

            Self::save_users(&users).map_err(|e| e.to_string())?;
            Ok(format!("Database '{}' deleted successfully", db_name))
        } else {
            Err("User not found".to_string())
        }
    }

    pub fn rename_database(&self, username: &str, password: &str, old_name: &str, new_name: &str) -> Result<String, String> {
        if !self.authenticate(username, password)? {
            return Err("Invalid credentials".to_string());
        }

        if new_name.is_empty() || new_name.contains(' ') || new_name.contains('/') || new_name.contains('\\') {
            return Err("Invalid new database name. Cannot contain spaces or special characters".to_string());
        }

        let mut users = Self::load_users().map_err(|e| e.to_string())?;

        if let Some(user) = users.iter_mut().find(|u| u.user == username) {
            // Check existence of old DB
            if !user.db.iter().any(|db| db.namedb == old_name) {
                return Err("Database not found for this user".to_string());
            }

            // Prevent overwriting an existing DB with the new name
            if user.db.iter().any(|db| db.namedb == new_name) {
                return Err("A database with the new name already exists".to_string());
            }

            let old_path = user_dir_path(username).join(format!("{}.json", old_name));
            let new_path = user_dir_path(username).join(format!("{}.json", new_name));

            if !old_path.exists() {
                return Err("Database file not found".to_string());
            }

            if new_path.exists() {
                return Err("File with the new name already exists".to_string());
            }

            fs::rename(&old_path, &new_path).map_err(|e| format!("Error renaming database file: {}", e))?;

            // Update users metadata
            for db in &mut user.db {
                if db.namedb == old_name {
                    db.namedb = new_name.to_string();
                    break;
                }
            }

            Self::save_users(&users).map_err(|e| e.to_string())?;

            // Invalidate caches for old and new db names
            DatabaseManager::invalidate_cache(username, old_name);
            DatabaseManager::invalidate_cache(username, new_name);

            Ok(format!("Database '{}' renamed to '{}'", old_name, new_name))
        } else {
            Err("User not found".to_string())
        }
    }

    pub fn get_user_databases(&self, username: &str, password: &str) -> Result<Vec<Database>, String> {
        if !self.authenticate(username, password)? {
            return Err("Invalid credentials".to_string());
        }

        let users = Self::load_users().map_err(|e| e.to_string())?;
        
        if let Some(user) = users.iter().find(|u| u.user == username) {
            Ok(user.db.clone())
        } else {
            Err("User not found".to_string())
        }
    }

    pub fn user_has_database(&self, username: &str, password: &str, db_name: &str) -> Result<bool, String> {
        if !self.authenticate(username, password)? {
            return Err("Invalid credentials".to_string());
        }

        let users = Self::load_users().map_err(|e| e.to_string())?;

        if let Some(user) = users.iter().find(|u| u.user == username) {
            Ok(user.db.iter().any(|db| db.namedb == db_name))
        } else {
            Err("User not found".to_string())
        }
    }

    /// Authenticate and verify database access in a single pass (one bcrypt + one disk read).
    /// Returns Ok(()) on success, Err on bad credentials or missing database.
    pub fn authenticate_and_check_db(
        &self,
        username: &str,
        password: &str,
        db_name: &str,
    ) -> Result<(), String> {
        let users = Self::load_users().map_err(|e| e.to_string())?;
        let user = users
            .iter()
            .find(|u| u.user == username)
            .ok_or_else(|| "Invalid credentials".to_string())?;

        if !verify(password, &user.password).map_err(|e| e.to_string())? {
            return Err("Invalid credentials".to_string());
        }

        if user.db.iter().any(|db| db.namedb == db_name) {
            Ok(())
        } else {
            Err(format!("Database '{}' not found for this user", db_name))
        }
    }
}