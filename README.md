# Magic Mouse Gesture Recognition

A Rust-based gesture recognition tool for Apple Magic Mouse on Linux, specifically designed for the Magic Mouse 2 USB-C 2024 model.

## Features

- **Multi-touch gesture recognition**: Detects swipes, scrolls, taps, and pinch gestures
- **Two-finger tap recognition**: Advanced two-finger tap detection based on Linux Multi-Touch Protocol
- **Configurable actions**: Execute custom commands for each gesture
- **Direct input handling**: Works with raw input events from evdev
- **Async processing**: Non-blocking event processing using Tokio
- **Customizable configuration**: JSON-based configuration system

## Supported Gestures

- **2-finger swipes**: Navigate browser history, open/close tabs
- **Scrolling**: Vertical and horizontal scrolling
- **Taps**: Single and double-finger taps for click simulation
- **Pinch**: Zoom in/out functionality
- **Button clicks**: Standard mouse button support

## Prerequisites

### Hardware Requirements

- Apple Magic Mouse 2 USB-C 2024
- Linux system with evdev support

### Software Requirements

- Rust 1.70+ (for building)
- `hid-magicmouse` kernel module (see [setup guide](docs/apple.md))
- `xdotool` (for executing actions)

### Installation of Dependencies

On Arch Linux:

```bash
sudo pacman -S xdotool
```

On Ubuntu/Debian:

```bash
sudo apt install xdotool
```

## Installation

### Automated Installation (Recommended)

The easiest way to install Magic Mouse Gesture Recognition is using the automated installation script:

```bash
git clone <repository-url>
cd mouse-gesture

# Run the installation script
./scripts/install.sh
```

The installation script will:
- Install Rust dependencies and build the project
- Copy the binary to `/usr/local/bin`
- Set up configuration in `~/.config/mouse-gesture/`
- Create udev rules for device access
- Add your user to the `input` group
- Install and configure a systemd user service
- Set up automatic startup

After installation:
```bash
# Start the service immediately
systemctl --user start mouse-gesture.service

# Enable automatic startup on login
systemctl --user enable mouse-gesture.service
```

### Manual Installation

If you prefer to build and configure manually:

1. Clone the repository:

```bash
git clone <repository-url>
cd mouse-gesture
```

2. Build the project:

```bash
cargo build --release
```

### Uninstallation

To completely remove the application:

```bash
./scripts/uninstall.sh
```

## Configuration

The application will create a default configuration file `config.json` on first run. You can customize gestures and actions by editing this file.

### Example Configuration

```json
{
  "device": {
    "path": null,
    "auto_detect": true,
    "name_pattern": "Magic Mouse"
  },
  "gesture": {
    "scroll_threshold": 50.0,
    "swipe_threshold": 100.0,
    "pinch_threshold": 0.1,
    "tap_timeout_ms": 300,
    "debounce_ms": 100
  },
  "actions": {
    "swipe_left_2finger": "xdotool key alt+Right",
    "swipe_right_2finger": "xdotool key alt+Left",
    "swipe_up_2finger": "xdotool key ctrl+t",
    "swipe_down_2finger": "xdotool key ctrl+w",
    "scroll_vertical": "scroll_vertical",
    "scroll_horizontal": "scroll_horizontal",
    "tap_1finger": "click",
    "tap_2finger": "right_click",
    "pinch_in": "xdotool key ctrl+minus",
    "pinch_out": "xdotool key ctrl+plus"
  }
}
```

## Usage

### Basic Usage

After installation, the service runs automatically. For manual testing or development:

```bash
# Check system dependencies and device detection
mouse-gesture-recognition --check-deps

# Run manually with auto-detection (for testing)
mouse-gesture-recognition

# Specify device path explicitly
mouse-gesture-recognition -d /dev/input/event26

# Enable verbose logging for debugging
mouse-gesture-recognition -v

# Run directly from build directory (before installation)
sudo ./target/release/mouse-gesture-recognition --check-deps
```

