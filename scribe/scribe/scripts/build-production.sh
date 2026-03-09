#!/bin/bash
# Build Scribe (Ghost) desktop installers for production.
# Set these env vars before running, or edit below:
#   APP_ENDPOINT     - scribe-api URL (e.g. https://api.ghost.exora.solutions)
#   PAYMENT_ENDPOINT - same as APP_ENDPOINT
#   API_ACCESS_KEY   - must match server

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# Load .env from src-tauri or project root if vars not already set
if [[ -z "${API_ACCESS_KEY}" ]] && [[ -f src-tauri/.env ]]; then
  set -a
  source src-tauri/.env
  set +a
fi

# Production API URL - change to your scribe-api host
export APP_ENDPOINT="${APP_ENDPOINT:-https://api.ghost.exora.solutions}"
export PAYMENT_ENDPOINT="${PAYMENT_ENDPOINT:-$APP_ENDPOINT}"
export API_ACCESS_KEY="${API_ACCESS_KEY:?Set API_ACCESS_KEY - must match your server}"

# Vite env vars (frontend uses these at build time)
export VITE_APP_ENDPOINT="${APP_ENDPOINT}"
export VITE_API_URL="${APP_ENDPOINT}"
export VITE_GHOST_GATEWAY_WS_URL="${APP_ENDPOINT/http/ws}/gateway"

echo "Building Ghost desktop app for production..."
echo "  APP_ENDPOINT=$APP_ENDPOINT"
echo "  PAYMENT_ENDPOINT=$PAYMENT_ENDPOINT"
echo ""

node node_modules/@tauri-apps/cli/tauri.js build

echo ""
echo "Done. Installers are in: src-tauri/target/release/bundle/"
echo "  - msi/     Windows MSI"
echo "  - nsis/    Windows EXE installer"
echo "  - dmg/     macOS"
echo "  - deb/     Linux Debian/Ubuntu"
echo "  - appimage/ Linux AppImage"
echo ""
echo "Copy the files to admin/admin-ui/public/desktop/ to serve from the website."
