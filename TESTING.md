# 🚀 Quick Testing Guide - SarychDB

## ✅ Server Status
The server is running at http://localhost:3030

## 📋 Steps to test the system:

### 1. Create a user
```bash
curl -X POST http://localhost:3030/api/users \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "password123"
  }'
```

### 2. Create a database
```bash
curl -X POST http://localhost:3030/api/databases \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "password123", 
    "db_name": "products"
  }'
```

### 3. Insert some test data
```bash
# Product 1
curl -X POST "http://localhost:3030/sarych?url=sarychdb://testuser@password123/products/post" \
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

# Product 2  
curl -X POST "http://localhost:3030/sarych?url=sarychdb://testuser@password123/products/post" \
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

# Product 3
curl -X POST "http://localhost:3030/sarych?url=sarychdb://testuser@password123/products/post" \
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

### 4. Test searches
```bash
# Search all products
curl "http://localhost:3030/sarych?url=sarychdb://testuser@password123/products/get"

# Search products by brand TechPro
curl "http://localhost:3030/sarych?url=sarychdb://testuser@password123/products/get?query=TechPro"

# Search products with code P1605
curl "http://localhost:3030/sarych?url=sarychdb://testuser@password123/products/get?query=P1605"

# Search accessories
curl "http://localhost:3030/sarych?url=sarychdb://testuser@password123/products/get?query=accessories"
```

### 5. Update data
```bash
# Update stock for product P1605
curl -X PUT "http://localhost:3030/sarych?url=sarychdb://testuser@password123/products/put?query=P1605" \
  -H "Content-Type: application/json" \
  -d '{
    "stock": 10,
    "price": 1199.99
  }'
```

### 6. View statistics
```bash
curl "http://localhost:3030/sarych?url=sarychdb://testuser@password123/products/stats"
```

### 7. Delete inactive products
```bash
curl -X DELETE "http://localhost:3030/sarych?url=sarychdb://testuser@password123/products/delete?query=false"
```

## 🔍 Search Engine Features

- ✅ **Universal search**: Automatically searches all JSON fields
- ✅ **Parallel search**: Uses multiple cores for better performance  
- ✅ **Recursive search**: Finds values in nested objects and arrays
- ✅ **Multiple types**: Searches strings, numbers, booleans
- ✅ **Custom protocol**: sarychdb:// for native operations

## 📁 Generated Files

After testing, you should see these files:
- `users.json` - Contains created users
- `users/testuser/` - User-specific folder
- `users/testuser/products.json` - Contains inserted product data

## 🎯 Next Steps

1. Test with larger JSON files (like the mentioned 500MB.json)
2. Run benchmarks with `cargo run benchmark`
3. Implement more features according to project needs