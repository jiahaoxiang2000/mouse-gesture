#!/bin/bash
# Update script for Magic Mouse Gesture Recognition
# - Rebuilds and reinstalls the binary
# - Restarts the service if it was running

set -e

SERVICE_NAME="mouse-gesture.service"
BINARY_NAME="mouse-gesture-recognition"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Check if service is running
echo "Checking if $SERVICE_NAME is running..."
if systemctl --user is-active --quiet "$SERVICE_NAME"; then
    SERVICE_WAS_RUNNING=1
    echo "$SERVICE_NAME is running. Stopping it for update..."
    systemctl --user stop "$SERVICE_NAME"
else
    SERVICE_WAS_RUNNING=0
    echo "$SERVICE_NAME is not running."
fi

# Build and install
cd "$PROJECT_DIR"
cargo build --release
sudo cp target/release/$BINARY_NAME /usr/local/bin/
sudo chmod +x /usr/local/bin/$BINARY_NAME
echo "Binary updated."

# Restart service if it was running
if [ "$SERVICE_WAS_RUNNING" -eq 1 ]; then
    echo "Restarting $SERVICE_NAME..."
    systemctl --user start "$SERVICE_NAME"
    echo "$SERVICE_NAME restarted."
fi
