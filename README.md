# cardapio-geo-api

Componente **WASI/Rust** que expõe uma API de geocoding + roteamento OSM
focada em **Alagoas, BR**. Roda via [Spin (Fermyon)](https://developer.fermyon.com/spin/v2).

---

## Pré-requisitos

- [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- [VS Code](https://code.visualstudio.com/) + extensão [Dev Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)

> O Spin CLI não tem binário nativo para Windows. O dev container resolve isso —
> você desenvolve dentro de um Linux gerenciado pelo Docker, transparente no VS Code.

---

## Desenvolvimento (dev container)

### 1. Abrir no container

```
Ctrl+Shift+P → Dev Containers: Reopen in Container
```

O VS Code vai:
1. Baixar a imagem `mcr.microsoft.com/devcontainers/rust:1-bookworm`
2. Rodar `.devcontainer/setup.sh` — instala `wasm32-wasip1`, Spin CLI e cargo tools
3. Instalar as extensões (`rust-analyzer`, LLDB, TOML, etc.)
4. Abrir o terminal já dentro do Linux

Isso acontece **uma vez**. Nas próximas vezes o container já está pronto.

### 2. Iniciar o servidor

```
Ctrl+Shift+B
```

Dispara o `spin watch` — compila e sobe a API em `http://localhost:3000` com hot-reload.
Qualquer alteração em `src/` recompila e reinicia automaticamente.

### 3. Testar

No terminal dentro do container (ou no PowerShell do Windows — a porta 3000 é redirecionada):

```bash
curl http://localhost:3000/health
curl "http://localhost:3000/geocode?q=Arapiraca"
curl "http://localhost:3000/reverse?lat=-9.7514&lng=-36.6605"
curl "http://localhost:3000/route?from_lat=-9.7514&from_lng=-36.6605&to_lat=-9.80&to_lng=-36.70"
```

### Outras tasks (`Ctrl+Shift+P` → Tasks: Run Task)

| Task | O que faz |
|------|-----------|
| `spin watch` | Build + servidor hot-reload (padrão, `Ctrl+Shift+B`) |
| `cargo clippy` | Linter — avisos além do compilador |
| `cargo fmt` | Formata todo o código |
| `cargo build (release)` | Gera o `.wasm` final |
| `cargo audit` | Verifica CVEs nas dependências |

---

## Estrutura do projeto

```
cardapio-geo-api/
├── .devcontainer/
│   ├── devcontainer.json   # imagem, extensões, porta, postCreateCommand
│   └── setup.sh            # instala wasm32-wasip1, Spin CLI, git hook
├── .vscode/
│   ├── tasks.json          # Ctrl+Shift+B → spin watch
│   └── extensions.json     # extensões recomendadas
├── src/
│   ├── lib.rs              # entry point WASI, roteamento por path
│   ├── bounds.rs           # bbox Alagoas, fn classify()
│   ├── helpers.rs          # parse_qs, json_ok, error_json, CORS
│   ├── geocode.rs          # GET /geocode  → Nominatim
│   ├── reverse.rs          # GET /reverse  → Nominatim
│   └── route.rs            # GET /route    → OSRM
├── Cargo.toml
├── spin.toml               # manifesto Spin v2
├── rust-toolchain.toml     # trava Rust stable + target
├── Dockerfile              # imagem de produção (~80 MB)
└── docker-compose.yml      # sobe a imagem de produção
```

---

## Endpoints

| Método | Rota | Descrição |
|--------|------|-----------|
| GET | `/geocode?q=Rua+X,+Arapiraca[&limit=5]` | Forward geocoding (Nominatim) |
| GET | `/reverse?lat=-9.75&lng=-36.66` | Reverse geocoding (Nominatim) |
| GET | `/route?from_lat=&from_lng=&to_lat=&to_lng=` | Rota de carro (OSRM) |
| GET | `/health` | Status / versão |
| OPTIONS | `/*` | CORS preflight |

### `/geocode` — resposta

Array Nominatim `jsonv2` + campos extras:

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

### `/route` — resposta

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

> `routes[0].distance` em **metros** — idêntico ao OSRM bruto.
> Código que já lê `routes[0].distance / 1000` funciona sem alteração.

Destino fora da área:

```json
{ "ok": false, "out_of_range": true, "distance_km": null, "routes": [] }
```

---

## Produção

### Subir com Docker Compose

```bash
docker compose up --build
# API em http://localhost:3000 — imagem final ~80 MB, sem ferramentas de dev
```

### Deploy gratuito (Fermyon Cloud)

Dentro do dev container:

```bash
spin login    # abre browser para autenticação
spin deploy   # retorna: https://cardapio-geo-api.SEU-USUARIO.fermyon.app
```

---

## Substituindo por servidores self-hosted

Para produção, troque os upstreams por instâncias próprias de Nominatim e OSRM
com o extrato do Nordeste (~120 MB).

**OSRM:**
```bash
wget https://download.geofabrik.de/south-america/brazil/nordeste-latest.osm.pbf

docker run -t -v $(pwd):/data osrm/osrm-backend \
  osrm-extract -p /opt/car.lua /data/nordeste-latest.osm.pbf
docker run -t -v $(pwd):/data osrm/osrm-backend osrm-partition /data/nordeste-latest.osrm
docker run -t -v $(pwd):/data osrm/osrm-backend osrm-customize /data/nordeste-latest.osrm
docker run -d -p 5000:5000 -v $(pwd):/data \
  osrm/osrm-backend osrm-routed --algorithm mld /data/nordeste-latest.osrm
```

Depois em `src/route.rs`:
```rust
const OSRM_BASE: &str = "http://localhost:5000/route/v1/driving";
```

E em `spin.toml`:
```toml
allowed_outbound_hosts = [
  "http://localhost:5000",
  "https://nominatim.openstreetmap.org",
]
```

---

## Integração com o address.js

Veja [`INTEGRATION.md`](./INTEGRATION.md) — são 3 linhas de mudança.

---

## OSRM self-hosted (docker compose)

O `docker-compose.yml` inclui um servidor OSRM local com os dados do Nordeste.
Isso elimina a dependência da API pública `router.project-osrm.org` e remove
o limite de requisições.

### Primeira vez — prepara os dados (~10 min, roda uma vez)

```bash
docker compose --profile prepare up osrm-prepare
```

O que acontece:
1. Baixa o extrato do Nordeste do Geofabrik (~120 MB)
2. Roda `osrm-extract` → `osrm-partition` → `osrm-customize`
3. Salva os dados no volume Docker `osrm-data`

Os dados ficam no volume — não precisa repetir mesmo que o container seja removido.

### Uso normal — sobe tudo

```bash
docker compose up -d --build
```

Isso sobe dois serviços:
- `osrm` — servidor de rotas na porta 5000 (interno)
- `geo-api` — API Spin na porta 3000, apontando para o OSRM local

O `geo-api` só sobe depois que o `osrm` passar no healthcheck.

### Variável de ambiente

O endereço do OSRM é configurado via `SPIN_VARIABLE_OSRM_BASE`:

| Contexto | Valor |
|----------|-------|
| docker compose (padrão) | `http://osrm:5000/route/v1/driving` |
| dev container local | `http://localhost:5000/route/v1/driving` |
| API pública (sem configuração) | `https://router.project-osrm.org/route/v1/driving` |

Para usar o OSRM local no dev container, adicione no terminal:
```bash
export SPIN_VARIABLE_OSRM_BASE="http://localhost:5000/route/v1/driving"
spin watch
```