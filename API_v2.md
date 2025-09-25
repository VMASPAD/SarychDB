# 🚀 SarychDB API v2.0 - Complete Documentation & Testing Guide

## 🌟 New Features in v2.0

### ✅ Header-Based Authentication
All APIs now use headers for authentication instead of URL parameters for better security.

### ✅ Advanced Query Types
- **key**: Search by specific key existence
- **value**: Search by value in any part of the structure  

### ✅ ID-Based Updates
Use `idUpdate` header to update specific records by their `_id` field.

### ✅ Operation Timing
All API responses now include a `time` field showing operation duration in milliseconds.

### ✅ File Read Statistics
Database stats now include `read_time_ms` showing how long it took to read the file.

### ✅ User-Isolated File System
Each user gets their own folder (`users/{username}/`) for complete data isolation.

---

## ⚙️ Configuration and Setup

### Prerequisites
- Rust (latest stable version)
- Cargo package manager

### Installation
```bash
git clone <repository-url>
cd SarychDB
cargo build --release
```

### Running the Server
```bash
# Development mode
cargo run

# Production mode  
cargo run --release

# Benchmark mode
cargo run benchmark
```

---

## 🌐 HTTP API Endpoints

### 1. Create User
```bash
curl -X POST http://localhost:3030/api/users \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "my_secure_password"
  }'
```

**Response:**
```json
{
  "message": "User 'admin' created successfully with folder at: users/admin",
  "time": 45
}
```

### 2. Create Database
```bash
curl -X POST http://localhost:3030/api/databases \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "my_secure_password",
    "db_name": "my_database"
  }'
```

**Response:**
```json
{
  "message": "Database 'my_database' created successfully at: users/admin/my_database.json",
  "time": 23
}
```

### 3. List User Databases
```bash
curl "http://localhost:3030/api/databases?username=admin&password=my_secure_password"
```

**Response:**
```json
{
  "user": "admin",
  "databases": [{"namedb": "my_database"}],
  "time": 12
}
```

## 🔗 SarychDB Protocol (Updated)

### New URL Format:
```
/database/operation?query=search_value
```

### Required Headers:
- `username`: Your username
- `password`: Your password

### Optional Headers:
- `queryType`: "key" or "value" 
- `idUpdate`: Record ID for PUT operations

### Available Operations:

#### GET - Search Records
```bash
# Search all records
curl "http://localhost:3030/sarych?url=/my_database/get" \
  -H "username: admin" \
  -H "password: my_secure_password"

# Search by key existence
curl "http://localhost:3030/sarych?url=/my_database/get?query=name" \
  -H "username: admin" \
  -H "password: my_secure_password" \
  -H "queryType: key"

# Search by value
curl "http://localhost:3030/sarych?url=/my_database/get?query=John" \
  -H "username: admin" \
  -H "password: my_secure_password" \
  -H "queryType: value"
```

**Response:**
```json
{
  "operation": "get",
  "database": "my_database",
  "query": "John",
  "query_type": "value",
  "results": [...],
  "count": 5,
  "time": 34
}
```

#### POST - Insert Record
```bash
curl -X POST "http://localhost:3030/sarych?url=/my_database/post" \
  -H "username: admin" \
  -H "password: my_secure_password" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "John Doe",
    "age": 30,
    "email": "john@email.com",
    "active": true
  }'
```

**Response:**
```json
{
  "operation": "post",
  "database": "my_database", 
  "message": "Record inserted successfully",
  "time": 18
}
```

#### PUT - Update Records

**By Query:**
```bash
curl -X PUT "http://localhost:3030/sarych?url=/my_database/put?query=John" \
  -H "username: admin" \
  -H "password: my_secure_password" \
  -H "Content-Type: application/json" \
  -d '{
    "age": 31,
    "city": "New York"
  }'
```

**By ID:**
```bash
curl -X PUT "http://localhost:3030/sarych?url=/my_database/put" \
  -H "username: admin" \
  -H "password: my_secure_password" \
  -H "idUpdate: 550e8400-e29b-41d4-a716-446655440000" \
  -H "Content-Type: application/json" \
  -d '{
    "age": 31,
    "city": "New York"
  }'
```

**Response:**
```json
{
  "operation": "put",
  "database": "my_database",
  "query": "John",
  "id_update": null,
  "message": "Updated 2 records",
  "time": 28
}
```

#### DELETE - Delete Records
```bash
curl -X DELETE "http://localhost:3030/sarych?url=/my_database/delete?query=inactive" \
  -H "username: admin" \
  -H "password: my_secure_password"
```

**Response:**
```json
{
  "operation": "delete",
  "database": "my_database",
  "query": "inactive",
  "message": "Deleted 3 records",
  "time": 25
}
```

#### STATS - Database Statistics  
```bash
curl "http://localhost:3030/sarych?url=/my_database/stats" \
  -H "username: admin" \
  -H "password: my_secure_password"
```

**Response:**
```json
{
  "database": "my_database",
  "username": "admin",
  "total_records": 1250,
  "size_bytes": 2048576,
  "read_time_ms": 45,
  "timestamp": "2025-09-24T10:30:00Z",
  "time": 52
}
```

## 🔍 Search Engine Features

### Query Types:
- **Default**: Searches entire JSON structure recursively
- **key**: Returns records that have the specified key name
- **value**: Returns records containing the specified value anywhere

### Examples:
```bash
# Find all records with "email" field
curl "http://localhost:3030/sarych?url=/users/get?query=email" \
  -H "username: admin" \
  -H "password: password123" \
  -H "queryType: key"

# Find all records containing "gmail.com"  
curl "http://localhost:3030/sarych?url=/users/get?query=gmail.com" \
  -H "username: admin" \
  -H "password: password123" \
  -H "queryType: value"
```

