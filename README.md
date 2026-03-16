# cardapio-geo-api

API de geocoding e roteamento para **Alagoas, BR**, escrita em Rust/WASI e servida pelo [Spin (Fermyon)](https://developer.fermyon.com/spin/v2).

Proxy sobre Nominatim OSM e OSRM com validação de coordenadas dentro da área operacional. Resposta compatível com o `address.js` do Cardápio Digital Pro — veja [`INTEGRATION.md`](./INTEGRATION.md) para os detalhes.

---

## Índice

- [Pré-requisitos](#pré-requisitos)
- [Estrutura do projeto](#estrutura-do-projeto)
- [Desenvolvimento local](#desenvolvimento-local)
- [Rotina diária](#rotina-diária)
- [Testes](#testes)
- [CI/CD](#cicd)
- [Endpoints](#endpoints)
- [Produção](#produção)
- [Infra self-hosted (OSRM)](#infra-self-hosted-osrm)

---

## Pré-requisitos

| Ferramenta | Versão mínima | Instalação |
|---|---|---|
| Docker Desktop | qualquer | [docker.com](https://www.docker.com/products/docker-desktop/) |
| VS Code | qualquer | [code.visualstudio.com](https://code.visualstudio.com/) |
| Extensão Dev Containers | qualquer | [marketplace](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) |

Rust, Spin CLI e todas as ferramentas de build rodam **dentro do dev container** — você não precisa instalar nada no seu sistema.

---

## Estrutura do projeto

```
cardapio-geo-api/
├── .devcontainer/
│   ├── devcontainer.json      # dockerComposeFile, extensões, portas, postCreateCommand
│   ├── docker-compose.dev.yml # serviço "devcontainer" + caches Cargo + OSRM sidecar
│   └── setup.sh               # instala wasm32-wasip1, Spin CLI, cargo tools, git hook
├── .github/
│   ├── dependabot.yml         # atualização automática semanal de deps
│   └── workflows/
│       ├── ci.yml             # fmt → clippy → test → build → audit → deny
│       └── cd.yml             # deploy → Fermyon Cloud (só na main)
├── src/
│   ├── lib.rs                 # entry point WASI — roteamento por path
│   ├── errors.rs              # ApiError tipado com thiserror
│   ├── bounds.rs              # bbox Alagoas, fn classify()
│   ├── helpers.rs             # parse_qs, url_decode, json_ok, cors
│   ├── geocode.rs             # GET /geocode  → Nominatim
│   ├── reverse.rs             # GET /reverse  → Nominatim
│   └── route.rs               # GET /route    → OSRM
├── Cargo.toml
├── Cargo.lock                 # commitado — builds reproduzíveis
├── deny.toml                  # política de licenças e CVEs (cargo-deny)
├── Makefile                   # atalhos para todos os comandos do projeto
├── spin.toml                  # manifesto Spin v2
├── rust-toolchain.toml        # trava Rust stable + target wasm32-wasip1
├── Dockerfile                 # imagem de produção (~80 MB)
└── docker-compose.yml         # geo-api + OSRM self-hosted (prod e base do dev)
```

---

## Desenvolvimento local

O dev container usa `dockerComposeFile` — sobe **dois containers em paralelo**: o ambiente Rust (onde você edita e compila) e o OSRM (servidor de rotas local). Ambos compartilham a rede interna do Docker.

### Como o OSRM funciona no dev

O OSRM precisa de dados processados antes de servir rotas. Se os dados não existirem, ele sobe em modo `sleep` e o endpoint `/route` retorna 502 — todos os outros endpoints (`/geocode`, `/reverse`, `/health`) funcionam normalmente. Isso é intencional: você pode abrir o container e trabalhar sem precisar esperar o processamento dos dados.

### 1. Primeira vez — preparar os dados do OSRM

Execute **fora do container** (no terminal do seu sistema operacional):

```bash
# Baixa o extrato do Nordeste (~120 MB) e processa os dados (~10 min)
# Os dados ficam no volume Docker "osrm-data" — roda uma vez só
make osrm-prepare
```

Você vai ver:

```
→ Baixando extrato do Nordeste (~120 MB)…
→ osrm-extract…
→ osrm-partition…
→ osrm-customize…
✅  OSRM pronto.
```

### 2. Abrir no container

```
Ctrl+Shift+P → Dev Containers: Reopen in Container
```

O VS Code vai:
1. Subir o container `devcontainer` (ambiente Rust) e o `osrm` (servidor de rotas)
2. Rodar `.devcontainer/setup.sh` — instala wasm32-wasip1, Spin CLI, cargo tools, git hook
3. Instalar as extensões do VS Code

**Acontece uma vez.** Nas próximas aberturas os containers já estão prontos e os caches do Cargo preservados.

> Se você abriu o container **antes** de rodar `make osrm-prepare`, o OSRM está em modo sleep. Após preparar os dados, rode dentro do container:
> ```bash
> make osrm-restart
> ```

### 3. Iniciar o servidor

```
Ctrl+Shift+B
```

Isso dispara `spin watch`. O servidor sobe em `http://localhost:3000` com hot-reload — qualquer alteração em `src/` recompila e reinicia automaticamente.

**Como confirmar que o hot reload está funcionando:**

```
Building component geo with `cargo build --target wasm32-wasip1 --release`
Finished `release` profile [optimized] target(s) in 2.3s
Serving http://127.0.0.1:3000
```

Primeiro build: 30–60s. Rebuilds incrementais: 2–5s (só recompila o que mudou).

Teste rápido: muda a versão em `helpers.rs` de `"0.1.0"` para `"0.2.0-dev"`, salva e checa:

```bash
curl -s http://localhost:3000/health | jq .version
# "0.2.0-dev"
```

**Verificar o OSRM local:**

```bash
make osrm-status
# ✅  OSRM online (localhost:5000)
# ou
# ❌  OSRM offline — rode 'make osrm-prepare' se for o primeiro uso
```

### 4. Configurar o VS Code

As configurações do rust-analyzer já estão no `devcontainer.json`. Se precisar sobrescrever localmente, crie `.vscode/settings.json`:

```json
{
  "rust-analyzer.cargo.target": "wasm32-wasip1",
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": ["--", "-D", "warnings"],
  "rust-analyzer.inlayHints.parameterHints.enable": true,
  "rust-analyzer.inlayHints.typeHints.enable": true,
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

O `cargo.target: wasm32-wasip1` é o mais importante: sem ele o rust-analyzer analisa para `x86_64` e esconde erros que só aparecem no WASI.

### Como os volumes de cache funcionam

O `docker-compose.dev.yml` define três volumes nomeados:

| Volume | O que guarda | Efeito sem ele |
|---|---|---|
| `cargo-registry` | índice e tarballs do crates.io | re-baixa ~300 MB a cada rebuild |
| `cargo-git` | deps de repositórios git | re-clona deps git |
| `cargo-target` | artefatos de compilação wasm32 | re-compila tudo do zero (~60s) |

Os volumes sobrevivem a `docker compose down` e a rebuilds do devcontainer. Para limpar tudo:

```bash
docker volume rm cardapio-geo-api_cargo-registry cardapio-geo-api_cargo-git cardapio-geo-api_cargo-target
```

---

## Rotina diária

### Fluxo normal

```bash
# 1. Abre o container (VS Code faz isso automaticamente)
# 2. Sobe o servidor com hot-reload
make dev           # equivalente a Ctrl+Shift+B

# 3. Edita código — rebuild automático a cada save

# 4. Antes de commitar (o git hook faz isso, mas pode rodar manualmente)
make fmt           # formata
make lint          # clippy -D warnings
make test          # testes unitários

# 5. Verifica tudo de uma vez (igual ao CI)
make ci
```

### Comandos disponíveis

```bash
make help          # lista todos os comandos com descrição
```

| Comando | O que faz |
|---|---|
| `make dev` | `spin watch` — hot-reload em `:3000` |
| `make build` | build debug para wasm32 |
| `make release` | build release — gera o `.wasm` final |
| `make fmt` | `cargo fmt --all` |
| `make fmt-check` | verifica formatação sem alterar (usado no CI) |
| `make lint` | `cargo clippy -- -D warnings` |
| `make test` | testes unitários nativos |
| `make test-watch` | testes em modo watch (recompila ao salvar) |
| `make audit` | `cargo-audit` — verifica CVEs |
| `make deny` | `cargo-deny` — verifica licenças e bans |
| `make ci` | roda todos os checks em sequência |
| `make smoke` | testa os 4 endpoints via curl/jq (precisa do servidor rodando) |
| `make deploy` | `spin deploy` → Fermyon Cloud |
| `make osrm-prepare` | baixa e processa dados do Nordeste (1x, ~10 min) |
| `make up` | sobe geo-api + OSRM local via Docker Compose |
| `make down` | derruba a stack |
| `make logs` | tail dos logs do geo-api |

---

## Testes

### Testes unitários

Cobrem a lógica pura (sem rede, sem WASI runtime):

```bash
make test
# ou diretamente:
cargo test --lib -- --nocapture
```

O que está coberto:

- **`bounds.rs`** — classify para Maceió, Arapiraca, Penedo, Delmiro, São Paulo, Fortaleza; invariante AL ⊆ extended; formato viewbox do Nominatim
- **`helpers.rs`** — url_decode para ASCII, `+` como espaço, percent-encoding, UTF-8 multibyte (ex: `%C3%A7` → `ç`), sequência inválida; parse_qs para casos normais, vazio, valor ausente; require_f64 para valor válido, chave ausente, não-numérico

```bash
# Exemplo de output esperado
running 17 tests
test bounds::tests::maceio_within_alagoas ... ok
test bounds::tests::arapiraca_within_alagoas ... ok
test bounds::tests::sao_paulo_outside_alagoas ... ok
test bounds::tests::alagoas_is_subset_of_extended ... ok
test helpers::tests::url_decode_utf8_multibyte ... ok
...
test result: ok. 17 passed; 0 failed
```

### Smoke test manual (servidor rodando)

```bash
make smoke
```

Testa os 4 endpoints em sequência e formata a resposta com `jq`. Saída esperada:

```bash
▶ /health
{ "status": "ok", "version": "0.1.0" }

▶ /geocode?q=Arapiraca
{ "display_name": "Arapiraca, ...", "within_alagoas": true, "within_region": true }

▶ /reverse (Maceió)
{ "display_name": "..., Maceió, Alagoas, ...", "within_alagoas": true }

▶ /route (Maceió → Arapiraca)
{ "ok": true, "distance_km": 128.4, "duration_min": 98.0 }
```

### Testes manuais com curl

```bash
# Health
curl http://localhost:3000/health

# Geocoding — endereço com acento (valida o url_decode)
curl "http://localhost:3000/geocode?q=Rua+S%C3%A3o+Jo%C3%A3o%2C+Arapiraca"

# Geocoding com limit
curl "http://localhost:3000/geocode?q=Maceio&limit=3"

# Reverse geocoding
curl "http://localhost:3000/reverse?lat=-9.7514&lng=-36.6605"

# Rota normal (dentro de AL)
curl "http://localhost:3000/route?from_lat=-9.6658&from_lng=-35.7350&to_lat=-9.7522&to_lng=-36.6613"

# Rota fora da área (espera out_of_range: true)
curl "http://localhost:3000/route?from_lat=-9.6658&from_lng=-35.7350&to_lat=-23.55&to_lng=-46.63"

# Parâmetro ausente (espera HTTP 400)
curl -i "http://localhost:3000/route?from_lat=-9.6658"

# CORS preflight
curl -i -X OPTIONS http://localhost:3000/geocode \
  -H "Origin: http://localhost:8080" \
  -H "Access-Control-Request-Method: GET"
```

---

## CI/CD

### Pipeline

O CI roda automaticamente em todo push para `main` e `develop`, e em todo Pull Request para `main`.

```
push / PR
    │
    ├── fmt-check      verifica formatação (rustfmt)
    ├── clippy         linter com -D warnings (target: wasm32-wasip1)
    ├── test           testes unitários (target nativo)
    ├── build          cargo build --release → sobe o .wasm como artifact
    ├── audit          cargo-audit — CVEs nas dependências
    └── deny           cargo-deny — licenças, bans, advisories
```

Os 6 jobs rodam em paralelo. O job `build` salva o `.wasm` como artifact do GitHub Actions por 7 dias.

### Deploy

O deploy roda **automaticamente** quando um commit chega na `main` — mas só depois que todos os 6 jobs do CI passarem.

```
main
    │
    └── ci (gate) ── todos os 6 jobs ──► deploy
                                              │
                                              ├── spin deploy → Fermyon Cloud
                                              └── smoke test no /health da URL retornada
```

Para habilitar o deploy automático, você precisa de um secret no repositório:

1. Gere o token: [cloud.fermyon.com/user-settings](https://cloud.fermyon.com/user-settings)
2. No GitHub: **Settings → Secrets and variables → Actions → New repository secret**
3. Nome: `FERMYON_CLOUD_TOKEN`, valor: o token gerado

### Deploy manual

```bash
# Dentro do dev container
make login     # abre browser para autenticação (só na primeira vez)
make deploy    # build release + spin deploy
```

A URL de produção vai aparecer no output:

```
Deployed cardapio-geo-api version 0.1.0+XXXXXXXX
Available Routes:
  geo: https://cardapio-geo-api.seu-usuario.fermyon.app (wildcard)
```

### Dependências automáticas

O Dependabot abre PRs toda segunda-feira com atualizações de deps Rust e GitHub Actions. Minor e patch são automáticos. Major precisa de aprovação manual.

Os PRs de deps passam pelo CI completo antes de aparecer para revisão — se o `cargo-audit` ou `cargo-deny` reprovar a nova versão, o PR já chega marcado como falho.

---

## Endpoints

| Método | Rota | Descrição |
|---|---|---|
| GET | `/geocode?q=Rua+X,+Arapiraca[&limit=5]` | Forward geocoding (Nominatim) |
| GET | `/reverse?lat=-9.75&lng=-36.66` | Reverse geocoding (Nominatim) |
| GET | `/route?from_lat=&from_lng=&to_lat=&to_lng=` | Rota de carro (OSRM) |
| GET | `/health` | Status e versão |
| OPTIONS | `/*` | CORS preflight |

Todos os endpoints retornam `Content-Type: application/json; charset=utf-8` e headers CORS (`Access-Control-Allow-Origin: *`).

### `/geocode`

```json
[
  {
    "place_id": 123456,
    "lat": "-9.7514",
    "lon": "-36.6605",
    "display_name": "Rua Comendador Leão, Centro, Arapiraca, Alagoas, Brasil",
    "address": {
      "road": "Rua Comendador Leão",
      "suburb": "Centro",
      "city": "Arapiraca",
      "state": "Alagoas",
      "postcode": "57300-140"
    },
    "within_alagoas": true,
    "within_region": true
  }
]
```

### `/route`

```json
{
  "ok": true,
  "out_of_range": false,
  "distance_km": 5.3,
  "duration_min": 12.0,
  "within_alagoas": true,
  "within_region": true,
  "routes": [{ "distance": 5300.0, "duration": 720.0 }]
}
```

`routes[0].distance` em **metros** — idêntico ao OSRM bruto. Código que já lê `routes[0].distance / 1000` funciona sem alteração.

Destino fora da área:

```json
{
  "ok": false,
  "out_of_range": true,
  "distance_km": null,
  "duration_min": null,
  "routes": [],
  "message": "destino fora da área de entrega"
}
```

### Erros

Todos os erros seguem o mesmo schema:

```json
{ "error": true, "message": "parâmetro 'lat' ausente" }
```

| Status | Quando |
|---|---|
| 400 | Parâmetro ausente ou inválido |
| 404 | Rota não existe / endereço não encontrado (reverse) |
| 405 | Método diferente de GET |
| 422 | Coordenadas fora da região operacional |
| 500 | Erro interno inesperado |
| 502 | Nominatim ou OSRM retornou erro |

---

## Produção

### Docker Compose (self-hosted)

```bash
docker compose up --build
# API em http://localhost:3000
```

A imagem final tem ~80 MB e não inclui ferramentas de desenvolvimento.

### Fermyon Cloud (gratuito)

```bash
# Dentro do dev container
make login
make deploy
```

### Substituindo por servidores self-hosted

Para produção com volume, use instâncias próprias de Nominatim e OSRM com o extrato do Nordeste.

---

## Infra self-hosted (OSRM)

### Primeira vez — prepara os dados (~10 min, roda uma vez)

```bash
make osrm-prepare
# ou: docker compose --profile prepare up osrm-prepare
```

Baixa o extrato do Nordeste (~120 MB), roda `extract → partition → customize` e salva no volume Docker `osrm-data`. Não precisa repetir mesmo que o container seja removido.

### Uso normal

```bash
make up    # sobe geo-api + OSRM em background
make logs  # acompanha os logs
make down  # derruba tudo
```

### Variável de ambiente do OSRM

| Contexto | `SPIN_VARIABLE_OSRM_BASE` |
|---|---|
| Docker Compose (padrão) | `http://osrm:5000/route/v1/driving` |
| Dev container local | `http://localhost:5000/route/v1/driving` |
| Sem configuração | `https://router.project-osrm.org/route/v1/driving` |

Para usar OSRM local no dev container:

```bash
export SPIN_VARIABLE_OSRM_BASE="http://localhost:5000/route/v1/driving"
spin watch
```