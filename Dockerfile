FROM rust:slim AS builder
# Instala o target WASI
RUN rustup target add wasm32-wasip1

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src

RUN cargo build --target wasm32-wasip1 --release

# ── Stage 2: Runtime com Spin ──────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Instala Spin CLI
RUN apt-get update && apt-get install -y curl ca-certificates && \
    curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash && \
    mv spin /usr/local/bin/spin && \
    apt-get remove -y curl && apt-get autoremove -y && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY spin.toml .
COPY --from=builder \
     /app/target/wasm32-wasip1/release/cardapio_geo_api.wasm \
     target/wasm32-wasip1/release/cardapio_geo_api.wasm

EXPOSE 3000

# Spin escuta em 0.0.0.0 para ser acessível fora do container
CMD ["spin", "up", "--listen", "0.0.0.0:3000"]