**Note**: After installation, no `sudo` is required as the application runs with user permissions and accesses devices through proper group membership.

### Finding Your Device

Use `evtest` to find your Magic Mouse device:

```bash
sudo evtest
```

Look for a device with "Magic Mouse" in the name and note its event number.

### Finding Your Device Event Number

To find out which `/dev/input/eventX` device corresponds to your Magic Mouse:

1. Run the following command:

```fish
sudo evtest
```

2. You will see a list of input devices. Look for one with "Magic Mouse" in the name. Note the event number (e.g., `/dev/input/event27`).

3. Select that number when prompted, or use it directly:

```fish
evtest /dev/input/event27
```

4. Move or tap your Magic Mouse. If you see events appear, you have found the correct device.

If you want to check without root, add your user to the `input` group and log out/in:

```fish
sudo usermod -aG input (whoami)
```

If you do not want to log out, you can use `newgrp input` to start a new shell with the new group applied immediately:

```fish
newgrp input
```

Then use `evtest` without `sudo` in that shell.

### Running as a Service

The installation script automatically sets up a systemd user service. After installation:

```bash
# Enable the service to start automatically
systemctl --user enable mouse-gesture.service

# Start the service
systemctl --user start mouse-gesture.service

# Check service status
systemctl --user status mouse-gesture.service

# View service logs
journalctl --user -u mouse-gesture.service -f
```

The service is configured to:
- Start automatically when you log in
- Restart automatically if it crashes
- Run with your user permissions (no root required)
- Access the configuration in `~/.config/mouse-gesture/config.json`

You can also use the provided service management script:

```bash
# Service management
./scripts/service.sh start      # Start the service
./scripts/service.sh stop       # Stop the service
./scripts/service.sh restart    # Restart the service
./scripts/service.sh status     # Check service status
./scripts/service.sh enable     # Enable automatic startup
./scripts/service.sh disable    # Disable automatic startup

# Monitoring and configuration
./scripts/service.sh logs       # View service logs
./scripts/service.sh edit-config # Edit configuration file
```

### Additional Management Scripts

The project includes several utility scripts:

```bash
# Development helpers
./scripts/dev.sh build          # Build project
./scripts/dev.sh run            # Run directly (not as service)
./scripts/dev.sh test           # Run tests
./scripts/dev.sh clean          # Clean build artifacts

# Build system
make install                    # Same as ./scripts/install.sh
make uninstall                  # Same as ./scripts/uninstall.sh
make build                      # Build project
make run                        # Run project
```

## Permissions

The application runs with user privileges and accesses input devices through proper group membership. The installation script:

1. Adds your user to the `input` group for device access
2. Sets up udev rules for Magic Mouse devices
3. Configures appropriate file permissions

No root privileges are required for normal operation. The service runs as your user account with access to:
- Input devices (via `input` group membership)
- Your desktop environment for executing actions
- User configuration directory (`~/.config/mouse-gesture/`)

## Troubleshooting

### Device Not Found

- Ensure the Magic Mouse is connected and paired
- Check that the `hid-magicmouse` module is loaded
- Verify the device path with `ls /dev/input/event*`

### Gestures Not Recognized

- Enable verbose logging with `-v` flag
- Check the gesture thresholds in configuration
- Ensure multi-touch events are being generated

### Actions Not Executing

- Run `--check-deps` to verify system dependencies
- Check that `xdotool` is installed and working
- Verify the command syntax in configuration

## Development

### Project Structure

```
src/
├── main.rs           # Application entry point
├── device.rs         # Magic Mouse device handling
├── gesture.rs        # Gesture recognition algorithms
├── config.rs         # Configuration management
└── event_handler.rs  # Action execution
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT OR Apache-2.0 license.

## Acknowledgments

- [Linux Magic Trackpad 2 USB-C Driver](https://github.com/mr-cal/Linux-Magic-Trackpad-2-USB-C-Driver) for the kernel module
- The evdev and Rust community for excellent libraries
