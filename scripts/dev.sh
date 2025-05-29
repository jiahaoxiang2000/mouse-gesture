#!/bin/bash
# Magic Mouse Gesture Recognition - Development Helper Script

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get project directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

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

show_usage() {
    echo -e "${BLUE}Magic Mouse Gesture Recognition - Development Helper${NC}"
    echo "====================================================="
    echo ""
    echo "Usage: $0 {build|run|test|clean|check|format|install-dev|uninstall-dev}"
    echo ""
    echo "Commands:"
    echo "  build       - Build the project in debug mode"
    echo "  run         - Build and run the project"
    echo "  test        - Run all tests"
    echo "  clean       - Clean build artifacts"
    echo "  check       - Run cargo check and clippy"
    echo "  format      - Format code with rustfmt"
    echo "  install-dev - Install to system for development"
    echo "  uninstall-dev - Remove development installation"
    echo ""
}

build_project() {
    print_status "Building project in debug mode..."
    cd "$PROJECT_DIR"
    cargo build
}

run_project() {
    print_status "Building and running project..."
    cd "$PROJECT_DIR"
    
    # Check if config exists
    if [[ ! -f "config.json" ]]; then
        print_warning "No config.json found, creating default..."
        cargo run -- --help > /dev/null 2>&1 || true
    fi
    
    echo ""
    echo "Available options:"
    echo "  --verbose    : Enable debug logging"
    echo "  --check-deps : Check system dependencies"
    echo "  --device PATH: Specify device path"
    echo ""
    
    # Run with common development flags
    cargo run -- --verbose --check-deps
}

test_project() {
    print_status "Running tests..."
    cd "$PROJECT_DIR"
    cargo test
}

clean_project() {
    print_status "Cleaning build artifacts..."
    cd "$PROJECT_DIR"
    cargo clean
    print_status "Clean completed"
}

check_project() {
    print_status "Running cargo check..."
    cd "$PROJECT_DIR"
    cargo check
    
    print_status "Running clippy..."
    cargo clippy -- -W clippy::all
    
    print_status "Checking formatting..."
    cargo fmt -- --check || {
        print_warning "Code formatting issues found. Run '$0 format' to fix."
    }
}

format_project() {
    print_status "Formatting code..."
    cd "$PROJECT_DIR"
    cargo fmt
    print_status "Code formatted"
}

install_dev() {
    print_status "Installing development version..."
    cd "$PROJECT_DIR"
    
    # Build in debug mode for faster compilation
    cargo build
    
    # Install with dev suffix to avoid conflicts
    sudo cp "target/debug/mouse-gesture-recognition" "/usr/local/bin/mouse-gesture-recognition-dev"
    sudo chmod +x "/usr/local/bin/mouse-gesture-recognition-dev"
    
    print_status "Development binary installed as 'mouse-gesture-recognition-dev'"
    echo "Run with: mouse-gesture-recognition-dev --verbose"
}

uninstall_dev() {
    print_status "Removing development installation..."
    
    if [[ -f "/usr/local/bin/mouse-gesture-recognition-dev" ]]; then
        sudo rm -f "/usr/local/bin/mouse-gesture-recognition-dev"
        print_status "Development binary removed"
    else
        print_warning "Development binary not found"
    fi
}

# Main function
main() {
    case "${1:-}" in
        build)
            build_project
            ;;
        run)
            run_project
            ;;
        test)
            test_project
            ;;
        clean)
            clean_project
            ;;
        check)
            check_project
            ;;
        format)
            format_project
            ;;
        install-dev)
            install_dev
            ;;
        uninstall-dev)
            uninstall_dev
            ;;
        *)
            show_usage
            exit 1
            ;;
    esac
}

main "$@"
