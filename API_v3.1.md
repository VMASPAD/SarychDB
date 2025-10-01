# 🚀 SarychDB API v3.1 - Smart Caching & Flexible Pagination

## ⚠️ IMPORTANT: Data Format Requirements

### ✅ Correct Data Format
SarychDB stores data as an **array of objects**. Each record must be a JSON object:

```json
[
  {
    "id": 1,
    "name": "Product A",
    "price": 99.99
  },
  {
    "id": 2,
    "name": "Product B",
    "price": 149.99
  },
  {
    "id": 3,
    "name": "Product C",
    "price": 199.99
  }
]
```

**Key Points:**
- ✅ Top level is an array `[]`
- ✅ Each element is an object `{}`
- ✅ Objects can have any fields
- ✅ All query operations work correctly

---

### ❌ Incorrect Data Format
Do **NOT** mix arrays and objects at the same level:

```json
[
  [],           // ❌ Empty array - will cause issues
  [1, 2, 3],    // ❌ Nested array - queries won't work
  "string",     // ❌ Plain string - not searchable
  {},           // ✅ Only this is correct
  123           // ❌ Plain number - not searchable
]
```

**Why This Matters:**
- ❌ Search operations won't find data in nested arrays
- ❌ Filtering won't work on non-object elements
- ❌ Sorting requires object fields
- ❌ Pagination may return unexpected results
- ❌ Update/Delete operations may fail

---

### 📝 Inserting Data (POST)

#### Correct Way
```bash
curl -X POST "http://localhost:3030/sarych?url=sarychdb://admin@pass/products/post" \
  -H "username: admin" \
  -H "password: pass" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Gaming Laptop",
    "price": 1299.99,
    "category": "electronics",
    "stock": 50
  }'
```

**Result in database:**
```json
[
  {
    "_id": "550e8400-e29b-41d4-a716-446655440000",
    "_created_at": "2025-10-01T10:30:00Z",
    "name": "Gaming Laptop",
    "price": 1299.99,
    "category": "electronics",
    "stock": 50
  }
]
```

#### Incorrect Way (Don't Do This)
```bash
# ❌ Sending an array instead of an object
curl -X POST "..." \
  -d '["item1", "item2", "item3"]'

# ❌ Sending a plain value
curl -X POST "..." \
  -d '"just a string"'

# ❌ Sending nested structures incorrectly
curl -X POST "..." \
  -d '{
    "items": [1, 2, 3],
    "nested": [[]]
  }'
```

---

### 💡 Best Practices

1. **Always Use Objects**: Each record should be a JSON object with key-value pairs
2. **Flat Structure**: Keep data as flat as possible for optimal querying
3. **Consistent Schema**: Use similar field names across records for better filtering
4. **Avoid Deep Nesting**: Limit nested objects to 2-3 levels maximum
5. **Use Arrays for Tags**: Arrays are fine as field values: `{"tags": ["new", "sale"]}`

---

### ✅ Valid Field Value Types

```json
{
  "string_field": "text value",           // ✅ Strings
  "number_field": 42,                     // ✅ Numbers
  "boolean_field": true,                  // ✅ Booleans
  "null_field": null,                     // ✅ Null values
  "array_field": ["tag1", "tag2"],        // ✅ Arrays as values
  "nested_object": {                      // ✅ Nested objects
    "sub_field": "value"
  }
}
```

**Remember:** The top level must always be an array of objects like this!

---

## 🌟 What's New in v3.1

### ✅ Smart Caching System (NEW!)
- Automatic 5-minute cache for all read operations
- ~10x faster repeated queries
- Auto-invalidation on data changes
- Zero configuration required

### ✅ Flexible Limit Behavior (UPDATED!)
- **Limit only**: Get first N records instantly
- **Limit + Page**: Traditional pagination
- **No parameters**: Smart default (first 10 records)

### ✅ Enhanced Performance
- Cached reads with intelligent TTL
- Efficient pagination without overhead
- Optimized filter→sort→paginate pipeline

---

## 📘 BROWSE Operation - Flexible Data Retrieval

### URL Format
```
GET /sarych?url=sarychdb://user@pass/database/browse
```

### Headers
- **Required**: `username`, `password`
- **Optional**: `limit`, `page`

### 🎯 Usage Modes

#### 1️⃣ Limit Only (No Pagination)
Get first N records. Perfect for "load more" or quick previews.

```bash
curl "http://localhost:3030/sarych?url=sarychdb://admin@pass/products/browse" \
  -H "username: admin" \
  -H "password: pass" \
  -H "limit: 200"
```

**Response:**
```json
{
  "operation": "browse",
  "database": "products",
  "data": [/* 200 records */],
  "pagination": {
    "limit": 200,
    "returned": 200,
    "total_records": 1500,
    "mode": "limit_only"
  },
  "time": 5
}
```

