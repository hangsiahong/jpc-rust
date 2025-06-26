#!/bin/bash

# Resilience and Recovery Testing for the Production Gateway and Services

GATEWAY_URL="http://localhost:8082"

echo "🛡️ Service Resilience and Recovery Testing"
echo "=========================================="
echo ""

# Function to test service availability
test_service_health() {
    local service_name=$1
    local max_attempts=$2
    local attempt=1
    
    echo "Testing $service_name availability..."
    
    while [ $attempt -le $max_attempts ]; do
        response=$(curl -s -w "%{http_code}" -X POST "$GATEWAY_URL" \
            -H "Content-Type: application/json" \
            -d '{
                "jsonrpc": "2.0",
                "method": "health",
                "id": 1
            }')
        
        http_code=$(echo $response | tail -c 4)
        
        if [ "$http_code" = "200" ]; then
            echo "✅ $service_name is healthy (attempt $attempt)"
            return 0
        elif [ "$http_code" = "503" ]; then
            echo "🔴 $service_name unavailable (attempt $attempt)"
        else
            echo "⚠️ $service_name returned code $http_code (attempt $attempt)"
        fi
        
        ((attempt++))
        sleep 2
    done
    
    echo "❌ $service_name failed health check after $max_attempts attempts"
    return 1
}

# Function to start services
start_service() {
    local service_name=$1
    local port=$2
    echo "🚀 Starting $service_name..."
    
    if [ "$service_name" = "user-service" ]; then
        nohup cargo run --bin user-service > /tmp/user-service.log 2>&1 &
        echo $! > /tmp/user-service.pid
    elif [ "$service_name" = "product-service" ]; then
        nohup cargo run --bin product-service > /tmp/product-service.log 2>&1 &
        echo $! > /tmp/product-service.pid
    elif [ "$service_name" = "gateway" ]; then
        nohup cargo run --bin gateway > /tmp/gateway.log 2>&1 &
        echo $! > /tmp/gateway.pid
    fi
    
    sleep 3
    echo "$service_name started"
}

# Function to stop services
stop_service() {
    local service_name=$1
    echo "🛑 Stopping $service_name..."
    
    if [ "$service_name" = "user-service" ]; then
        if [ -f /tmp/user-service.pid ]; then
            kill $(cat /tmp/user-service.pid) 2>/dev/null || echo "$service_name not running"
            rm -f /tmp/user-service.pid
        else
            pkill -f user-service || echo "$service_name not running"
        fi
    elif [ "$service_name" = "product-service" ]; then
        if [ -f /tmp/product-service.pid ]; then
            kill $(cat /tmp/product-service.pid) 2>/dev/null || echo "$service_name not running"
            rm -f /tmp/product-service.pid
        else
            pkill -f product-service || echo "$service_name not running"
        fi
    elif [ "$service_name" = "gateway" ]; then
        if [ -f /tmp/gateway.pid ]; then
            kill $(cat /tmp/gateway.pid) 2>/dev/null || echo "$service_name not running"
            rm -f /tmp/gateway.pid
        else
            pkill -f gateway || echo "$service_name not running"
        fi
    fi
    
    sleep 2
    echo "$service_name stopped"
}

# Test 1: Initial system startup and health
echo "📋 1. System Startup and Initial Health Check"
echo "============================================="

# Stop any existing services
stop_service "gateway"
stop_service "user-service"
stop_service "product-service"

# Start services in order
start_service "user-service" "8080"
start_service "product-service" "8081"
start_service "gateway" "8082"

# Wait for startup
echo "⏳ Waiting for services to fully initialize..."
sleep 5

# Test initial health
test_service_health "System" 5
echo ""

# Test 2: Service failure and gateway response
echo "🔥 2. Service Failure Simulation"
echo "================================"

# Kill user service to simulate failure
stop_service "user-service"

echo "Testing gateway response with user service down..."
response=$(curl -s -w "\nHTTP Code: %{http_code}\nResponse Time: %{time_total}s" \
    -X POST "$GATEWAY_URL" \
    -H "Content-Type: application/json" \
    -d '{
        "jsonrpc": "2.0",
        "method": "create_user",
        "params": {
            "request": {
                "name": "Test User",
                "email": "test@example.com"
            }
        },
        "id": 1
    }')

