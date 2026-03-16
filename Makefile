# cardapio-geo-api — Makefile
#
# Contexto de execução:
#   Targets Rust (dev/build/test/ci) → dentro do devcontainer
#   Targets Docker (osrm-*/up/down)  → no Windows/Mac/Linux (host), FORA do devcontainer
#
# Windows não tem `make` por padrão.
# Instala com: winget install GnuWin32.Make
# Ou use os comandos docker compose diretamente (README tem os equivalentes).

WASM_TARGET := wasm32-wasip1
WASM_BIN    := target/$(WASM_TARGET)/release/cardapio_geo_api.wasm
SPIN_PORT   := 3000

.DEFAULT_GOAL := help

# ── Help ──────────────────────────────────────────────────────────────────────
.PHONY: help
help:
	@echo ""
	@echo "  cardapio-geo-api"
	@echo ""
	@echo "  Dentro do devcontainer (VS Code):"
	@echo "    make dev          spin watch — hot-reload em :$(SPIN_PORT)"
	@echo "    make build        cargo build debug"
	@echo "    make release      cargo build release"
	@echo "    make fmt          formata o código"
	@echo "    make lint         clippy -D warnings"
	@echo "    make test         testes unitários"
	@echo "    make test-watch   testes em modo watch"
	@echo "    make audit        cargo-audit (CVEs)"
	@echo "    make deny         cargo-deny (licenças)"
	@echo "    make ci           todos os checks (igual ao GitHub Actions)"
	@echo "    make smoke        testa os 4 endpoints via curl"
	@echo "    make deploy       spin deploy → Fermyon Cloud"
	@echo ""
	@echo "  No Windows/Mac/Linux (FORA do devcontainer):"
	@echo "    make osrm-up      sobe OSRM — baixa e processa dados na 1ª vez (~10 min)"
	@echo "    make osrm-down    derruba o OSRM"
	@echo "    make osrm-status  verifica se o OSRM está respondendo"
	@echo "    make osrm-logs    tail dos logs do OSRM"
	@echo "    make up           stack de produção completa"
	@echo "    make down         derruba stack de produção"
	@echo ""

# ── Rust (devcontainer) ───────────────────────────────────────────────────────
.PHONY: dev
dev:
	spin watch

.PHONY: build
build:
	cargo build --target $(WASM_TARGET)

.PHONY: release
release: $(WASM_BIN)

$(WASM_BIN):
	cargo build --target $(WASM_TARGET) --release

.PHONY: fmt
fmt:
	cargo fmt --all

.PHONY: fmt-check
fmt-check:
	cargo fmt --all -- --check

.PHONY: lint
lint:
	cargo clippy --target $(WASM_TARGET) -- -D warnings

.PHONY: test
test:
	cargo test --lib -- --nocapture

.PHONY: test-watch
test-watch:
	cargo watch -x "test --lib -- --nocapture"

.PHONY: audit
audit:
	cargo audit

.PHONY: deny
deny:
	cargo deny check

.PHONY: ci
ci: fmt-check lint test audit deny release
	@echo ""
	@echo "✅  Todos os checks passaram. Pronto para push."

.PHONY: smoke
smoke:
	@echo "▶ /health"
	@curl -s http://localhost:$(SPIN_PORT)/health | jq .
	@echo ""
	@echo "▶ /geocode?q=Arapiraca"
	@curl -s "http://localhost:$(SPIN_PORT)/geocode?q=Arapiraca" | jq '.[0] | {display_name, within_alagoas, within_region}'
	@echo ""
	@echo "▶ /reverse (Maceió)"
	@curl -s "http://localhost:$(SPIN_PORT)/reverse?lat=-9.6658&lng=-35.7350" | jq '{display_name, within_alagoas}'
	@echo ""
	@echo "▶ /route (Maceió → Arapiraca)"
	@curl -s "http://localhost:$(SPIN_PORT)/route?from_lat=-9.6658&from_lng=-35.7350&to_lat=-9.7522&to_lng=-36.6613" | jq '{ok, distance_km, duration_min}'

.PHONY: deploy
deploy: release
	spin deploy

.PHONY: login
login:
	spin login

.PHONY: clean
clean:
	cargo clean

.PHONY: update
update:
	cargo update
	@echo "Rode 'make audit' e 'make deny' para verificar."

# ── Docker — roda no HOST (Windows/Mac/Linux), não no devcontainer ────────────

# Sobe o OSRM. Na primeira vez baixa o PBF e processa os dados (~10 min).
# Nas próximas vezes detecta os dados prontos e inicia em segundos.
# Acesse em localhost:5000 (host) ou host.docker.internal:5000 (devcontainer).
.PHONY: osrm-up
osrm-up:
	docker compose up -d osrm-download osrm

# Verifica se o OSRM está respondendo no host.
.PHONY: osrm-status
osrm-status:
	@curl -sf http://localhost:5000/health \
		&& echo "✅  OSRM online (localhost:5000)" \
		|| echo "❌  OSRM offline — rode 'make osrm-up'"

.PHONY: osrm-down
osrm-down:
	docker compose stop osrm osrm-download

.PHONY: osrm-logs
osrm-logs:
	docker compose logs -f osrm

# Stack de produção completa (sem devcontainer).
.PHONY: up
up:
	docker compose up -d --build

.PHONY: down
down:
	docker compose down