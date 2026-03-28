# SarychDB

<div align="center">
  <img src="SDB.svg" alt="SarychDB Logo" width="200"/>
</div>

---

SarychDB is a lightweight, non-relational document database written in Rust. It stores collections of JSON objects on disk, exposes a custom TCP protocol, and uses Rayon-powered parallel search with an in-memory read cache for high-throughput workloads.

## Features

- **Parallel search** — Rayon multi-core full-text search across all JSON fields
- **Smart caching** — 5-minute in-memory cache for reads; auto-invalidated on writes
- **Flexible pagination** — limit-only, page+limit, or sensible defaults
- **Advanced queries** — filter, sort, and paginate with the `list` operation
- **Case-insensitive search** — `queryType: "icontains"` for accent/case-agnostic lookups
- **bcrypt auth** — passwords are hashed; credentials are verified on every operation
- **Per-user databases** — each user has their own isolated directory

---

## Quick Start

```bash
# Start the server (default port 4040)
cargo run

# Custom port
cargo run -- --port 5000

# Benchmark mode (requires 500MB.json in the project root)
cargo run -- benchmark --nodes 8
```

### CLI Options

| Flag | Default | Description |
|------|---------|-------------|
| `--port` / `--protocol-port` | `4040` | TCP port to listen on |
| `--threads <N>` | CPU count | Rayon thread pool size |
| `--background` / `--silent` | off | Suppress startup output |
| `benchmark` | — | Run built-in benchmark instead of server |
| `--nodes <N>` | CPU count | Node count for benchmark |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `SARYCHDB_PROTOCOL_PORT` | Override default port |
| `PORT` | Fallback port (same as above) |
| `SARYCHDB_DATA_DIR` | Override data directory |

**Default data directory:** `~/Documents/SarychDB/`

---

## Protocol

SarychDB listens on a raw TCP socket. Each request is a single message; the server responds with a single JSON object followed by a newline.

### Two message formats are accepted:

**1. Plain URL (read-only shorthand)**
```
sarychdb://username@password/database/operation?query=value
```

**2. JSON message (full control)**
```json
{
  "url":       "sarychdb://username@password/database/operation",
  "op":        "post",
  "body":      { "field": "value" },
  "query":     "search term",
  "queryType": "key | value | icontains",
  "idUpdate":  "uuid-string",
  "updateData": { "field": "new value" },
  "page":      1,
  "limit":     50,
  "sortBy":    "price",
  "sortOrder": "asc | desc",
  "filters":   { "active": true, "category": ["a", "b"] }
}
```

All fields except `url` are optional.

---

## Data Format

SarychDB stores **arrays of JSON objects**. Each record must be a `{}` object.

```json
[
  { "id": 1, "name": "Product A", "price": 99.99 },
  { "id": 2, "name": "Product B", "price": 149.99 }
]
```

Records receive two automatic metadata fields on insert:

| Field | Description |
|-------|-------------|
| `_id` | UUID v4, unique identifier |
| `_created_at` | RFC 3339 timestamp |

Update operations also add `_updated_at`.

---

## Operations

### User & Database Management

| Operation | Aliases | Description |
|-----------|---------|-------------|
| `create_user` | `signup` | Create user + optionally create a database |
| `create_db` | `create_database` | Create a new database for the authenticated user |
| `delete_db` | `delete_database` | Permanently delete a database and its file |
| `rename_db` | `rename_database`, `update_db_name` | Rename a database |
| `list_dbs` | `list_databases` | List all databases for the user |
| `all_dbs` | `get_all_dbs` | List databases with record counts |
| `health` | — | Health check, no auth required |

#### Create user + database
```
sarychdb://alice@secret123/products/create_user
```

#### Create database only
```json
{ "url": "sarychdb://alice@secret123/products/create_db" }
```

#### Rename database
```json
{
  "url": "sarychdb://alice@secret123/products/rename_db",
  "body": { "new_name": "inventory" }
}
```

---

### GET — Full-text Search

Searches every field in every record. Uses parallel search for large datasets with automatic caching.

```
sarychdb://alice@secret123/products/get?query=laptop
```

```json
{
  "url": "sarychdb://alice@secret123/products/get?query=laptop",
  "queryType": "key"
}
```

#### `queryType` options

| Value | Behavior |
|-------|----------|
| *(omitted)* | Case-sensitive full-text search across all fields (parallel + cached) |
| `key` | Return records that have a field named exactly `query` |
| `value` | Search only field values (case-sensitive) |
| `icontains` | Case-insensitive full-text search |

**Response:**
```json
{
  "operation": "get",
  "database": "products",
  "query": "laptop",
  "query_type": null,
  "results": [ { "_id": "...", "name": "Gaming Laptop", "price": 1299.99 } ],
  "count": 1,
  "time": 4
}
```

---

### BROWSE — Simple Pagination

Paginates all records without filtering. Three modes:

#### Limit only — first N records
```json
{ "url": "sarychdb://alice@secret123/products/browse", "limit": 200 }
```

#### Page + limit — classic pagination
```json
{ "url": "sarychdb://alice@secret123/products/browse", "page": 3, "limit": 50 }
```

#### Default — first 10 records
```json
{ "url": "sarychdb://alice@secret123/products/browse" }
```

**Response:**
```json
{
  "operation": "browse",
  "database": "products",
  "data": [ /* records */ ],
  "pagination": {
    "page": 3,
    "limit": 50,
    "returned": 50,
    "total_records": 1500,
    "total_pages": 30,
    "has_next": true,
    "has_prev": true,
    "mode": "paginated"
  },
  "time": 3
}
```