## 🔐 Security Features

- ✅ Header-based authentication  
- ✅ User-isolated file system (`users/{username}/`)
- ✅ Bcrypt password hashing
- ✅ Database access validation per user
- ✅ Unique name validation for users and databases

## 📁 File Structure

```
users.json                    # User registry
users/
├── admin/                   # User-specific folder  
│   ├── products.json        # User's database files
│   └── orders.json
└── testuser/
    └── inventory.json
```

## ⚡ Performance Features

- ✅ Parallel search engine with multi-node processing
- ✅ Operation timing in all responses  
- ✅ File read time measurement
- ✅ Automatic record metadata (`_id`, `_created_at`, `_updated_at`)
- ✅ Efficient JSON structure exploration

---

## 🧪 Complete Testing Guide

### ✅ Server Status
Start the server with: `cargo run` (runs on http://localhost:3030)

### 📋 Step-by-Step Testing

#### 1. Create a User
```bash
curl -X POST http://localhost:3030/api/users \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "password123"
  }'
```

**Expected Response:**
```json
{
  "message": "User 'testuser' created successfully with folder at: users/testuser",
  "time": 45
}
```

#### 2. Create a Database
```bash
curl -X POST http://localhost:3030/api/databases \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "password123", 
    "db_name": "products"
  }'
```

**Expected Response:**
```json
{
  "message": "Database 'products' created successfully at: users/testuser/products.json",
  "time": 23
}
```

#### 3. Insert Test Data (New Header-Based Format)

**Product 1:**
```bash
curl -X POST "http://localhost:3030/sarych?url=/products/post" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Gaming Laptop",
    "price": 1299.99,
    "category": "electronics",
    "brand": "TechPro",
    "code": "P1605",
    "stock": 15,
    "active": true
  }'
```

**Product 2:**
```bash
curl -X POST "http://localhost:3030/sarych?url=/products/post" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Wireless Mouse",
    "price": 29.99,
    "category": "accessories",
    "brand": "TechPro", 
    "code": "A1001",
    "stock": 50,
    "active": true
  }'
```

**Product 3:**
```bash
curl -X POST "http://localhost:3030/sarych?url=/products/post" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Mechanical Keyboard",
    "price": 89.99,
    "category": "accessories",
    "brand": "KeyMaster",
    "code": "K2003", 
    "stock": 25,
    "active": false
  }'
```

#### 4. Test Advanced Search Features

**Search All Products:**
```bash
curl "http://localhost:3030/sarych?url=/products/get" \
  -H "username: testuser" \
  -H "password: password123"
```

**Search by Key Existence (find all products with 'brand' field):**
```bash
curl "http://localhost:3030/sarych?url=/products/get?query=brand" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "queryType: key"
```

**Search by Value (find all TechPro products):**
```bash
curl "http://localhost:3030/sarych?url=/products/get?query=TechPro" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "queryType: value"
```

**Search Specific Code:**
```bash
curl "http://localhost:3030/sarych?url=/products/get?query=P1605" \
  -H "username: testuser" \
  -H "password: password123"
```

#### 5. Test Update Operations

**Update by Query:**
```bash
curl -X PUT "http://localhost:3030/sarych?url=/products/put?query=P1605" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "Content-Type: application/json" \
  -d '{
    "stock": 10,
    "price": 1199.99
  }'
```

**Update by ID (first get the _id from a search, then use it):**
```bash
# First, get a record ID from search results, then:
curl -X PUT "http://localhost:3030/sarych?url=/products/put" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "idUpdate: [ID-FROM-SEARCH-RESULT]" \
  -H "Content-Type: application/json" \
  -d '{
    "discount": 10,
    "featured": true
  }'
```

#### 6. View Database Statistics
```bash
curl "http://localhost:3030/sarych?url=/products/stats" \
  -H "username: testuser" \
  -H "password: password123"
```

**Expected Response:**
```json
{
  "database": "products",
  "username": "testuser",
  "total_records": 3,
  "size_bytes": 1024,
  "read_time_ms": 5,
  "timestamp": "2025-09-24T10:30:00Z",
  "time": 12
}
```

#### 7. Delete Inactive Products
```bash
curl -X DELETE "http://localhost:3030/sarych?url=/products/delete?query=false" \
  -H "username: testuser" \
  -H "password: password123"
```

### 📁 Generated Files Structure

After completing the tests, you should see:
```
users.json                           # User registry
users/
└── testuser/                        # User-specific folder
    └── products.json               # User's database file
```

### 🎯 Advanced Testing Scenarios

#### Test Multiple Users:
```bash
# Create second user
curl -X POST http://localhost:3030/api/users \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123"}'

# Create database for second user  
curl -X POST http://localhost:3030/api/databases \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123", "db_name": "inventory"}'
```

#### Test Cross-User Access (should fail):
```bash
# Try to access testuser's products with admin credentials (should fail)
curl "http://localhost:3030/sarych?url=/products/get" \
  -H "username: admin" \
  -H "password: admin123"
```

#### Performance Testing:
```bash
# Run benchmark mode
cargo run benchmark

# Test with large datasets
# Insert multiple records and measure response times
```

### 🔍 Troubleshooting Guide

**Common Issues:**
1. **Authentication Error**: Check username/password in headers
2. **Database Not Found**: Ensure database was created for the specific user
3. **Permission Denied**: Verify user has access to the database
4. **Invalid JSON**: Check request body format

**Success Indicators:**
- All responses include `time` field
- Stats show `read_time_ms`
- User folders created in `users/` directory
- Each operation returns appropriate status codes