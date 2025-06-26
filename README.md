# JPC-Rust User Service 🦀

A modern Rust-based microservice architecture showcasing:

- **JSON-RPC 2.0 API** with `jsonrpsee`
- **SurrealDB** for data persistence
- **Pingora** as an API gateway
- **Docker Compose** for easy deployment

## 🏗️ Architecture

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Client    │───▶│   Pingora   │───▶│ User Service│
│             │    │  Gateway    │    │ (JSON-RPC)  │
└─────────────┘    └─────────────┘    └─────────────┘
                         │                    │
                         │                    ▼
                         │              ┌─────────────┐
                         │              │ SurrealDB   │
                         │              │             │
                         │              └─────────────┘
                         ▼
                   ┌─────────────┐
                   │   Logs &    │
                   │ Monitoring  │
                   └─────────────┘
```

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+
- Docker & Docker Compose
- `jq` (for testing scripts)
- `curl`

### Option 1: Run with Docker Compose (Recommended)

```bash
# Build and start all services
docker-compose up --build

# Test the API
./test_api.sh
```

### Option 2: Run Locally

```bash
# Start SurrealDB (optional - service uses in-memory DB by default)
docker run -p 8000:8000 surrealdb/surrealdb:latest start --user root --pass root memory

# Terminal 1: Start User Service
cargo run --bin user-service

# Terminal 2: Start Gateway
cargo run --bin gateway

# Terminal 3: Test the API
./test_api.sh
```

## 📡 API Endpoints

All endpoints are JSON-RPC 2.0 over HTTP POST to:

- **Direct**: `http://localhost:8080` (User Service)
- **Via Gateway**: `http://localhost:8081` (Pingora Gateway)

### Available Methods

#### `health()`

```json
{
  "jsonrpc": "2.0",
  "method": "health",
  "id": 1
}
```

#### `create_user(name: String, email: String)`

```json
{
  "jsonrpc": "2.0",
  "method": "create_user",
  "params": {
    "name": "John Doe",
    "email": "john.doe@example.com"
  },
  "id": 2
}
```

#### `get_user(id: String)`

```json
{
  "jsonrpc": "2.0",
  "method": "get_user",
  "params": {
    "id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "id": 3
}
```

#### `list_users()`

```json
{
  "jsonrpc": "2.0",
  "method": "list_users",
  "id": 4
}
```

## 🧪 Testing

### Automated Tests

```bash
./test_api.sh
```

### Manual Testing with curl

```bash
# Create a user
curl -X POST http://localhost:8081 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "create_user",
    "params": {
      "name": "Alice Johnson",
      "email": "alice@example.com"
    },
    "id": 1
  }'

# List all users
curl -X POST http://localhost:8081 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "list_users",
    "id": 2
  }'
```

## 🛠️ Development

### Project Structure

```
src/
├── lib.rs              # Library exports
├── models.rs           # Data models & DTOs
├── errors.rs           # Error types
├── repository.rs       # Database layer
├── service.rs          # Business logic
└── bin/
    ├── user_service.rs # JSON-RPC server
    └── gateway.rs      # Pingora gateway
```

### Key Dependencies

- `jsonrpsee` - JSON-RPC 2.0 server/client
- `surrealdb` - Multi-model database
- `pingora` - Layer-7 proxy
- `tokio` - Async runtime
- `serde` - Serialization
- `tracing` - Structured logging
- `anyhow` & `thiserror` - Error handling

### Building

```bash
# Build all binaries
cargo build --release

# Build specific binary
cargo build --bin user-service
cargo build --bin gateway
```

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests with running service
cargo run --bin user-service &
./test_api.sh
```

## 🔧 Configuration

### Environment Variables

- `RUST_LOG` - Log level (debug, info, warn, error)
- `DATABASE_URL` - SurrealDB connection string

### Ports

- **8080** - User Service (JSON-RPC)
- **8081** - Pingora Gateway
- **8000** - SurrealDB (if running separately)

## 🐳 Docker

### Building Images

```bash
# User Service
docker build -f Dockerfile.user-service -t user-service .

# Gateway
docker build -f Dockerfile.gateway -t gateway .
```

### Docker Compose Services

- `surrealdb` - Database server
- `user-service` - JSON-RPC API server
- `gateway` - Pingora proxy gateway

## 📊 Features

### ✅ Implemented

- [x] JSON-RPC 2.0 API with `jsonrpsee`
- [x] SurrealDB integration with in-memory storage
- [x] CRUD operations for users
- [x] Input validation & error handling
- [x] Pingora gateway with routing
- [x] Docker Compose setup
- [x] Structured logging with `tracing`
- [x] Health check endpoints
- [x] CORS support

### 🚧 Roadmap

- [ ] Authentication & authorization
- [ ] Rate limiting
- [ ] Metrics & monitoring (Prometheus)
- [ ] Database persistence (file/network storage)
- [ ] API documentation (OpenAPI)
- [ ] Load balancing multiple service instances
- [ ] WebSocket support
- [ ] Distributed tracing

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📝 License

MIT License - see LICENSE file for details.

---

**Built with ❤️ and 🦀 Rust**
