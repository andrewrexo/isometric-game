#!/bin/bash
# Persistent server runner with auto-restart on crash
# Also launches a web server for the WASM client

SERVER_DIR="$(cd "$(dirname "$0")" && pwd)"
WEB_DIR="$SERVER_DIR/../client/web"
LOG_FILE="$SERVER_DIR/server.log"
RESTART_DELAY=2
WEB_PORT=8080

cd "$SERVER_DIR" || exit 1

# Build first
echo "Building server..."
cargo build --release || exit 1

# Start web server for WASM client in background
if [ -d "$WEB_DIR" ]; then
    echo "Starting web server on http://localhost:$WEB_PORT (serving $WEB_DIR)"
    python3 -m http.server "$WEB_PORT" --directory "$WEB_DIR" &
    WEB_PID=$!
else
    echo "Warning: Web directory not found at $WEB_DIR, skipping web server"
    WEB_PID=""
fi

# Clean up web server on exit
cleanup() {
    if [ -n "$WEB_PID" ]; then
        echo "Stopping web server (PID $WEB_PID)..."
        kill "$WEB_PID" 2>/dev/null
    fi
}
trap cleanup EXIT

echo "Starting game server with auto-restart..."
echo "Logs: $LOG_FILE"
echo "Press Ctrl+C to stop"

while true; do
    echo "[$(date)] Starting server..." | tee -a "$LOG_FILE"

    ./target/release/isometric-server 2>&1 | tee -a "$LOG_FILE"

    EXIT_CODE=$?
    echo "[$(date)] Server exited with code $EXIT_CODE" | tee -a "$LOG_FILE"

    if [ $EXIT_CODE -eq 0 ]; then
        echo "Clean shutdown, not restarting."
        break
    fi

    echo "Restarting in ${RESTART_DELAY}s..." | tee -a "$LOG_FILE"
    sleep $RESTART_DELAY
done
