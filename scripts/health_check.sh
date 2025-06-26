#!/bin/bash
# Enhanced health check script

echo "🔍 Comprehensive Health Check"
echo "============================"

check_service() {
    local name=$1
    local url=$2
    local expected=$3
    
    echo -n "Checking $name... "
    response=$(curl -s -w "%{http_code}" -o /tmp/health_response "$url" 2>/dev/null)
    http_code=${response: -3}
    
    if [ "$http_code" = "200" ]; then
        if grep -q "$expected" /tmp/health_response 2>/dev/null; then
            echo "✅ Healthy"
            return 0
        else
            echo "⚠️ Responding but unexpected content"
            return 1
        fi
    else
        echo "❌ Down (HTTP $http_code)"
        return 1
    fi
}

# Gateway health
if check_service "Gateway" "http://localhost:8082" "User Service"; then
    GATEWAY_OK=1
else
    GATEWAY_OK=0
fi

# User service health
USER_HEALTH='{"jsonrpc":"2.0","method":"health","id":1}'
if check_service "User Service" \
   "http://localhost:8080" \
   "User Service is healthy"; then
    USER_OK=1
else
    USER_OK=0
fi

# Product service health  
PRODUCT_HEALTH='{"jsonrpc":"2.0","method":"health","id":1}'
if check_service "Product Service" \
   "http://localhost:8081" \
   "Product Service is healthy"; then
    PRODUCT_OK=1
else
    PRODUCT_OK=0
fi

echo ""
echo "📊 System Status:"
echo "================"
[ $GATEWAY_OK -eq 1 ] && echo "Gateway: ✅ Running" || echo "Gateway: ❌ Down"
[ $USER_OK -eq 1 ] && echo "User Service: ✅ Running" || echo "User Service: ❌ Down"  
[ $PRODUCT_OK -eq 1 ] && echo "Product Service: ✅ Running" || echo "Product Service: ❌ Down"

total_healthy=$((GATEWAY_OK + USER_OK + PRODUCT_OK))
echo ""
echo "Health Score: $total_healthy/3"

if [ $total_healthy -eq 3 ]; then
    echo "🎉 All systems operational!"
    exit 0
else
    echo "⚠️ Some services are down"
    exit 1
fi
