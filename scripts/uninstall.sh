#!/bin/bash
# Magic Mouse Gesture Recognition - Uninstall Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BINARY_NAME="mouse-gesture-recognition"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="$HOME/.config/mouse-gesture"
SERVICE_DIR="$HOME/.config/systemd/user"
UDEV_DIR="/etc/udev/rules.d"

echo -e "${BLUE}Magic Mouse Gesture Recognition - Uninstall Script${NC}"
echo "======================================================"

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    echo -e "${RED}Error: This script should not be run as root${NC}"
    echo "Please run as a regular user. The script will use sudo when needed."
    exit 1
fi

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

# Stop and disable service
stop_service() {
    print_status "Stopping and disabling service..."
    
    # Stop user service if running
    if systemctl --user is-active mouse-gesture.service &>/dev/null; then
        systemctl --user stop mouse-gesture.service
        print_status "Service stopped"
    fi
    
    # Disable user service if enabled
    if systemctl --user is-enabled mouse-gesture.service &>/dev/null; then
        systemctl --user disable mouse-gesture.service
        print_status "Service disabled"
    fi
}

# Remove systemd service
remove_service() {
    print_status "Removing systemd user service..."
    
    if [[ -f "$SERVICE_DIR/mouse-gesture.service" ]]; then
        rm -f "$SERVICE_DIR/mouse-gesture.service"
        systemctl --user daemon-reload
        print_status "Systemd user service removed"
    else
        print_warning "Systemd user service file not found"
    fi
}

# Remove udev rules
remove_udev_rules() {
    print_status "Removing udev rules..."
    
    if [[ -f "$UDEV_DIR/99-magic-mouse.rules" ]]; then
        sudo rm -f "$UDEV_DIR/99-magic-mouse.rules"
        sudo udevadm control --reload-rules
        sudo udevadm trigger
        print_status "udev rules removed"
    else
        print_warning "udev rules file not found"
    fi
}

# Remove configuration
remove_config() {
    print_status "Removing configuration..."
    
    if [[ -d "$CONFIG_DIR" ]]; then
        echo -e "${YELLOW}Configuration directory found: $CONFIG_DIR${NC}"
        read -p "Remove configuration directory and all settings? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$CONFIG_DIR"
            print_status "Configuration removed"
        else
            print_warning "Configuration preserved"
        fi
    else
        print_warning "Configuration directory not found"
    fi
}

# Remove binary
remove_binary() {
    print_status "Removing binary..."
    
    if [[ -f "$INSTALL_DIR/$BINARY_NAME" ]]; then
        sudo rm -f "$INSTALL_DIR/$BINARY_NAME"
        print_status "Binary removed"
    else
        print_warning "Binary not found in $INSTALL_DIR"
    fi
}

# Remove user from input group (optional)
remove_user_groups() {
    print_status "Checking user group membership..."
    
    if groups "$USER" | grep -q "input"; then
        echo -e "${YELLOW}User $USER is in 'input' group${NC}"
        read -p "Remove user from 'input' group? This may affect other applications. (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            sudo gpasswd -d "$USER" input
            print_warning "User removed from 'input' group. Please log out and back in for changes to take effect."
        else
            print_warning "User remains in 'input' group"
        fi
    else
        print_status "User not in 'input' group"
    fi
}

# Clean up any remaining processes
cleanup_processes() {
    print_status "Checking for running processes..."
    
    if pgrep -f "$BINARY_NAME" > /dev/null; then
        print_warning "Found running $BINARY_NAME processes"
        read -p "Kill running processes? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            pkill -f "$BINARY_NAME" || true
            print_status "Processes terminated"
        fi
    else
        print_status "No running processes found"
    fi
}

# Main uninstallation
main() {
    echo "This will remove Magic Mouse Gesture Recognition from your system."
    echo ""
    read -p "Continue with uninstallation? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Uninstallation cancelled."
        exit 0
    fi
    
    cleanup_processes
    stop_service
    remove_service
    remove_udev_rules
    remove_binary
    remove_config
    remove_user_groups
    
    echo ""
    echo -e "${GREEN}Uninstallation completed successfully!${NC}"
    echo ""
    echo "The following may still remain on your system:"
    echo "- Build artifacts in the project directory"
    echo "- User configuration in ~/.config/mouse-gesture/ (if any)"
    echo "- Log entries in journalctl"
    echo ""
    echo "These can be removed manually if desired."
}

main "$@"