**Use cases:**
- Initial data load
- "Show more" buttons
- Quick data previews
- Export first N records

---

#### 2️⃣ Limit + Page (Paginated)
Traditional pagination with page navigation.

```bash
curl "http://localhost:3030/sarych?url=sarychdb://admin@pass/products/browse" \
  -H "username: admin" \
  -H "password: pass" \
  -H "page: 4" \
  -H "limit: 200"
```

**Response:**
```json
{
  "operation": "browse",
  "database": "products",
  "data": [/* records 601-800 */],
  "pagination": {
    "page": 4,
    "limit": 200,
    "returned": 200,
    "total_records": 1500,
    "total_pages": 8,
    "has_next": true,
    "has_prev": true,
    "mode": "paginated"
  },
  "time": 3
}
```

**Pagination math:**
- Page 1: records 1-200
- Page 2: records 201-400
- Page 3: records 401-600
- Page 4: records 601-800
- ...
- Page 8: records 1401-1500 (only 100 records)

**Use cases:**
- Data tables with page numbers
- Classic pagination UI
- Navigate through large datasets
- "Previous/Next" navigation

---

#### 3️⃣ Default (No Parameters)
No configuration needed! Get first 10 records with pagination info.

```bash
curl "http://localhost:3030/sarych?url=sarychdb://admin@pass/products/browse" \
  -H "username: admin" \
  -H "password: pass"
```

**Response:**
```json
{
  "operation": "browse",
  "database": "products",
  "data": [/* 10 records */],
  "pagination": {
    "page": 1,
    "limit": 10,
    "returned": 10,
    "total_records": 1500,
    "total_pages": 150,
    "has_next": true,
    "has_prev": false,
    "mode": "default"
  },
  "time": 8
}
```

---

### ❌ Error: Page Without Limit

```bash
curl "http://localhost:3030/sarych?url=sarychdb://admin@pass/products/browse" \
  -H "username: admin" \
  -H "password: pass" \
  -H "page: 5"
```

**Response:**
```json
{
  "error": "Cannot use 'page' without 'limit'. Please provide both parameters.",
  "time": 2
}
```

**Why?** The system needs to know the page size to calculate offsets. Always provide `limit` when using `page`.

---

## 📋 LIST Operation - Advanced Queries

### URL Format
```
GET /sarych?url=sarychdb://user@pass/database/list
```

### Headers
- **Required**: `username`, `password`
- **Optional**: `limit`, `page`, `sortBy`, `sortOrder`, `filters`

### Examples

#### Get First 200 Records (Limit Only)
```bash
curl "http://localhost:3030/sarych?url=sarychdb://admin@pass/products/list" \
  -H "username: admin" \
  -H "password: pass" \
  -H "limit: 200"
```

#### Paginated with Sorting
```bash
curl "http://localhost:3030/sarych?url=sarychdb://admin@pass/products/list" \
  -H "username: admin" \
  -H "password: pass" \
  -H "page: 4" \
  -H "limit: 200" \
  -H "sortBy: price" \
  -H "sortOrder: desc"
```

#### Filter + Sort + Limit
```bash
curl "http://localhost:3030/sarych?url=sarychdb://admin@pass/products/list" \
  -H "username: admin" \
  -H "password: pass" \
  -H "limit: 100" \
  -H "sortBy: name" \
  -H "sortOrder: asc" \
  -H 'filters: {"category":"electronics","active":true}'
```

#### Filter with Multiple Values (OR logic)
```bash
curl "http://localhost:3030/sarych?url=sarychdb://admin@pass/products/list" \
  -H "username: admin" \
  -H "password: pass" \
  -H "page: 2" \
  -H "limit: 50" \
  -H 'filters: {"category":["electronics","accessories"]}'
```

---

## 🎯 JavaScript Client Examples

### BROWSE Examples

```javascript
const SarychDB = require('sarychdb');
const client = new SarychDB({
  username: 'admin',
  password: 'admin123',
  baseUrl: 'http://localhost:3030'
});

// 1. Get first 200 records (limit only)
const result1 = await client.browse('products', { limit: 200 });
console.log(result1.pagination.mode); // "limit_only"
console.log(result1.data.length); // 200

// 2. Page 4 with 200 per page
const result2 = await client.browse('products', { page: 4, limit: 200 });
console.log(result2.pagination.mode); // "paginated"
console.log(result2.pagination.page); // 4
console.log(result2.pagination.total_pages); // 8 (if 1500 total records)

// 3. Default behavior
const result3 = await client.browse('products');
console.log(result3.pagination.mode); // "default"
console.log(result3.data.length); // 10
```

### LIST Examples

```javascript
// Get first 200 records, sorted by price
const result1 = await client.list('products', {
  limit: 200,
  sortBy: 'price',
  sortOrder: 'desc'
});

// Paginated with filters
const result2 = await client.list('products', {
  page: 4,
  limit: 200,
  filters: { category: 'electronics', active: true },
  sortBy: 'name'
});

// Filter with multiple values
const result3 = await client.list('products', {
  limit: 100,
  filters: { category: ['electronics', 'accessories'] }
});
```

