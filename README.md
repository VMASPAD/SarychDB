# SarychDB

<div align="center">
  <img src="SDB.svg" alt="SarychDB Logo" width="200"/>
</div>

## 📘 Uso rápido

SarychDB expone un protocolo TCP propio. El formato de URL es:

```
sarychdb://usuario@password/database/operacion?query=valor
```

También acepta mensajes JSON:

```json
{
  "url": "sarychdb://usuario@password/database/operacion",
  "op": "post",
  "body": { "campo": "valor" },
  "queryType": "key",
  "idUpdate": "...",
  "page": 1,
  "limit": 10,
  "sortBy": "name",
  "sortOrder": "asc",
  "filters": { "activo": true }
}
```

### 🚀 Iniciar servidor

```bash
cargo run
```

Puerto por defecto: `4040` (configurable con `--protocol-port` o `SARYCHDB_PROTOCOL_PORT`).

### 🔧 Operaciones principales

- **create_user** / **signup**
```
sarychdb://admin@pass/mi_db/create_user
```

- **create_db**
```
sarychdb://admin@pass/mi_db/create_db
```

- **delete_db**
```
sarychdb://admin@pass/mi_db/delete_db
```

- **get** (búsqueda)
```
sarychdb://admin@pass/mi_db/get?query=valor
```

- **post** (insertar)
```json
{ "url": "sarychdb://admin@pass/mi_db/post", "body": { "name": "Item" } }
```

- **put** (actualizar por query o id)
```json
{ "url": "sarychdb://admin@pass/mi_db/put?query=Item", "body": { "price": 10 } }
```

- **edit** (actualizar por _id)
```json
{ "url": "sarychdb://admin@pass/mi_db/edit", "body": { "_id": "...", "price": 12 } }
```

- **delete** (por query)
```
sarychdb://admin@pass/mi_db/delete?query=Item
```

- **delete_by_id** (por _id)
```json
{ "url": "sarychdb://admin@pass/mi_db/delete_by_id", "body": { "_id": "..." } }
```

- **list_dbs** / **all_dbs**
```
sarychdb://admin@pass/mi_db/list_dbs
```

- **stats** / **health**
```
sarychdb://admin@pass/mi_db/stats
```

### 📁 Ubicación de datos

Por defecto se guardan en:

```
~/Documents/SarychDB/
```

Puedes cambiarlo con la variable `SARYCHDB_DATA_DIR`.