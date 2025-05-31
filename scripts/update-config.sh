#!/bin/bash
# Update config script for Magic Mouse Gesture Recognition
# - Replaces the config file with a new one
# - Restarts the service if it was running

set -e

SERVICE_NAME="mouse-gesture.service"
CONFIG_PATH="$HOME/.config/mouse-gesture/config.json"
NEW_CONFIG_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/config.json"

# Check if service is running
echo "Checking if $SERVICE_NAME is running..."
if systemctl --user is-active --quiet "$SERVICE_NAME"; then
    SERVICE_WAS_RUNNING=1
    echo "$SERVICE_NAME is running. Stopping it for config update..."
    systemctl --user stop "$SERVICE_NAME"
else
    SERVICE_WAS_RUNNING=0
    echo "$SERVICE_NAME is not running."
fi

# Update config
mkdir -p "$(dirname "$CONFIG_PATH")"
cp "$NEW_CONFIG_PATH" "$CONFIG_PATH"
echo "Config updated at $CONFIG_PATH."

# Restart service if it was running
if [ "$SERVICE_WAS_RUNNING" -eq 1 ]; then
    echo "Restarting $SERVICE_NAME..."
    systemctl --user start "$SERVICE_NAME"
    echo "$SERVICE_NAME restarted."
fi