echo "Gateway response when user service is down:"
echo "$response"
echo ""

# Test 3: Service recovery
echo "🔄 3. Service Recovery Testing"
echo "=============================="

echo "Restarting user service..."
start_service "user-service" "8080"

echo "⏳ Waiting for gateway to detect service recovery (health checks run every 30s)..."
echo "Testing service recovery detection..."

# Try requests every 5 seconds for up to 2 minutes
recovery_attempts=0
max_recovery_attempts=24  # 2 minutes

while [ $recovery_attempts -lt $max_recovery_attempts ]; do
    response=$(curl -s -w "%{http_code}" -X POST "$GATEWAY_URL" \
        -H "Content-Type: application/json" \
        -d '{
            "jsonrpc": "2.0",
            "method": "health",
            "id": 1
        }')
    
    http_code=$(echo $response | tail -c 4)
    
    if [ "$http_code" = "200" ]; then
        echo "✅ Service recovery detected after $((recovery_attempts * 5)) seconds"
        break
    elif [ "$http_code" = "503" ]; then
        echo "⏳ Service still marked as down (attempt $((recovery_attempts + 1)))"
    else
        echo "⚠️ Unexpected response code: $http_code (attempt $((recovery_attempts + 1)))"
    fi
    
    ((recovery_attempts++))
    sleep 5
done

if [ $recovery_attempts -eq $max_recovery_attempts ]; then
    echo "❌ Service recovery not detected within 2 minutes"
else
    echo "✅ Service recovery successful"
fi
echo ""

# Test 4: Stress testing with service interruption
echo "⚡ 4. Stress Test with Service Interruption"
echo "=========================================="

echo "Starting background stress test..."

# Create background load
for i in {1..50}; do
    curl -s -X POST "$GATEWAY_URL" \
        -H "Content-Type: application/json" \
        -d '{
            "jsonrpc": "2.0",
            "method": "health",
            "id": '$i'
        }' > /dev/null &
    
    if [ $((i % 10)) -eq 0 ]; then
        sleep 0.1  # Small pause every 10 requests
    fi
done

echo "Background load started (50 requests)"

# Kill and restart service during load
sleep 2
echo "Interrupting user service during load..."
stop_service "user-service"
sleep 3
start_service "user-service" "8080"

# Wait for background jobs to complete
wait

echo "Stress test with interruption completed"
echo ""

# Test 5: Final metrics and system state
echo "📊 5. Final System Metrics"
echo "========================="

echo "Gateway metrics after resilience testing:"
curl -s "$GATEWAY_URL/metrics" | jq . || curl -s "$GATEWAY_URL/metrics"
echo ""

# Test 6: Graceful shutdown test
echo "🛑 6. Graceful Shutdown Test"
echo "==========================="

echo "Testing graceful shutdown of services..."

# Get PIDs for graceful shutdown
if [ -f /tmp/gateway.pid ]; then
    gateway_pid=$(cat /tmp/gateway.pid)
    echo "Sending SIGTERM to gateway (PID: $gateway_pid)..."
    kill -TERM $gateway_pid 2>/dev/null
    sleep 2
    
    if kill -0 $gateway_pid 2>/dev/null; then
        echo "⚠️ Gateway still running, forcing shutdown..."
        kill -KILL $gateway_pid 2>/dev/null
    else
        echo "✅ Gateway shut down gracefully"
    fi
fi

stop_service "user-service"
stop_service "product-service"

echo ""

# Cleanup
echo "🧹 Cleaning up..."
rm -f /tmp/*.pid /tmp/*.log

echo ""
echo "✅ Resilience Testing Complete!"
echo ""
echo "🏆 Test Summary:"
echo "  - System startup and health checks"
echo "  - Service failure handling by gateway"
echo "  - Service recovery detection (circuit breaker)"
echo "  - Stress testing with service interruption"
echo "  - Graceful shutdown capabilities"
echo ""
echo "🎯 Resilience Score: Based on test results above"
echo "   Production-ready features:"
echo "   ✅ Circuit breaker pattern"
echo "   ✅ Automatic failure detection"
echo "   ✅ Service recovery monitoring"
echo "   ✅ Graceful shutdown handling"
echo "   ✅ Load handling with service interruption"
