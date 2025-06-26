# Performance improvements for production

## 1. Database Persistence

# Replace in-memory DB with persistent storage

# In repository files, change:

# Surreal::new::<Mem>(()).await?;

# To:

# Surreal::new::<File>("./data/service.db").await?;

## 2. Connection Pooling

# Add to Cargo.toml:

# tokio = { version = "1.0", features = ["full", "rt-multi-thread"] }

## 3. Gateway Optimizations

# - Keep alive connections

# - Request/Response caching

# - Load balancing for multiple instances

## 4. Monitoring

# Add metrics and health checks

# - Prometheus metrics

# - Health check endpoints

# - Request tracing

## 5. Horizontal Scaling

# Run multiple instances:

# - user-service on ports 8080, 8083, 8084

# - product-service on ports 8081, 8085, 8086

# - Gateway load balances between them
