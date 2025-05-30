#!/bin/bash
# Magic Mouse Gesture Recognition - Service Management Script

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SERVICE_NAME="mouse-gesture.service"

# Function to print status
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Show usage
show_usage() {
    echo -e "${BLUE}Magic Mouse Gesture Recognition - Service Manager${NC}"
    echo "================================================="
    echo ""
    echo "Usage: $0 {start|stop|restart|enable|disable|status|logs|edit-config}"
    echo ""
    echo "Commands:"
    echo "  start       - Start the service"
    echo "  stop        - Stop the service"
    echo "  restart     - Restart the service"
    echo "  enable      - Enable service to start automatically"
    echo "  disable     - Disable automatic startup"
    echo "  status      - Show service status"
    echo "  logs        - Show service logs (follow mode)"
    echo "  edit-config - Edit configuration file"
    echo ""
}

# Start service
start_service() {
    print_status "Starting $SERVICE_NAME..."
    if systemctl --user start "$SERVICE_NAME"; then
        print_status "Service started successfully"
        show_status
    else
        print_error "Failed to start service"
        exit 1
    fi
}

# Stop service
stop_service() {
    print_status "Stopping $SERVICE_NAME..."
    if systemctl --user stop "$SERVICE_NAME"; then
        print_status "Service stopped successfully"
    else
        print_error "Failed to stop service"
        exit 1
    fi
}

# Restart service
restart_service() {
    print_status "Restarting $SERVICE_NAME..."
    if systemctl --user restart "$SERVICE_NAME"; then
        print_status "Service restarted successfully"
        show_status
    else
        print_error "Failed to restart service"
        exit 1
    fi
}

# Enable service
enable_service() {
    print_status "Enabling $SERVICE_NAME for automatic startup..."
    if systemctl --user enable "$SERVICE_NAME"; then
        print_status "Service enabled successfully"
    else
        print_error "Failed to enable service"
        exit 1
    fi
}

# Disable service
disable_service() {
    print_status "Disabling $SERVICE_NAME automatic startup..."
    if systemctl --user disable "$SERVICE_NAME"; then
        print_status "Service disabled successfully"
    else
        print_error "Failed to disable service"
        exit 1
    fi
}

# Show status
show_status() {
    echo ""
    echo -e "${BLUE}Service Status:${NC}"
    systemctl --user status "$SERVICE_NAME" --no-pager || true
    echo ""
    
    # Show if enabled
    if systemctl --user is-enabled "$SERVICE_NAME" &>/dev/null; then
        echo -e "${GREEN}✓ Service is enabled for automatic startup${NC}"
    else
        echo -e "${YELLOW}⚠ Service is not enabled for automatic startup${NC}"
    fi
    
    # Show if active
    if systemctl --user is-active "$SERVICE_NAME" &>/dev/null; then
        echo -e "${GREEN}✓ Service is currently running${NC}"
    else
        echo -e "${RED}✗ Service is not running${NC}"
    fi
    echo ""
}

# Show logs
show_logs() {
    print_status "Showing service logs (Ctrl+C to exit)..."
    journalctl --user -u "$SERVICE_NAME" -f
}

# Edit configuration
edit_config() {
    CONFIG_FILE="$HOME/.config/mouse-gesture/config.json"
    
    if [[ ! -f "$CONFIG_FILE" ]]; then
        print_error "Configuration file not found: $CONFIG_FILE"
        exit 1
    fi
    
    print_status "Opening configuration file for editing..."
    
    # Use preferred editor or fall back to nano
    EDITOR=${EDITOR:-nano}
    "$EDITOR" "$CONFIG_FILE"
    
    print_status "Configuration edited. Restart the service to apply changes:"
    echo "  $0 restart"
}

# Check if service exists
check_service_exists() {
    if ! systemctl --user list-unit-files | grep -q "$SERVICE_NAME"; then
        print_error "Service $SERVICE_NAME not found. Is the application installed?"
        echo "Run the installation script first: ./scripts/install.sh"
        exit 1
    fi
}

# Main function
main() {
    case "${1:-}" in
        start)
            check_service_exists
            start_service
            ;;
        stop)
            check_service_exists
            stop_service
            ;;
        restart)
            check_service_exists
            restart_service
            ;;
        enable)
            check_service_exists
            enable_service
            ;;
        disable)
            check_service_exists
            disable_service
            ;;
        status)
            check_service_exists
            show_status
            ;;
        logs)
            check_service_exists
            show_logs
            ;;
        edit-config)
            edit_config
            ;;
        *)
            show_usage
            exit 1
            ;;
    esac
}

main "$@"
