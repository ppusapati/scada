.PHONY: help dev stop backend frontend simulator embedded-water embedded-solar build test clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ── Development ──

dev: ## Start all infrastructure (Postgres + Mosquitto + Backend)
	cd deploy/docker && docker-compose up -d postgres mosquitto
	@echo "Waiting for services..."
	@sleep 5
	@echo "Infrastructure ready. Run 'make backend' and 'make frontend' separately."

stop: ## Stop all infrastructure
	cd deploy/docker && docker-compose down

backend: ## Run Go backend server
	cd backend && go run cmd/server/main.go

frontend: ## Run Svelte frontend dev server
	cd frontend && npm run dev

simulator: ## Run MQTT data simulator (generates test data)
	cd backend && go run cmd/simulator/main.go

embedded-water: ## Run Rust water sensor firmware (simulator mode)
	cd embedded && cargo run --bin water-sensor

embedded-solar: ## Run Rust solar sensor firmware (simulator mode)
	cd embedded && cargo run --bin solar-sensor

# ── Build ──

build: build-backend build-frontend build-embedded ## Build everything

build-backend: ## Build Go backend
	cd backend && CGO_ENABLED=0 go build -o bin/server cmd/server/main.go
	cd backend && CGO_ENABLED=0 go build -o bin/simulator cmd/simulator/main.go

build-frontend: ## Build Svelte frontend
	cd frontend && npm run build

build-embedded: ## Build Rust embedded firmware
	cd embedded && cargo build --release

# ── Docker ──

docker-build: ## Build Docker images
	cd deploy/docker && docker-compose build

docker-up: ## Start full stack with Docker Compose
	cd deploy/docker && docker-compose up -d

docker-down: ## Stop Docker Compose stack
	cd deploy/docker && docker-compose down

docker-logs: ## Show Docker Compose logs
	cd deploy/docker && docker-compose logs -f

# ── Database ──

db-migrate: ## Run database migrations
	cd deploy/docker && docker-compose exec postgres psql -U scada -d scada_db -f /docker-entrypoint-initdb.d/001_init.sql

db-shell: ## Open PostgreSQL shell
	cd deploy/docker && docker-compose exec postgres psql -U scada -d scada_db

# ── MQTT ──

mqtt-sub: ## Subscribe to all SCADA MQTT topics (for debugging)
	mosquitto_sub -h localhost -t 'scada/#' -v

mqtt-test-water: ## Publish test water telemetry message
	mosquitto_pub -h localhost -t 'scada/water/test-001/telemetry' -m '{"device_id":"test-001","timestamp":"$(shell date -Iseconds)","metrics":{"tank_level":75.5,"ph_level":7.1,"pressure":3.2},"quality":0}'

# ── Kubernetes ──

k8s-deploy: ## Deploy to Kubernetes cluster
	kubectl apply -f deploy/k8s/namespace.yaml
	kubectl apply -f deploy/k8s/secrets.yaml
	kubectl apply -f deploy/k8s/postgres.yaml
	kubectl apply -f deploy/k8s/mosquitto.yaml
	kubectl apply -f deploy/k8s/backend.yaml

k8s-delete: ## Remove from Kubernetes cluster
	kubectl delete namespace scada

# ── Testing ──

test: ## Run all tests
	cd backend && go test ./...
	cd embedded && cargo test

clean: ## Clean build artifacts
	rm -rf backend/bin
	rm -rf frontend/build
	cd embedded && cargo clean