`mode` is `"limit_only"`, `"paginated"`, or `"default"`.

> Note: `page` requires `limit`. Sending `page` without `limit` returns an error.

---

### LIST — Filter + Sort + Paginate

```json
{
  "url": "sarychdb://alice@secret123/products/list",
  "page": 2,
  "limit": 50,
  "sortBy": "price",
  "sortOrder": "desc",
  "filters": { "category": "electronics", "active": true }
}
```

#### Filter with OR logic (array value)
```json
{
  "url": "sarychdb://alice@secret123/products/list",
  "limit": 100,
  "filters": { "category": ["electronics", "accessories"] }
}
```

**Response:**
```json
{
  "operation": "list",
  "database": "products",
  "data": [ /* records */ ],
  "pagination": {
    "page": 2,
    "limit": 50,
    "total_records": 1500,
    "filtered_records": 320,
    "total_pages": 7,
    "has_next": true,
    "has_prev": true
  },
  "sorting": { "field": "price", "order": "desc" },
  "time": 12
}
```

---

### POST — Insert Record

```json
{
  "url": "sarychdb://alice@secret123/products/post",
  "body": {
    "name": "Gaming Laptop",
    "price": 1299.99,
    "category": "electronics",
    "stock": 50
  }
}
```

**Response:**
```json
{
  "operation": "post",
  "database": "products",
  "message": "Record inserted successfully",
  "time": 8
}
```

---

### PUT — Update by Query or ID

#### Update by query (updates all matching records)
```json
{
  "url": "sarychdb://alice@secret123/products/put?query=Gaming+Laptop",
  "body": { "price": 999.99 }
}
```

#### Update by `_id` (updates exactly one record)
```json
{
  "url": "sarychdb://alice@secret123/products/put",
  "idUpdate": "550e8400-e29b-41d4-a716-446655440000",
  "body": { "stock": 25 }
}
```

---

### EDIT — Update by `_id` (body-driven)

```json
{
  "url": "sarychdb://alice@secret123/products/edit",
  "body": {
    "_id": "550e8400-e29b-41d4-a716-446655440000",
    "price": 899.99,
    "stock": 10
  }
}
```

---

### UPDATE_RECORDS — Explicit ID Update

```json
{
  "url": "sarychdb://alice@secret123/products/update_records",
  "body": {
    "idUpdate": "550e8400-e29b-41d4-a716-446655440000",
    "updateData": { "price": 750.00 }
  }
}
```

---

### DELETE — Delete by Query

```
sarychdb://alice@secret123/products/delete?query=discontinued
```

### DELETE_BY_ID — Delete one record

```json
{
  "url": "sarychdb://alice@secret123/products/delete_by_id",
  "body": { "_id": "550e8400-e29b-41d4-a716-446655440000" }
}
```

---

### STATS — Database Statistics

```json
{ "url": "sarychdb://alice@secret123/products/stats" }
```

**Response:**
```json
{
  "database": "products",
  "username": "alice",
  "total_records": 1500,
  "size_bytes": 524288,
  "read_time_ms": 2,
  "cached": true,
  "timestamp": "2026-03-27T10:30:00Z",
  "time": 3
}
```

`cached: true` means the data was served from the in-memory cache on this request.

---

## Caching

| Layer | Scope | TTL | Invalidation |
|-------|-------|-----|--------------|
| **DB cache** | Full database read | 5 min | Any write to that database |
| **Search cache** | Query result per database+query | 5 min | Any write to that database |

- First request reads from disk and populates both caches.
- Subsequent identical requests are ~10× faster.
- Any `post`, `put`, `edit`, `delete`, or `rename_db` automatically invalidates both caches.

---

## Operation Summary

| Operation | Auth | Description |
|-----------|------|-------------|
| `create_user` / `signup` | Self | Create account (+ optional DB) |
| `create_db` | User | Create a new database |
| `delete_db` | User | Delete a database |
| `rename_db` | User + DB | Rename a database |
| `list_dbs` | User | List databases |
| `all_dbs` | User | List databases with counts |
| `get` | User + DB | Full-text search |
| `browse` | User + DB | Paginate all records |
| `list` | User + DB | Filter + sort + paginate |
| `post` | User + DB | Insert a record |
| `put` | User + DB | Update by query or `_id` |
| `edit` | User + DB | Update by `_id` (body-driven) |
| `update_records` | User + DB | Explicit ID update |
| `delete` | User + DB | Delete by query |
| `delete_by_id` | User + DB | Delete by `_id` |
| `stats` | User + DB | Database statistics |
| `health` | None | Health check |

---

## Architecture

```
src/
├── main.rs              — CLI parsing, server/benchmark entry point
└── modules/
    ├── mod.rs
    ├── server.rs        — TCP listener, protocol parsing, request routing
    ├── database.rs      — CRUD operations, caching, pagination, filtering
    ├── search.rs        — Parallel/sequential search, search cache, node splitting
    └── auth.rs          — User management, bcrypt authentication
```

**Data directory layout:**
```
~/Documents/SarychDB/
├── users.json           — User registry (bcrypt hashed passwords)
└── users/
    └── <username>/
        ├── products.json
        └── orders.json
```

---

## Performance Notes

- Rayon automatically uses all available CPU cores for parallel search.
- For datasets under 1 000 records, sequential search is used to avoid threading overhead.
- `cargo run --release` gives significantly better search throughput than debug builds.
- Use `list` with specific `filters` to reduce dataset size before sorting.
- The 5-minute cache is ideal for dashboards and repeated read-heavy workloads.
