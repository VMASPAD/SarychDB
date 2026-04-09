# SarychDB REST API

This document explains the HTTP and HTTPS interface for SarychDB.
The REST layer is a transport wrapper over the existing execution engine, so the same database logic is used whether the request arrives through TCP, HTTP, or HTTPS.

## Overview

When you start SarychDB with the REST mode flag, the server listens on a port and exposes JSON endpoints. Each endpoint converts the incoming request into the same internal protocol message used by the native SarychDB handler, then returns the resulting JSON response.

That design keeps the behavior consistent across transports:

- authentication rules stay the same
- search, pagination, and update semantics stay the same
- response timing is still reported in milliseconds
- cache and write invalidation behavior does not change

## Starting the REST API

### HTTP

```bash
cargo run -- --rest --port 4040
```

### HTTPS

```bash
cargo run -- --https --port 4040 --tls-cert cert.pem --tls-key key.pem
```

### Environment variables

- `SARYCHDB_HTTP_PORT` sets the REST API port
- `SARYCHDB_TLS_CERT` sets the TLS certificate path
- `SARYCHDB_TLS_KEY` sets the TLS private key path

If no REST-specific port is provided, the server falls back to `SARYCHDB_PROTOCOL_PORT`, then `PORT`, and finally `4040`.

## Authentication

Most endpoints require a username and password. Depending on the endpoint, credentials can be sent in the JSON body, query string, or request headers.

The REST layer forwards those values into the internal protocol format, so the same validation rules apply as in the TCP mode.

## Endpoint Reference

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/health` | Health check |
| `POST` | `/api/users` | Create a new user, optionally with a database |
| `POST` | `/api/databases` | Create a database for a user |
| `GET` | `/api/databases` | List a user's databases |
| `DELETE` | `/api/databases/{database}` | Delete a database |
| `PATCH` | `/api/databases/{database}` | Rename a database |
| `GET` | `/api/databases/{database}/stats` | Return database statistics |
| `GET` | `/api/databases/{database}/browse` | Paginate records without filters |
| `GET` | `/api/databases/{database}/list` | Filter, sort, and paginate records |
| `GET` | `/api/databases/{database}/records` | Full-text search records |
| `POST` | `/api/databases/{database}/records` | Insert a record |
| `PUT` | `/api/databases/{database}/records` | Update records |
| `DELETE` | `/api/databases/{database}/records` | Delete records |
| `GET` / `POST` | `/sarych` | Compatibility bridge for protocol-style requests |

## Common Response Shape

Successful responses are JSON objects and usually include a `time` field:

```json
{
  "operation": "get",
  "database": "products",
  "results": [],
  "count": 0,
  "time": 3
}
```

Errors also return JSON:

```json
{
  "error": "Database does not exist"
}
```

## Examples

### Create a user

```bash
curl -X POST http://localhost:4040/api/users \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "secret123"
  }'
```

### Create a database

```bash
curl -X POST http://localhost:4040/api/databases \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "secret123",
    "db_name": "products"
  }'
```

### Insert a record

```bash
curl -X POST http://localhost:4040/api/databases/products/records \
  -H "Content-Type: application/json" \
  -H "username: admin" \
  -H "password: secret123" \
  -d '{
    "name": "Gaming Laptop",
    "price": 1299.99,
    "category": "electronics"
  }'
```

### Search records

```bash
curl "http://localhost:4040/api/databases/products/records?username=admin&password=secret123&query=laptop"
```

### Browse records with pagination

```bash
curl "http://localhost:4040/api/databases/products/browse?username=admin&password=secret123&limit=20&page=2"
```

### Use HTTPS

```bash
curl -k "https://localhost:4040/health"
```

## How It Works Internally

1. The HTTP server receives a request on one of the REST endpoints.
2. The handler extracts credentials, path parameters, query parameters, and body payloads.
3. The handler converts those values into the same protocol message format used by the TCP server.
4. `handle_protocol_message` executes the operation.
5. The HTTP layer returns the JSON response and maps common validation failures to the appropriate HTTP status code.

This means the REST API does not duplicate database logic. It only changes the transport and request shape.

## Notes

- The REST server and TCP server are separate modes.
- HTTPS requires a certificate and private key in PEM format.
- The native protocol is still available for direct clients that already speak `sarychdb://`.
- The compatibility bridge at `/sarych` is useful if you want to migrate gradually from protocol-style requests to resource-based REST endpoints.
