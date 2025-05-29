# Magic Mouse Gesture Recognition - Makefile
# Provides easy commands for building, installing, and managing the application

.PHONY: help build test clean install uninstall dev-install dev-uninstall service-start service-stop service-status check format

# Default target
help:
	@echo "Magic Mouse Gesture Recognition - Build System"
	@echo "=============================================="
	@echo ""
	@echo "Available targets:"
	@echo "  build        - Build the project in release mode"
	@echo "  test         - Run all tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  check        - Run checks (cargo check, clippy, format)"
	@echo "  format       - Format code with rustfmt"
	@echo ""
	@echo "Installation:"
	@echo "  install      - Install to system (requires sudo)"
	@echo "  uninstall    - Remove from system (requires sudo)"
	@echo "  dev-install  - Install development version"
	@echo "  dev-uninstall- Remove development version"
	@echo ""
	@echo "Service Management:"
	@echo "  service-start   - Start the service"
	@echo "  service-stop    - Stop the service"
	@echo "  service-restart - Restart the service"
	@echo "  service-status  - Show service status"
	@echo "  service-logs    - Show service logs"
	@echo "  service-enable  - Enable automatic startup"
	@echo "  service-disable - Disable automatic startup"
	@echo ""
	@echo "Development:"
	@echo "  run         - Build and run with verbose output"
	@echo "  debug       - Run with debug logging"
	@echo ""

# Build targets
build:
	cargo build --release

test:
	cargo test

clean:
	cargo clean

check:
	./scripts/dev.sh check

format:
	./scripts/dev.sh format

# Installation targets
install:
	./scripts/install.sh

uninstall:
	./scripts/uninstall.sh

dev-install:
	./scripts/dev.sh install-dev

dev-uninstall:
	./scripts/dev.sh uninstall-dev

# Service management targets
service-start:
	./scripts/service.sh start

service-stop:
	./scripts/service.sh stop

service-restart:
	./scripts/service.sh restart

service-status:
	./scripts/service.sh status

service-logs:
	./scripts/service.sh logs

service-enable:
	./scripts/service.sh enable

service-disable:
	./scripts/service.sh disable

service-edit-config:
	./scripts/service.sh edit-config

# Development targets
run:
	./scripts/dev.sh run

debug:
	RUST_LOG=debug cargo run -- --verbose

# Quick development cycle
dev: format check test

# Complete build and install cycle
release: clean build test install

# Show system information
info:
	@echo "System Information:"
	@echo "=================="
	@echo "Rust version: $$(rustc --version)"
	@echo "Cargo version: $$(cargo --version)"
	@echo "Target directory: $$(pwd)/target"
	@echo "Install directory: /usr/local/bin"
	@echo "Config directory: /etc/mouse-gesture"
	@echo ""
	@echo "Magic Mouse devices:"
	@ls /dev/input/event* | xargs -I {} sh -c 'echo -n "{}: "; cat /sys/class/input/$$(basename {})/device/name 2>/dev/null || echo "unknown"' | grep -i mouse || echo "No Magic Mouse devices found"
