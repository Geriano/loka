# Loka Stratum Mining Proxy - Development Makefile
.PHONY: help build test clean fmt lint audit bench docker docker-build docker-push deploy-dev deploy-prod migrate setup ci

# Default target
.DEFAULT_GOAL := help

# Variables
RUST_VERSION ?= stable
CARGO := cargo
DOCKER := docker
COMPOSE := docker-compose
PROJECT_NAME := loka-stratum
REGISTRY := ghcr.io/geriano/loka
DATABASE_URL ?= postgres://root:root@localhost:5432/loka

# Colors for output
RED := \033[0;31m
GREEN := \033[0;32m
YELLOW := \033[1;33m
BLUE := \033[0;34m
NC := \033[0m # No Color

help: ## Show this help message
	@echo "$(BLUE)Loka Stratum Mining Proxy - Development Commands$(NC)"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"; printf "Usage: make $(GREEN)<target>$(NC)\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*?##/ { printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2 } \
		/^##@/ { printf "\n$(BLUE)%s$(NC)\n", substr($$0, 5) }' $(MAKEFILE_LIST)

##@ Development

setup: ## Setup development environment
	@echo "$(BLUE)Setting up development environment...$(NC)"
	rustup update $(RUST_VERSION)
	rustup component add rustfmt clippy
	$(CARGO) install sea-orm-cli --locked --force
	$(CARGO) install cargo-audit --locked --force
	$(CARGO) install cargo-outdated --locked --force
	@echo "$(GREEN)✅ Development environment setup complete$(NC)"

build: ## Build all workspace crates
	@echo "$(BLUE)Building workspace...$(NC)"
	$(CARGO) build --workspace
	@echo "$(GREEN)✅ Build complete$(NC)"

build-release: ## Build release version
	@echo "$(BLUE)Building release version...$(NC)"
	$(CARGO) build --workspace --release
	@echo "$(GREEN)✅ Release build complete$(NC)"

test: ## Run all tests
	@echo "$(BLUE)Running tests...$(NC)"
	$(CARGO) test --workspace
	@echo "$(GREEN)✅ Tests complete$(NC)"

test-integration: ## Run integration tests with database
	@echo "$(BLUE)Running integration tests...$(NC)"
	@if [ -z "$(DATABASE_URL)" ]; then \
		echo "$(RED)❌ DATABASE_URL not set$(NC)"; \
		exit 1; \
	fi
	cd migration && $(CARGO) run
	$(CARGO) test --workspace --test '*'
	@echo "$(GREEN)✅ Integration tests complete$(NC)"

##@ Code Quality

fmt: ## Format code
	@echo "$(BLUE)Formatting code...$(NC)"
	$(CARGO) fmt --all
	@echo "$(GREEN)✅ Code formatted$(NC)"

fmt-check: ## Check code formatting
	@echo "$(BLUE)Checking code formatting...$(NC)"
	$(CARGO) fmt --all -- --check
	@echo "$(GREEN)✅ Code formatting check complete$(NC)"

lint: ## Run clippy linter
	@echo "$(BLUE)Running clippy...$(NC)"
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	@echo "$(GREEN)✅ Linting complete$(NC)"

audit: ## Run security audit
	@echo "$(BLUE)Running security audit...$(NC)"
	$(CARGO) audit
	@echo "$(GREEN)✅ Security audit complete$(NC)"

outdated: ## Check for outdated dependencies
	@echo "$(BLUE)Checking for outdated dependencies...$(NC)"
	$(CARGO) outdated
	@echo "$(GREEN)✅ Dependency check complete$(NC)"

##@ Performance

bench: ## Run all benchmarks
	@echo "$(BLUE)Running benchmarks...$(NC)"
	cd stratum && chmod +x run_benchmarks.sh && ./run_benchmarks.sh all
	@echo "$(GREEN)✅ Benchmarks complete$(NC)"

bench-critical: ## Run critical path benchmarks
	@echo "$(BLUE)Running critical path benchmarks...$(NC)"
	cd stratum && chmod +x run_benchmarks.sh && ./run_benchmarks.sh critical
	@echo "$(GREEN)✅ Critical path benchmarks complete$(NC)"

bench-metrics: ## Run metrics performance benchmarks
	@echo "$(BLUE)Running metrics benchmarks...$(NC)"
	cd stratum && chmod +x run_benchmarks.sh && ./run_benchmarks.sh metrics
	@echo "$(GREEN)✅ Metrics benchmarks complete$(NC)"

validate-performance: ## Validate performance requirements
	@echo "$(BLUE)Validating performance...$(NC)"
	cd stratum && chmod +x validate_performance.sh && ./validate_performance.sh
	@echo "$(GREEN)✅ Performance validation complete$(NC)"

##@ Database

migrate: ## Run database migrations
	@echo "$(BLUE)Running database migrations...$(NC)"
	@if [ -z "$(DATABASE_URL)" ]; then \
		echo "$(RED)❌ DATABASE_URL not set$(NC)"; \
		exit 1; \
	fi
	cd migration && $(CARGO) run
	@echo "$(GREEN)✅ Migrations complete$(NC)"

migrate-status: ## Check migration status
	@echo "$(BLUE)Checking migration status...$(NC)"
	@if [ -z "$(DATABASE_URL)" ]; then \
		echo "$(RED)❌ DATABASE_URL not set$(NC)"; \
		exit 1; \
	fi
	cd migration && $(CARGO) run -- status
	@echo "$(GREEN)✅ Migration status check complete$(NC)"

generate-entities: ## Generate SeaORM entities from database
	@echo "$(BLUE)Generating SeaORM entities...$(NC)"
	@if [ -z "$(DATABASE_URL)" ]; then \
		echo "$(RED)❌ DATABASE_URL not set$(NC)"; \
		exit 1; \
	fi
	sea-orm-cli generate entity --database-url $(DATABASE_URL) --output-dir model/src/entities
	@echo "$(GREEN)✅ Entity generation complete$(NC)"

##@ Docker

docker-build: ## Build Docker image
	@echo "$(BLUE)Building Docker image...$(NC)"
	$(DOCKER) build -f stratum/Dockerfile -t $(PROJECT_NAME):latest .
	@echo "$(GREEN)✅ Docker build complete$(NC)"

docker-build-release: ## Build release Docker image
	@echo "$(BLUE)Building release Docker image...$(NC)"
	$(DOCKER) build -f stratum/Dockerfile --target release -t $(PROJECT_NAME):release .
	@echo "$(GREEN)✅ Release Docker build complete$(NC)"

docker-build-debug: ## Build debug Docker image
	@echo "$(BLUE)Building debug Docker image...$(NC)"
	$(DOCKER) build -f stratum/Dockerfile --target debug -t $(PROJECT_NAME):debug .
	@echo "$(GREEN)✅ Debug Docker build complete$(NC)"

docker-run: ## Run Docker container locally
	@echo "$(BLUE)Running Docker container...$(NC)"
	$(DOCKER) run --rm -it \
		-p 3333:3333 \
		-p 9090:9090 \
		-e DATABASE_URL=$(DATABASE_URL) \
		$(PROJECT_NAME):latest
	@echo "$(GREEN)✅ Container stopped$(NC)"

docker-test: ## Test Docker image
	@echo "$(BLUE)Testing Docker image...$(NC)"
	$(DOCKER) run --rm -d --name $(PROJECT_NAME)-test \
		-p 3333:3333 -p 9090:9090 \
		-e DATABASE_URL=$(DATABASE_URL) \
		$(PROJECT_NAME):latest
	sleep 10
	curl -f http://localhost:9090/health || (echo "$(RED)❌ Health check failed$(NC)" && exit 1)
	curl -f http://localhost:9090/metrics/prometheus > /dev/null || (echo "$(RED)❌ Metrics check failed$(NC)" && exit 1)
	$(DOCKER) stop $(PROJECT_NAME)-test
	@echo "$(GREEN)✅ Docker test complete$(NC)"

docker-push: ## Push Docker image to registry
	@echo "$(BLUE)Pushing Docker image...$(NC)"
	$(DOCKER) tag $(PROJECT_NAME):latest $(REGISTRY)/$(PROJECT_NAME):latest
	$(DOCKER) push $(REGISTRY)/$(PROJECT_NAME):latest
	@echo "$(GREEN)✅ Docker push complete$(NC)"

##@ Docker Compose

up: ## Start services with docker-compose
	@echo "$(BLUE)Starting services...$(NC)"
	cd stratum && $(COMPOSE) up -d
	@echo "$(GREEN)✅ Services started$(NC)"

up-monitoring: ## Start monitoring stack
	@echo "$(BLUE)Starting monitoring stack...$(NC)"
	cd monitoring && $(COMPOSE) up -d
	@echo "$(GREEN)✅ Monitoring stack started$(NC)"

up-full: ## Start full stack (monitoring + application)
	@echo "$(BLUE)Starting full stack...$(NC)"
	cd monitoring && $(COMPOSE) up -d
	sleep 15
	cd ../stratum && $(COMPOSE) up -d
	@echo "$(GREEN)✅ Full stack started$(NC)"

down: ## Stop services
	@echo "$(BLUE)Stopping services...$(NC)"
	cd stratum && $(COMPOSE) down
	@echo "$(GREEN)✅ Services stopped$(NC)"

down-monitoring: ## Stop monitoring stack
	@echo "$(BLUE)Stopping monitoring stack...$(NC)"
	cd monitoring && $(COMPOSE) down
	@echo "$(GREEN)✅ Monitoring stack stopped$(NC)"

down-full: ## Stop full stack
	@echo "$(BLUE)Stopping full stack...$(NC)"
	cd stratum && $(COMPOSE) down
	cd ../monitoring && $(COMPOSE) down
	@echo "$(GREEN)✅ Full stack stopped$(NC)"

logs: ## Show application logs
	@echo "$(BLUE)Showing application logs...$(NC)"
	cd stratum && $(COMPOSE) logs -f

logs-monitoring: ## Show monitoring stack logs
	@echo "$(BLUE)Showing monitoring logs...$(NC)"
	cd monitoring && $(COMPOSE) logs -f

##@ CI/CD Simulation

ci: ## Run full CI pipeline locally
	@echo "$(BLUE)Running CI pipeline locally...$(NC)"
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) audit
	$(MAKE) test
	$(MAKE) docker-build
	$(MAKE) docker-test
	@echo "$(GREEN)✅ CI pipeline complete$(NC)"

ci-performance: ## Run CI pipeline with performance tests
	@echo "$(BLUE)Running CI pipeline with performance tests...$(NC)"
	$(MAKE) ci
	$(MAKE) bench-critical
	$(MAKE) validate-performance
	@echo "$(GREEN)✅ CI pipeline with performance tests complete$(NC)"

##@ Deployment

deploy-dev: ## Deploy to development environment
	@echo "$(BLUE)Deploying to development...$(NC)"
	$(MAKE) docker-build
	$(MAKE) up-full
	@echo "$(GREEN)✅ Development deployment complete$(NC)"

deploy-staging: ## Deploy to staging environment  
	@echo "$(BLUE)Deploying to staging...$(NC)"
	@echo "$(YELLOW)⚠️ This would deploy to staging environment$(NC)"
	@echo "Implement actual staging deployment logic here"

deploy-prod: ## Deploy to production environment
	@echo "$(BLUE)Deploying to production...$(NC)"
	@echo "$(YELLOW)⚠️ This would deploy to production environment$(NC)"
	@echo "Implement actual production deployment logic here"

##@ Maintenance

clean: ## Clean build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	$(CARGO) clean
	$(DOCKER) system prune -f
	@echo "$(GREEN)✅ Clean complete$(NC)"

clean-all: ## Clean everything including Docker volumes
	@echo "$(BLUE)Cleaning everything...$(NC)"
	$(MAKE) down-full
	$(CARGO) clean
	$(DOCKER) system prune -af --volumes
	@echo "$(GREEN)✅ Deep clean complete$(NC)"

update: ## Update dependencies
	@echo "$(BLUE)Updating dependencies...$(NC)"
	$(CARGO) update
	@echo "$(GREEN)✅ Dependencies updated$(NC)"

##@ Utilities

status: ## Show system status
	@echo "$(BLUE)System Status$(NC)"
	@echo "Rust version: $$(rustc --version)"
	@echo "Cargo version: $$(cargo --version)"
	@echo "Docker version: $$(docker --version)"
	@echo "Docker Compose version: $$(docker-compose --version)"
	@echo ""
	@echo "$(BLUE)Service Status$(NC)"
	@if $(DOCKER) ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | grep -E "(loka|prometheus|grafana)" > /dev/null 2>&1; then \
		$(DOCKER) ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | grep -E "(loka|prometheus|grafana)"; \
	else \
		echo "No Loka services running"; \
	fi

health: ## Check health of running services
	@echo "$(BLUE)Checking service health...$(NC)"
	@if curl -s http://localhost:9090/health > /dev/null 2>&1; then \
		echo "$(GREEN)✅ Loka Stratum: Healthy$(NC)"; \
	else \
		echo "$(RED)❌ Loka Stratum: Unhealthy or not running$(NC)"; \
	fi
	@if curl -s http://localhost:3000/api/health > /dev/null 2>&1; then \
		echo "$(GREEN)✅ Grafana: Healthy$(NC)"; \
	else \
		echo "$(RED)❌ Grafana: Unhealthy or not running$(NC)"; \
	fi
	@if curl -s http://localhost:9090/-/healthy > /dev/null 2>&1; then \
		echo "$(GREEN)✅ Prometheus: Healthy$(NC)"; \
	else \
		echo "$(RED)❌ Prometheus: Unhealthy or not running$(NC)"; \
	fi

##@ Development Shortcuts

dev: ## Start development environment
	@echo "$(BLUE)Starting development environment...$(NC)"
	$(MAKE) up-monitoring
	sleep 15
	$(MAKE) build
	$(MAKE) up
	$(MAKE) health
	@echo "$(GREEN)✅ Development environment ready$(NC)"
	@echo "$(YELLOW)Access points:$(NC)"
	@echo "  - Loka Stratum: http://localhost:9091"
	@echo "  - Grafana: http://localhost:3000 (admin/admin123)"
	@echo "  - Prometheus: http://localhost:9090"

dev-logs: ## Watch development logs
	@echo "$(BLUE)Watching development logs...$(NC)"
	$(MAKE) logs

dev-stop: ## Stop development environment
	@echo "$(BLUE)Stopping development environment...$(NC)"
	$(MAKE) down-full
	@echo "$(GREEN)✅ Development environment stopped$(NC)"

quick-test: ## Quick test cycle (format, lint, test)
	@echo "$(BLUE)Running quick test cycle...$(NC)"
	$(MAKE) fmt
	$(MAKE) lint
	$(MAKE) test
	@echo "$(GREEN)✅ Quick test cycle complete$(NC)"

##@ Performance Testing

perf-test: ## Run performance test suite
	@echo "$(BLUE)Running performance test suite...$(NC)"
	$(MAKE) bench-critical
	$(MAKE) validate-performance
	cd stratum && ./run_benchmarks.sh compare
	@echo "$(GREEN)✅ Performance testing complete$(NC)"

load-test: ## Run load testing
	@echo "$(BLUE)Running load test...$(NC)"
	@echo "$(YELLOW)⚠️ Implement load testing logic$(NC)"
	@echo "Consider using tools like wrk, hey, or Artillery"

##@ Release Management

pre-release: ## Prepare for release
	@echo "$(BLUE)Preparing for release...$(NC)"
	$(MAKE) ci-performance
	$(MAKE) docker-build-release
	@echo "$(GREEN)✅ Release preparation complete$(NC)"

release-check: ## Check if ready for release
	@echo "$(BLUE)Checking release readiness...$(NC)"
	@echo "Running comprehensive checks..."
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) audit
	$(MAKE) test-integration
	$(MAKE) bench-critical
	$(MAKE) docker-test
	@echo "$(GREEN)✅ Release readiness check complete$(NC)"

##@ Documentation

docs: ## Generate documentation
	@echo "$(BLUE)Generating documentation...$(NC)"
	$(CARGO) doc --workspace --no-deps
	@echo "$(GREEN)✅ Documentation generated$(NC)"
	@echo "Open: target/doc/loka_stratum/index.html"

docs-open: ## Generate and open documentation
	@echo "$(BLUE)Generating and opening documentation...$(NC)"
	$(CARGO) doc --workspace --no-deps --open

##@ Troubleshooting

debug: ## Start in debug mode
	@echo "$(BLUE)Starting in debug mode...$(NC)"
	RUST_LOG=debug $(CARGO) run -p loka-stratum -- -vv start

debug-docker: ## Run Docker container in debug mode
	@echo "$(BLUE)Running Docker container in debug mode...$(NC)"
	$(DOCKER) run --rm -it \
		-p 3333:3333 \
		-p 9090:9090 \
		-e DATABASE_URL=$(DATABASE_URL) \
		-e RUST_LOG=debug \
		-e RUST_BACKTRACE=1 \
		$(PROJECT_NAME):latest

check-deps: ## Check dependency tree
	@echo "$(BLUE)Checking dependency tree...$(NC)"
	$(CARGO) tree

check-features: ## Check feature flags
	@echo "$(BLUE)Checking feature flags...$(NC)"
	$(CARGO) tree --format "{p} {f}"

##@ Environment Information

env-info: ## Show environment information
	@echo "$(BLUE)Environment Information$(NC)"
	@echo "Working Directory: $$(pwd)"
	@echo "Rust Version: $$(rustc --version)"
	@echo "Cargo Version: $$(cargo --version)"
	@echo "Platform: $$(uname -a)"
	@echo "Docker Version: $$(docker --version)"
	@echo "Available Memory: $$(free -h 2>/dev/null || echo 'N/A (not Linux)')"
	@echo "CPU Cores: $$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 'Unknown')"