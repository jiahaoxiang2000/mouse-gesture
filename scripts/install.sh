#!/bin/bash
# Magic Mouse Gesture Recognition - Installation Script

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

# Get the directory of this script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "${BLUE}Magic Mouse Gesture Recognition - Installation Script${NC}"
echo "=================================================="

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

# Check dependencies
check_dependencies() {
    print_status "Checking dependencies..."
    
    # Check for Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    # Check for required system tools
    local missing_deps=()
    
    if ! command -v wtype &> /dev/null; then
        missing_deps+=("wtype")
    fi

    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        print_warning "Optional dependencies missing: ${missing_deps[*]}"
        echo "Install with your package manager:"
        echo "  Arch Linux: sudo pacman -S ${missing_deps[*]}"
        echo "  Ubuntu/Debian: sudo apt install ${missing_deps[*]}"
        echo ""
        read -p "Continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
}

# Build the project
build_project() {
    print_status "Building project in release mode..."
    cd "$PROJECT_DIR"
    cargo build --release
    
    if [[ ! -f "target/release/$BINARY_NAME" ]]; then
        print_error "Build failed - binary not found"
        exit 1
    fi
}

# Install binary
install_binary() {
    print_status "Installing binary to $INSTALL_DIR..."
    sudo cp "$PROJECT_DIR/target/release/$BINARY_NAME" "$INSTALL_DIR/"
    sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"
    print_status "Binary installed successfully"
}

# Install configuration
install_config() {
    print_status "Setting up configuration directory..."
    mkdir -p "$CONFIG_DIR"
    
    if [[ -f "$PROJECT_DIR/config.json" ]]; then
        cp "$PROJECT_DIR/config.json" "$CONFIG_DIR/config.json"
        print_status "Default configuration installed"
    else
        print_warning "No default config found - will be created on first run"
    fi
    
    # Set appropriate permissions
    chmod 644 "$CONFIG_DIR/config.json" 2>/dev/null || true
}

# Install udev rules
install_udev_rules() {
    print_status "Installing udev rules for device access..."
    
    cat << 'EOF' | sudo tee "$UDEV_DIR/99-magic-mouse.rules" > /dev/null
# Magic Mouse udev rules for gesture recognition
# Allows members of 'input' group to access Magic Mouse devices
SUBSYSTEM=="input", ATTRS{name}=="*Magic Mouse*", MODE="0664", GROUP="input"
SUBSYSTEM=="input", ATTRS{name}=="*Magic Trackpad*", MODE="0664", GROUP="input"
EOF
    
    # Reload udev rules
    sudo udevadm control --reload-rules
    sudo udevadm trigger
    
    print_status "udev rules installed"
}

# Create systemd service
install_service() {
    print_status "Installing systemd user service..."
    
    # Create user systemd directory if it doesn't exist
    mkdir -p "$SERVICE_DIR"
    
    cat << EOF > "$SERVICE_DIR/mouse-gesture.service"
[Unit]
Description=Magic Mouse Gesture Recognition
Documentation=https://github.com/jiahaoxiang2000/mouse-gesture
After=graphical-session.target
Wants=graphical-session.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/$BINARY_NAME --config $CONFIG_DIR/config.json
Restart=always
RestartSec=5
Environment=DISPLAY=:0
Environment=WAYLAND_DISPLAY=wayland-0

# Security settings
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=default.target
EOF
    
    systemctl --user daemon-reload
    print_status "Systemd user service installed"
}

# Setup user groups
setup_user_groups() {
    print_status "Setting up user permissions..."
    
    # Add user to input group
    if ! groups "$USER" | grep -q "input"; then
        sudo usermod -a -G input "$USER"
        print_warning "Added $USER to 'input' group. Please log out and back in for changes to take effect."
    else
        print_status "User already in 'input' group"
    fi
}

# Main installation
main() {
    check_dependencies
    build_project
    install_binary
    install_config
    install_udev_rules
    install_service
    setup_user_groups
    
    echo ""
    echo -e "${GREEN}Installation completed successfully!${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Log out and back in (if you were added to 'input' group)"
    echo "2. Connect and pair your Magic Mouse"
    echo "3. Start the service:"
    echo "   systemctl --user enable mouse-gesture.service"
    echo "   systemctl --user start mouse-gesture.service"
    echo ""
    echo "Configuration file: $CONFIG_DIR/config.json"
    echo "Service logs: journalctl --user -u mouse-gesture.service -f"
    echo ""
    echo "To uninstall, run: $SCRIPT_DIR/uninstall.sh"
}

main "$@"
