#!/bin/bash
# Persistent server runner with auto-restart on crash

SERVER_DIR="$(cd "$(dirname "$0")" && pwd)"
LOG_FILE="$SERVER_DIR/server.log"
RESTART_DELAY=2

cd "$SERVER_DIR" || exit 1

# Build first
echo "Building server..."
cargo build --release || exit 1

echo "Starting server with auto-restart..."
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
