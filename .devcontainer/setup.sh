#!/usr/bin/env bash
# .devcontainer/setup.sh
#
# Roda UMA VEZ quando o container é criado (postCreateCommand).
# Idempotente — pode ser re-executado sem dano se algo falhar.
set -euo pipefail

echo "╔══════════════════════════════════════════╗"
echo "║  cardapio-geo-api — setup do container   ║"
echo "╚══════════════════════════════════════════╝"

# ── 1. Garante que o target WASI está presente ───────────────────────────
echo "▶  Verificando target wasm32-wasip1…"
rustup target add wasm32-wasip1

# ── 2. Instala Spin CLI ───────────────────────────────────────────────────
echo "▶  Instalando Spin CLI…"
SPIN_VERSION="v2.7.0"   # altere para a versão mais recente se quiser
SPIN_BIN="$HOME/.local/bin"
mkdir -p "$SPIN_BIN"

if ! command -v spin &>/dev/null; then
  curl -fsSL "https://github.com/fermyon/spin/releases/download/${SPIN_VERSION}/spin-${SPIN_VERSION}-linux-amd64.tar.gz" \
    | tar -xz -C "$SPIN_BIN" spin
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
  export PATH="$SPIN_BIN:$PATH"
  echo "   Spin $(spin --version) instalado."
else
  echo "   Spin já instalado: $(spin --version)"
fi

# ── 3. Ferramentas de qualidade de código ────────────────────────────────
echo "▶  Instalando cargo tools…"

# cargo-watch  → recompila ao salvar (alternativa ao `spin watch` para libs)
# cargo-audit  → verifica CVEs nas dependências
# cargo-expand → expande macros (útil para debug de proc-macros)
cargo install cargo-watch cargo-audit cargo-expand \
  --quiet \
  2>&1 | grep -E "^(Compiling|Installing|Installed|error)" || true

# ── 4. Pre-commit hook — roda fmt + clippy antes de cada commit ───────────
echo "▶  Configurando git hooks…"
HOOK_FILE=".git/hooks/pre-commit"
if [ -d ".git" ] && [ ! -f "$HOOK_FILE" ]; then
  cat > "$HOOK_FILE" << 'HOOK'
#!/usr/bin/env bash
set -e
echo "  [pre-commit] cargo fmt --check…"
cargo fmt --all -- --check
echo "  [pre-commit] cargo clippy…"
cargo clippy --target wasm32-wasip1 -- -D warnings
HOOK
  chmod +x "$HOOK_FILE"
  echo "   Hook instalado."
else
  echo "   Hook já existe ou .git não encontrado — pulando."
fi

echo ""
echo "✅  Container pronto!"
echo ""
echo "   Comandos úteis:"
echo "   spin watch          → build + servidor hot-reload em :3000"
echo "   cargo clippy        → linter"
echo "   cargo fmt           → formata o código"
echo "   cargo audit         → verifica CVEs"
echo "   cargo watch -x test → testa ao salvar"