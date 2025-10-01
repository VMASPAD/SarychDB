# 🚀 SarychDB API v3.0 - Advanced Query System

## 🌟 What's New in v3.0

### ✅ Pagination System
Efficiently retrieve large datasets with configurable page size and navigation.

### ✅ Flexible Sorting
Sort results by any field in ascending or descending order.

### ✅ Generic Filtering
Apply multiple filters dynamically based on your data structure.

### ✅ Performance Optimized
Filter first, then sort, then paginate for maximum efficiency.

---

## � BROWSE Operation - Simple Pagination

The `browse` operation provides a straightforward way to paginate through all records in a database without filtering or sorting.

### Base URL Format
```
GET /sarych?url=/database/browse
```

### Headers

#### Required Headers
- `username`: Your username
- `password`: Your password

#### Optional Headers
- `page`: Page number (default: `1`)
- `limit`: Records per page (default: `10`)

### Examples

#### Basic Browse - First Page
```bash
curl "http://localhost:3030/sarych?url=/products/browse" \
  -H "username: testuser" \
  -H "password: password123"
```

**Response:**
```json
{
  "operation": "browse",
  "database": "products",
  "data": [
    {
      "_id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Gaming Laptop",
      "price": 1299.99,
      "_created_at": "2025-09-30T10:30:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 10,
    "total_records": 150,
    "total_pages": 15,
    "has_next": true,
    "has_prev": false
  },
  "time": 25
}
```

#### Custom Page Size
```bash
curl "http://localhost:3030/sarych?url=/products/browse" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "limit: 50"
```

#### Navigate to Specific Page
```bash
curl "http://localhost:3030/sarych?url=/products/browse" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "page: 3" \
  -H "limit: 25"
```

---

## �📋 LIST Operation - Advanced Query Endpoint

The new `list` operation provides a powerful way to query databases with pagination, sorting, and filtering capabilities.

### Base URL Format
```
GET /sarych?url=/database/list
```

### Headers

#### Required Headers
- `username`: Your username
- `password`: Your password

#### Optional Headers
- `page`: Page number (default: `1`)
- `limit`: Records per page (default: `10`)
- `sortBy`: Field name to sort by
- `sortOrder`: Sort direction - `asc` or `desc` (default: `asc`)
- `filters`: JSON string with filter criteria

---

## 🔍 Examples

### 1. Basic Pagination

Get first 10 records (page 1):
```bash
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: testuser" \
  -H "password: password123"
```

**Response:**
```json
{
  "operation": "list",
  "database": "products",
  "data": [
    {
      "_id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Gaming Laptop",
      "price": 1299.99,
      "category": "electronics",
      "_created_at": "2025-09-30T10:30:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 10,
    "total_records": 150,
    "filtered_records": 150,
    "total_pages": 15,
    "has_next": true,
    "has_prev": false
  },
  "sorting": {
    "field": null,
    "order": "asc"
  },
  "time": 45
}
```

### 2. Custom Page Size

Get 20 records per page:
```bash
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "limit: 20"
```

### 3. Navigate Pages

Get page 3 with 50 records per page:
```bash
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "page: 3" \
  -H "limit: 50"
```

### 4. Sort Results

Sort by price in descending order:
```bash
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "sortBy: price" \
  -H "sortOrder: desc"
```

Sort by name alphabetically:
```bash
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "sortBy: name" \
  -H "sortOrder: asc"
```

### 5. Apply Filters

Filter by exact match (single value):
```bash
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: testuser" \
  -H "password: password123" \
  -H 'filters: {"category":"electronics","active":true}'
```

Filter with multiple possible values (OR logic):
```bash
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: testuser" \
  -H "password: password123" \
  -H 'filters: {"category":["electronics","accessories"]}'
```

### 6. Combine Everything

Paginated, sorted, and filtered query:
```bash
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "page: 2" \
  -H "limit: 25" \
  -H "sortBy: price" \
  -H "sortOrder: desc" \
  -H 'filters: {"category":"electronics","active":true}'
```

This returns:
- Only electronics products that are active
- Sorted by price (highest first)
- Page 2 with 25 results per page

