#!/usr/bin/env bash
# Syncs local Mercado Livre bot products to the remote Procurali VPS over a restricted
# physical-key Ed25519 SSH tunnel (port forwarding only, nologin on the server).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SECRETS_DIR="${PROCURALI_BOT_SECRETS_DIR:-$REPO_ROOT/.secrets/procurali-local-bot}"
VPS_HOST="${PROCURALI_VPS_HOST:-72.60.147.2}"
DEFAULT_INPUT="/home/iiii/PESSOAL-PROJETOS-ALEXANDRE/chatgpt-recorder/.local/commercial/mercadolivre/products.json"

INPUT_FILE="${PROCURALI_ML_INPUT:-$DEFAULT_INPUT}"
CATEGORY="hardware-e-perifericos"
CHANNEL="@procurali"
POLL_SECONDS=5
MODE="once"
QUERY_REF=""
PRODUCT_ID=""
REFRESH_REF=0

usage() {
  cat <<'EOF'
Uso: scripts/procurali-local-sync.sh [--once | --watch | --sync-reference | --query-ref <texto>] [--product-id <MLB...>] [--refresh] [--input <products.json>] [--category <slug>] [--channel <@canal>] [--poll-seconds <n>]

Abre o túnel SSH restrito por chave física para a VPS e envia ofertas do bot local
para o banco/Hub, disparando atualização do site, fila do Telegram e rascunhos do WhatsApp.
Tambem sincroniza/consulta a base local rapida (`vps-reference.json`) com historico e resumo de precos.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --once)
      MODE="once"
      shift
      ;;
    --watch)
      MODE="watch"
      shift
      ;;
    --sync-reference)
      MODE="sync-ref"
      shift
      ;;
    --query-ref)
      MODE="query-ref"
      QUERY_REF="${2:-}"
      shift 2
      ;;
    --product-id)
      MODE="query-ref"
      PRODUCT_ID="${2:?missing --product-id value}"
      shift 2
      ;;
    --refresh)
      REFRESH_REF=1
      shift
      ;;
    --input)
      INPUT_FILE="${2:?missing --input path}"
      shift 2
      ;;
    --category)
      CATEGORY="${2:?missing --category slug}"
      shift 2
      ;;
    --channel)
      CHANNEL="${2:?missing --channel value}"
      shift 2
      ;;
    --poll-seconds)
      POLL_SECONDS="${2:?missing --poll-seconds value}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Argumento desconhecido: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

KEY_FILE="$SECRETS_DIR/procurali-bot.ed25519"
KNOWN_HOSTS="$SECRETS_DIR/known_hosts"
TOKEN_FILE="$SECRETS_DIR/procurali-bot.token"
CONFIG_FILE="$SECRETS_DIR/worker-local.toml"
STATE_FILE="$SECRETS_DIR/last-products.sha256"
WA_DRAFTS_FILE="$SECRETS_DIR/whatsapp-assisted-drafts.json"
REF_FILE="$SECRETS_DIR/vps-reference.json"

for required in "$KEY_FILE" "$KNOWN_HOSTS" "$TOKEN_FILE" "$CONFIG_FILE"; do
  if [ ! -f "$required" ]; then
    echo "Arquivo obrigatório ausente: $required" >&2
    exit 1
  fi
done

if [ -x "$REPO_ROOT/target/release/zaturo-worker" ]; then
  WORKER_BIN="$REPO_ROOT/target/release/zaturo-worker"
elif [ -x "$REPO_ROOT/target/debug/zaturo-worker" ]; then
  WORKER_BIN="$REPO_ROOT/target/debug/zaturo-worker"
else
  echo "Binário zaturo-worker não encontrado; compile com: cargo build --release -p zaturo-worker" >&2
  exit 1
fi

