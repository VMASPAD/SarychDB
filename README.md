# SarychDB

<div align="center">
  <img src="SDB.svg" alt="SarychDB Logo" width="200"/>
</div>

## Library

[NPM](https://www.npmjs.com/package/sarychdb-client)


## 🚀 Start the Server

```bash
cargo run
```

The server will start on port 3030 by default.

## 📋 API Endpoints

### 1. Create User
```bash
curl -X POST http://localhost:3030/api/users \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "my_secure_password"
  }'
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

### 3. List User Databases
```bash
curl "http://localhost:3030/api/databases?username=admin&password=my_secure_password"
```

## 🔗 SarychDB Protocol

### URL Format:
```
sarychdb://username@password/database/operation?query=search_value
```

### Available Operations:

#### GET - Search records
```bash
# Search all records
curl "http://localhost:3030/sarych?url=sarychdb://admin@my_secure_password/my_database/get"

# Search records containing "value"
curl "http://localhost:3030/sarych?url=sarychdb://admin@my_secure_password/my_database/get?query=value"
```

#### POST - Insert record
```bash
curl -X POST "http://localhost:3030/sarych?url=sarychdb://admin@my_secure_password/my_database/post" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "John Doe",
    "age": 30,
    "email": "john@email.com",
    "active": true
  }'
```

#### PUT - Update records
```bash
# Update all records containing "John"
curl -X PUT "http://localhost:3030/sarych?url=sarychdb://admin@my_secure_password/my_database/put?query=John" \
  -H "Content-Type: application/json" \
  -d '{
    "age": 31,
    "city": "New York"
  }'
```

#### DELETE - Delete records
```bash
# Delete all records containing "inactive"
curl -X DELETE "http://localhost:3030/sarych?url=sarychdb://admin@my_secure_password/my_database/delete?query=inactive"
```

#### STATS - Database statistics
```bash
curl "http://localhost:3030/sarych?url=sarychdb://admin@my_secure_password/my_database/stats"
```

## 🔍 Parallel Search Engine

The system uses a parallel search engine that:

- **Searches entire JSON structure**: Not just specific fields, but any value
- **Automatic parallelization**: Divides data into nodes for parallel search
- **Recursive search**: Explores nested arrays and objects
- **Multiple data types**: Strings, numbers, booleans, etc.

## 🔐 Authentication

- Users are stored in `users.json`
- Passwords are encrypted with bcrypt
- Each user has access only to their own databases

## 📁 File Structure

- `users.json` - Users and their databases
- `users/{username}/` - User-specific folder
- `users/{username}/{db_name}.json` - Individual database files
- Each record includes automatic metadata (`_id`, `_created_at`, `_updated_at`)

## ⚡ Benchmark Mode

To run search engine benchmarks:

```bash
cargo run benchmark
```

## 🌟 Features

- ✅ Custom `sarychdb://` protocol
- ✅ Complete CRUD operations
- ✅ Multi-node parallel search
- ✅ User authentication with passwords
- ✅ Flexible JSON databases
- ✅ REST API for administration
- ✅ Automatic record metadata
- ✅ Database statistics
- ✅ User-isolated file system
- ✅ Duplicate name prevention