---

## 🎯 Filter Syntax

### Exact Match
Match a specific value for a field:
```json
{
  "category": "electronics",
  "active": true,
  "stock": 10
}
```

### Multiple Values (OR Logic)
Match any of several values:
```json
{
  "category": ["electronics", "accessories", "computers"],
  "brand": ["TechPro", "KeyMaster"]
}
```

### Mixed Filters
Combine exact matches with OR conditions:
```json
{
  "category": ["electronics", "accessories"],
  "active": true,
  "price": 29.99
}
```

This matches records where:
- `category` is either "electronics" OR "accessories"
- AND `active` is true
- AND `price` is exactly 29.99

---

## 📊 Response Structure

### Success Response
```json
{
  "operation": "list",
  "database": "database_name",
  "data": [...],
  "pagination": {
    "page": 1,
    "limit": 10,
    "total_records": 150,
    "filtered_records": 75,
    "total_pages": 8,
    "has_next": true,
    "has_prev": false
  },
  "sorting": {
    "field": "price",
    "order": "desc"
  },
  "time": 45
}
```

### Pagination Object
- `page`: Current page number
- `limit`: Records per page
- `total_records`: Total records in database (before filtering)
- `filtered_records`: Total records after applying filters
- `total_pages`: Total pages available with current limit
- `has_next`: Boolean - more pages available
- `has_prev`: Boolean - previous pages available

### Sorting Object
- `field`: Field name used for sorting (null if not sorted)
- `order`: Sort direction (`asc` or `desc`)

---

## 🔄 Comparison: GET vs BROWSE vs LIST

### GET Operation (v2.0)
- Best for: Searching with queries
- Returns: All matching records
- Use when: You need to search text across all fields

```bash
curl "http://localhost:3030/sarych?url=/products/get?query=laptop" \
  -H "username: testuser" \
  -H "password: password123"
```

### BROWSE Operation (v3.0 - NEW!)
- Best for: Simple paginated browsing
- Returns: Paginated records in original order
- Use when: You just need to paginate through all records without filtering or sorting

```bash
curl "http://localhost:3030/sarych?url=/products/browse" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "page: 1" \
  -H "limit: 20"
```

### LIST Operation (v3.0)
- Best for: Advanced queries with filters and sorting
- Returns: Paginated, sorted, filtered results
- Use when: You need to display data in tables with complex requirements

```bash
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: testuser" \
  -H "password: password123" \
  -H "page: 1" \
  -H "limit: 20" \
  -H "sortBy: price"
```

---

## 💡 Use Cases

### 1. Simple Pagination (BROWSE)
```bash
# Just paginate through all products
curl "http://localhost:3030/sarych?url=/products/browse" \
  -H "username: admin" \
  -H "password: admin123" \
  -H "page: 1" \
  -H "limit: 20"
```

### 2. Display Products Table (LIST)
```bash
# Page 1, 20 items, sorted by name
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: admin" \
  -H "password: admin123" \
  -H "page: 1" \
  -H "limit: 20" \
  -H "sortBy: name" \
  -H "sortOrder: asc"
```

### 3. Filter Active Electronics (LIST)
```bash
# Only active electronics, sorted by price
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: admin" \
  -H "password: admin123" \
  -H 'filters: {"category":"electronics","active":true}' \
  -H "sortBy: price" \
  -H "sortOrder: asc"
```

### 4. Multi-Category Filter (LIST)
```bash
# Electronics OR accessories, sorted by creation date
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: admin" \
  -H "password: admin123" \
  -H 'filters: {"category":["electronics","accessories"]}' \
  -H "sortBy: _created_at" \
  -H "sortOrder: desc"
```

### 5. Inventory Management (LIST)
```bash
# Low stock items (stock < 10), sorted by stock level
curl "http://localhost:3030/sarych?url=/inventory/list" \
  -H "username: admin" \
  -H "password: admin123" \
  -H "sortBy: stock" \
  -H "sortOrder: asc" \
  -H "limit: 50"
```

---

## ⚡ Performance Tips

