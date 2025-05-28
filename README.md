# Magic Mouse Gesture Recognition

A Rust-based gesture recognition tool for Apple Magic Mouse on Linux, specifically designed for the Magic Mouse 2 USB-C 2024 model.

## Features

- **Multi-touch gesture recognition**: Detects swipes, scrolls, taps, and pinch gestures
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

## Building

1. Clone the repository:

```bash
git clone <repository-url>
cd mouse-gesture
```

2. Build the project:

```bash
cargo build --release
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

```bash
# Run with auto-detection
sudo ./target/release/mouse-gesture-recognition

# Specify device path explicitly
sudo ./target/release/mouse-gesture-recognition -d /dev/input/event26

# Enable verbose logging
sudo ./target/release/mouse-gesture-recognition -v

# Check system dependencies
./target/release/mouse-gesture-recognition --check-deps
```

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

2. You will see a list of input devices. Look for one with "Magic Mouse" in the name. Note the event number (e.g., `/dev/input/event26`).

3. Select that number when prompted, or use it directly:

```fish
evtest /dev/input/event26
```

4. Move or tap your Magic Mouse. If you see events appear, you have found the correct device.

If you want to check without root, add your user to the `input` group and log out/in:

```fish
sudo usermod -aG input (whoami)
```

Then use `evtest` without `sudo` after re-logging in.

### Running as a Service

To run the gesture recognition as a system service, create a systemd service file:

```ini
[Unit]
Description=Magic Mouse Gesture Recognition
After=graphical-session.target

[Service]
Type=simple
ExecStart=/path/to/mouse-gesture-recognition -d /dev/input/event26
Restart=always
User=root
Environment=DISPLAY=:0

[Install]
WantedBy=graphical-session.target
```

## Permissions

The application requires root privileges to access input devices. For security, consider:

1. Adding your user to the `input` group
2. Setting appropriate udev rules for device access
3. Using capabilities instead of full root access

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