---

## 🔄 Caching Details

### How It Works
- **First Request**: Reads from disk, stores in cache (5-minute TTL)
- **Subsequent Requests**: Served from cache (~10x faster)
- **After 5 Minutes**: Cache expires, next read refreshes cache
- **On Write Operations**: Cache invalidated immediately

### Cache Behavior

```bash
# First request (cache miss)
curl "..." -H "limit: 100"
# Response time: 50ms

# Second request (cache hit)
curl "..." -H "limit: 100"
# Response time: 5ms (~10x faster!)

# After data update (cache invalidated)
curl "..." -X POST -d '{...}'
# Next read will refresh cache
```

### Stats Endpoint
```bash
curl "http://localhost:3030/sarych?url=sarychdb://admin@pass/products/stats" \
  -H "username: admin" \
  -H "password: pass"
```

**Response:**
```json
{
  "database": "products",
  "username": "admin",
  "total_records": 1500,
  "size_bytes": 524288,
  "read_time_ms": 5,
  "cached": true,
  "timestamp": "2025-10-01T10:30:00Z",
  "time": 6
}
```

---

## 📊 Response Structure

### BROWSE Response
```json
{
  "operation": "browse",
  "database": "database_name",
  "data": [/* array of records */],
  "pagination": {
    "page": 4,              // Current page (paginated mode only)
    "limit": 200,           // Records per page / total limit
    "returned": 200,        // Actual records returned
    "total_records": 1500,  // Total records in database
    "total_pages": 8,       // Total pages (paginated mode only)
    "has_next": true,       // More pages available (paginated mode only)
    "has_prev": true,       // Previous pages available (paginated mode only)
    "mode": "paginated"     // Mode: "limit_only", "paginated", or "default"
  },
  "time": 3
}
```

### LIST Response
```json
{
  "operation": "list",
  "database": "database_name",
  "data": [/* filtered and sorted records */],
  "pagination": {
    "page": 1,
    "limit": 100,
    "returned": 100,
    "total_records": 1500,
    "filtered_records": 450,
    "total_pages": 5,
    "has_next": true,
    "has_prev": false
  },
  "sorting": {
    "field": "price",
    "order": "desc"
  },
  "time": 15
}
```

---

## 🆚 Operation Comparison

| Feature | GET | BROWSE | LIST |
|---------|-----|--------|------|
| **Purpose** | Search | Simple pagination | Advanced queries |
| **Caching** | ✅ Yes | ✅ Yes | ✅ Yes |
| **Pagination** | ❌ No | ✅ Flexible | ✅ Yes |
| **Sorting** | ❌ No | ❌ No | ✅ Yes |
| **Filtering** | Search only | ❌ No | ✅ Yes |
| **Best For** | Text search | Quick browsing | Complex queries |

---

## 💡 Best Practices

### 1. Choose the Right Operation
- **GET**: Searching for specific text across all fields
- **BROWSE**: Simple pagination without filters
- **LIST**: Complex queries with filters and sorting

### 2. Use Appropriate Limits
- **Small datasets (<100)**: Use default or limit-only mode
- **Medium datasets (100-1000)**: Use limit: 50-100
- **Large datasets (1000+)**: Use pagination with limit: 100-200

### 3. Leverage Caching
- Repeated queries are ~10x faster with cache
- Cache stays valid for 5 minutes
- Perfect for dashboards and reports

### 4. Optimize Filters
- Apply filters in LIST to reduce dataset before pagination
- Use specific filters to improve performance
- Combine filters with sorting for best results

---

## 🚀 Performance Tips

1. **Use Cache Wisely**: Repeated queries benefit from 5-minute cache
2. **Reasonable Limits**: 100-200 records per page is optimal
3. **Filter First**: Reduce dataset with filters before sorting
4. **Avoid Large Pages**: Don't request 1000+ records at once
5. **Plan Pagination**: Calculate total_pages to show proper UI

---

## 🎉 Summary

### Key Features
✅ **Smart Caching**: 5-minute automatic cache for read operations  
✅ **Flexible Limits**: Use alone or with pagination  
✅ **Three Modes**: limit_only, paginated, default  
✅ **Zero Config**: Works with sensible defaults  
✅ **Fast Performance**: Cached reads are ~10x faster  
✅ **Clear Responses**: Mode indicator shows operation type  

### Migration from v3.0
- **Removed**: `offset` and `last` headers (simplified!)
- **Added**: Flexible limit behavior (limit-only mode)
- **Added**: Smart caching system (automatic)
- **Added**: Mode indicator in responses
- **Improved**: Performance with caching

---

**SarychDB v3.1** - Smarter, faster, simpler! 🚀