run_query_ref() {
  local args=(--config "$CONFIG_FILE" commerce query-reference --file "$REF_FILE" --limit 20)
  if [ -n "$QUERY_REF" ]; then
    args+=(--query "$QUERY_REF")
  fi
  if [ -n "$PRODUCT_ID" ]; then
    args+=(--product-id "$PRODUCT_ID")
  fi
  "$WORKER_BIN" "${args[@]}"
}

# Consulta puramente local em milissegundos quando a base em cache já existe (sem abrir SSH).
if [ "$MODE" = "query-ref" ] && [ "$REFRESH_REF" -eq 0 ] && [ -f "$REF_FILE" ]; then
  run_query_ref
  exit 0
fi

LOCAL_HUB_PORT="${PROCURALI_LOCAL_HUB_PORT:-27878}"
LOCAL_MINIO_PORT="${PROCURALI_LOCAL_MINIO_PORT:-29000}"

TUNNEL_PID=""
cleanup() {
  if [ -n "$TUNNEL_PID" ] && kill -0 "$TUNNEL_PID" 2>/dev/null; then
    kill "$TUNNEL_PID" 2>/dev/null || true
    wait "$TUNNEL_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

ssh -N -T \
  -o BatchMode=yes \
  -o IdentitiesOnly=yes \
  -o ExitOnForwardFailure=yes \
  -o ServerAliveInterval=15 \
  -o ServerAliveCountMax=3 \
  -o StrictHostKeyChecking=yes \
  -o UserKnownHostsFile="$KNOWN_HOSTS" \
  -i "$KEY_FILE" \
  -L "127.0.0.1:${LOCAL_HUB_PORT}:127.0.0.1:7878" \
  -L "127.0.0.1:${LOCAL_MINIO_PORT}:127.0.0.1:9000" \
  "procurali-link@${VPS_HOST}" &
TUNNEL_PID=$!

for _ in $(seq 1 40); do
  if ! kill -0 "$TUNNEL_PID" 2>/dev/null; then
    echo "Falha ao abrir túnel SSH seguro com procurali-link@${VPS_HOST}" >&2
    exit 1
  fi
  if curl -sf "http://127.0.0.1:${LOCAL_HUB_PORT}/healthz" >/dev/null 2>&1 \
    && python3 -c "import socket,sys; s=socket.create_connection(('127.0.0.1', int(sys.argv[1])), 1); s.close()" "$LOCAL_MINIO_PORT" 2>/dev/null; then
    break
  fi
  sleep 0.25
done

if ! kill -0 "$TUNNEL_PID" 2>/dev/null \
  || ! curl -sf "http://127.0.0.1:${LOCAL_HUB_PORT}/healthz" >/dev/null 2>&1; then
  echo "Túnel não respondeu em http://127.0.0.1:${LOCAL_HUB_PORT}/healthz" >&2
  exit 1
fi

export ZATURO_WORKER_TOKEN
ZATURO_WORKER_TOKEN="$(tr -d '\r\n' < "$TOKEN_FILE")"

if [ "$MODE" = "sync-ref" ]; then
  "$WORKER_BIN" --config "$CONFIG_FILE" commerce sync-reference --output "$REF_FILE"
  exit 0
fi

if [ "$MODE" = "query-ref" ]; then
  "$WORKER_BIN" --config "$CONFIG_FILE" commerce sync-reference --output "$REF_FILE" >/dev/null
  run_query_ref
  exit 0
fi

SOURCE_BASE_DIR="$(dirname "$INPUT_FILE")"

CMD=(
  "$WORKER_BIN"
  --config "$CONFIG_FILE"
  commerce watch-ml
  --input "$INPUT_FILE"
  --state-file "$STATE_FILE"
  --category "$CATEGORY"
  --channel "$CHANNEL"
  --source-base-dir "$SOURCE_BASE_DIR"
  --whatsapp-drafts-out "$WA_DRAFTS_FILE"
  --reference-out "$REF_FILE"
  --poll-seconds "$POLL_SECONDS"
)
if [ "$MODE" = "once" ]; then
  CMD+=(--once)
fi

"${CMD[@]}"
