#!/bin/bash

# Test scripts for the User Service API

BASE_URL="http://localhost:8081"

echo "🧪 Testing User Service API through Pingora Gateway"
echo "=================================================="

# Test health endpoint
echo ""
echo "1. Testing health endpoint..."
curl -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "health",
    "id": 1
  }' | jq .

echo ""
echo "2. Creating a user..."
curl -X POST "$BASE_URL" \
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
echo "3. Creating another user..."
USER2_RESPONSE=$(curl -s -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "create_user",
    "params": {
      "request": {
        "name": "Jane Smith",
        "email": "jane.smith@example.com"
      }
    },
    "id": 3
  }')

echo "$USER2_RESPONSE" | jq .

# Extract user ID from response
USER_ID=$(echo "$USER2_RESPONSE" | jq -r '.result.id')

echo ""
echo "4. Getting user by ID: $USER_ID"
curl -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -d "{
    \"jsonrpc\": \"2.0\",
    \"method\": \"get_user\",
    \"params\": {
      \"request\": {
        \"id\": \"$USER_ID\"
      }
    },
    \"id\": 4
  }" | jq .

echo ""
echo "5. Listing all users..."
curl -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "list_users",
    "id": 5
  }' | jq .

echo ""
echo "6. Testing error case - invalid email..."
curl -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "create_user",
    "params": {
      "request": {
        "name": "Bad Email User",
        "email": "invalid-email"
      }
    },
    "id": 6
  }' | jq .

echo ""
echo "7. Testing error case - duplicate email..."
curl -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "create_user",
    "params": {
      "request": {
        "name": "Duplicate User",
        "email": "john.doe@example.com"
      }
    },
    "id": 7
  }' | jq .

echo ""
echo "✅ API tests completed!"
