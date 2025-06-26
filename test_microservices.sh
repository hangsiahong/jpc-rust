#!/bin/bash

# Test scripts for both User Service and Product Service APIs

GATEWAY_URL="http://localhost:8082"
USER_SERVICE_URL="http://localhost:8080"
PRODUCT_SERVICE_URL="http://localhost:8081"

echo "🧪 Testing Multi-Service Architecture"
echo "====================================="
echo ""
echo "Gateway URL: $GATEWAY_URL"
echo "User Service URL: $USER_SERVICE_URL"
echo "Product Service URL: $PRODUCT_SERVICE_URL"
echo ""

# Test User Service through Gateway
echo "🏢 Testing User Service through Gateway"
echo "======================================="

echo ""
echo "1. Testing user service health endpoint..."
curl -X POST "$GATEWAY_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "health",
    "id": 1
  }' | jq .

echo ""
echo "2. Creating a user..."
curl -X POST "$GATEWAY_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "create_user",
    "params": {
      "request": {
        "name": "John Doe",
        "email": "john.doe@example.com"
      }
    },
    "id": 2
  }' | jq .

echo ""
echo "3. Listing users..."
curl -X POST "$GATEWAY_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "list_users",
    "id": 3
  }' | jq .

# Test Product Service through Gateway
echo ""
echo "🛍️  Testing Product Service through Gateway"
echo "==========================================="

echo ""
echo "4. Testing product service health..."
curl -X POST "$PRODUCT_SERVICE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "health",
    "id": 4
  }' | jq .

echo ""
echo "5. Creating a product..."
curl -X POST "$PRODUCT_SERVICE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "create_product",
    "params": {
      "request": {
        "name": "Laptop",
        "description": "High-performance laptop for developers",
        "price": 1299.99,
        "category": "Electronics",
        "stock_quantity": 50
      }
    },
    "id": 5
  }' | jq .

echo ""
echo "6. Creating another product..."
curl -X POST "$PRODUCT_SERVICE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "create_product",
    "params": {
      "request": {
        "name": "Wireless Mouse",
        "description": "Ergonomic wireless mouse",
        "price": 29.99,
        "category": "Electronics",
        "stock_quantity": 100
      }
    },
    "id": 6
  }' | jq .

echo ""
echo "7. Listing all products..."
curl -X POST "$PRODUCT_SERVICE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "list_products",
    "id": 7
  }' | jq .

echo ""
echo "8. Getting products by category..."
curl -X POST "$PRODUCT_SERVICE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "get_products_by_category",
    "params": {
      "request": {
        "category": "Electronics"
      }
    },
    "id": 8
  }' | jq .

# Test Direct Service Access
echo ""
echo "🔄 Testing Direct Service Access (Bypassing Gateway)"
echo "===================================================="

echo ""
echo "9. Testing user service directly..."
curl -X POST "$USER_SERVICE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "health",
    "id": 9
  }' | jq .

echo ""
echo "10. Testing product service directly..."
curl -X POST "$PRODUCT_SERVICE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "health",
    "id": 10
  }' | jq .

echo ""
echo "✅ API Testing Complete!"
echo ""
echo "Services Summary:"
echo "- User Service: Running on port 8080"
echo "- Product Service: Running on port 8081"
echo "- Gateway: Running on port 8082 (routes to both services)"
echo ""
echo "You can now test routing through the gateway by sending requests to:"
echo "- User requests to: $GATEWAY_URL (will route to user service)"
echo "- Product requests to: $GATEWAY_URL (will route to product service based on path/content)"