1. **Use Filters First**: Apply filters to reduce dataset before sorting
2. **Reasonable Limits**: Keep page size between 10-100 for best performance
3. **Index Common Sorts**: Frequently sorted fields benefit from consistent naming
4. **Cache Results**: Cache paginated results in your frontend when possible

---

## 🚨 Error Handling

### Invalid Page Number
If page exceeds total pages, you get empty results:
```json
{
  "data": [],
  "pagination": {
    "page": 999,
    "limit": 10,
    "total_pages": 15,
    "has_next": false,
    "has_prev": true
  }
}
```

### Invalid Filter JSON
```json
{
  "error": "Invalid filter format",
  "time": 5
}
```

### Database Not Found
```json
{
  "error": "Database does not exist",
  "time": 3
}
```

---

## 📚 Frontend Integration Examples

### React with Pagination
```javascript
const [page, setPage] = useState(1);
const [products, setProducts] = useState([]);

const fetchProducts = async () => {
  const response = await fetch('http://localhost:3030/sarych?url=/products/list', {
    headers: {
      'username': 'admin',
      'password': 'admin123',
      'page': page.toString(),
      'limit': '20',
      'sortBy': 'name',
      'sortOrder': 'asc'
    }
  });
  const data = await response.json();
  setProducts(data.data);
};
```

### Vue with Filters
```javascript
const filters = ref({
  category: 'electronics',
  active: true
});

const fetchFiltered = async () => {
  const response = await fetch('http://localhost:3030/sarych?url=/products/list', {
    headers: {
      'username': 'admin',
      'password': 'admin123',
      'filters': JSON.stringify(filters.value)
    }
  });
  return await response.json();
};
```

---

## 🔐 Security Notes

- All `list` operations require authentication
- Filters are applied server-side for security
- Users can only access their own databases
- Invalid filter syntax is rejected safely

---

## 🆚 Migration from v2.0

### Before (v2.0 - GET all records)
```bash
curl "http://localhost:3030/sarych?url=/products/get" \
  -H "username: admin" \
  -H "password: admin123"
# Returns ALL records (could be thousands)
```

### After (v3.0 - BROWSE with pagination)
```bash
curl "http://localhost:3030/sarych?url=/products/browse" \
  -H "username: admin" \
  -H "password: admin123" \
  -H "limit: 20"
# Returns only 20 records per page - Simple and fast!
```

### Advanced (v3.0 - LIST with everything)
```bash
curl "http://localhost:3030/sarych?url=/products/list" \
  -H "username: admin" \
  -H "password: admin123" \
  -H "limit: 20" \
  -H "sortBy: price" \
  -H 'filters: {"active":true}'
# Filtered, sorted, and paginated - Full control!
```

---

## 📝 Summary

### Operations Overview

| Operation | Purpose | Pagination | Sorting | Filtering | Use Case |
|-----------|---------|------------|---------|-----------|----------|
| **GET** | Search by query | ❌ | ❌ | ❌ | Text search across all fields |
| **BROWSE** | Simple pagination | ✅ | ❌ | ❌ | Quick browsing through all records |
| **LIST** | Advanced queries | ✅ | ✅ | ✅ | Complex data display with filters |

### Headers Reference

| Feature | Header | Values | Example |
|---------|--------|--------|---------|
| **Pagination** | `page` | Number (1+) | `page: 2` |
| | `limit` | Number (1-1000) | `limit: 50` |
| **Sorting** | `sortBy` | Field name | `sortBy: price` |
| | `sortOrder` | `asc` or `desc` | `sortOrder: desc` |
| **Filtering** | `filters` | JSON object | `filters: {"active":true}` |

---

## 🎉 Benefits

✅ **BROWSE**: Simple pagination - perfect for quick data browsing  
✅ **LIST**: Advanced queries - full control with filters and sorting  
✅ **Efficient**: Only load what you need  
✅ **Flexible**: Works with any data structure  
✅ **Fast**: Filter → Sort → Paginate optimization  
✅ **User-Friendly**: Easy frontend integration  
✅ **Scalable**: Handle thousands of records smoothly

---

**SarychDB v3.0** - Making data management simple and powerful! 🚀
