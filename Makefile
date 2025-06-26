# Makefile for JPC-Rust User Service

.PHONY: help build run-local run-docker test clean

# Default target
help:
	@echo "🦀 JPC-Rust User Service"
	@echo "======================="
	@echo ""
	@echo "Available targets:"
	@echo "  build        - Build all binaries"
	@echo "  run-local    - Run services locally"
	@echo "  run-docker   - Run with Docker Compose"
	@echo "  test         - Run tests"
	@echo "  test-api     - Test API endpoints"
	@echo "  clean        - Clean build artifacts"
	@echo "  stop         - Stop Docker services"
	@echo ""

# Build all binaries
build:
	@echo "🔨 Building all binaries..."
	cargo build --release

# Run services locally (requires SurrealDB)
run-local:
	@echo "🚀 Starting services locally..."
	@echo "Note: Make sure SurrealDB is running on port 8000"
	@echo ""
	@echo "Starting User Service on port 8080..."
	cargo run --bin user-service &
	@sleep 2
	@echo "Starting Product Service on port 8081..."
	cargo run --bin product-service &
	@sleep 2
	@echo "Starting Gateway on port 8082..."
	cargo run --bin gateway &
	@echo ""
	@echo "✅ Services started! Test with: make test-api"

# Run with Docker Compose
run-docker:
	@echo "🐳 Starting services with Docker Compose..."
	docker-compose up --build -d
	@echo ""
	@echo "✅ Services started! Test with: make test-api"

# Run unit tests
test:
	@echo "🧪 Running unit tests..."
	cargo test

# Test API endpoints
test-api:
	@echo "🧪 Testing API endpoints..."
	./test_api.sh

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	docker-compose down -v
	docker system prune -f

# Stop Docker services
stop:
	@echo "🛑 Stopping Docker services..."
	docker-compose down

# Development helpers
dev-setup:
	@echo "🛠️  Setting up development environment..."
	rustup update
	cargo install cargo-watch
	@echo "✅ Development setup complete!"

# Watch mode for development
dev-watch:
	@echo "👀 Starting development watch mode..."
	cargo watch -x "run --bin user-service"

# Format code
fmt:
	@echo "🎨 Formatting code..."
	cargo fmt

# Lint code
lint:
	@echo "🔍 Linting code..."
	cargo clippy -- -D warnings

# Check everything
check: fmt lint test
	@echo "✅ All checks passed!"
