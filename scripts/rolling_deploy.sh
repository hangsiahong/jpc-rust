#!/bin/bash
# Rolling deployment script

SERVICE=$1
if [ -z "$SERVICE" ]; then
    echo "Usage: $0 <service-name>"
    echo "Available services: user-service, product-service, gateway"
    exit 1
fi

echo "🔄 Rolling deployment for $SERVICE..."

# Build the specific service
echo "📦 Building $SERVICE..."
cargo build --bin $SERVICE --release

# Kill old process gracefully
echo "🛑 Stopping old $SERVICE..."
pkill -f "target/release/$SERVICE" || echo "No existing process found"

# Wait a moment for graceful shutdown
sleep 2

# Start new process in background
echo "🚀 Starting new $SERVICE..."
nohup ./target/release/$SERVICE > logs/$SERVICE.log 2>&1 &

echo "✅ $SERVICE deployment complete!"
echo "📊 Process status:"
ps aux | grep $SERVICE | grep -v grep
