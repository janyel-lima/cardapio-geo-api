#!/usr/bin/env bash
# scripts/osrm-prepare.sh
#
# Alternativa ao profile "prepare" do docker compose — útil se você
# quiser preparar os dados fora do Docker (ex: em um servidor com mais RAM).
#
# USO:
#   bash scripts/osrm-prepare.sh
#   bash scripts/osrm-prepare.sh /caminho/para/nordeste.osm.pbf   # PBF local
set -euo pipefail

DATA_DIR="$(pwd)/osrm-data"
PBF_URL="https://download.geofabrik.de/south-america/brazil/nordeste-latest.osm.pbf"
PBF="$DATA_DIR/nordeste-latest.osm.pbf"

mkdir -p "$DATA_DIR"

# Se passado um PBF local, copia para o diretório de dados
if [ "${1:-}" != "" ] && [ -f "$1" ]; then
  echo "→ Usando PBF local: $1"
  cp "$1" "$PBF"
fi

# Download se necessário
if [ ! -f "$PBF" ]; then
  echo "→ Baixando extrato do Nordeste (~120 MB)…"
  wget -q --show-progress -O "$PBF" "$PBF_URL"
else
  echo "→ PBF já existe: $PBF"
fi

if [ -f "$DATA_DIR/nordeste-latest.osrm" ]; then
  echo "→ Dados já processados em $DATA_DIR — nada a fazer."
  echo "   Delete $DATA_DIR/nordeste-latest.osrm para reprocessar."
  exit 0
fi

echo "→ osrm-extract…"
docker run --rm -v "$DATA_DIR:/data" osrm/osrm-backend \
  osrm-extract -p /opt/car.lua /data/nordeste-latest.osm.pbf

echo "→ osrm-partition…"
docker run --rm -v "$DATA_DIR:/data" osrm/osrm-backend \
  osrm-partition /data/nordeste-latest.osrm

echo "→ osrm-customize…"
docker run --rm -v "$DATA_DIR:/data" osrm/osrm-backend \
  osrm-customize /data/nordeste-latest.osrm

echo ""
echo "✅  Dados prontos em $DATA_DIR"
echo "   Suba a stack com: docker compose up -d"