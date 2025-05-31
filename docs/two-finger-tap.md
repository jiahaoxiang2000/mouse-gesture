# Multi-Touch Two-Finger Tap Recognition Implementation

## Overview

This document describes the implementation of two-finger tap recognition for the Apple Magic Mouse on Linux, based on the Linux Multi-Touch Protocol (Type B).

## Linux Multi-Touch Protocol Implementation

Our implementation follows the Linux Multi-Touch Protocol Type B specification, which uses slots and tracking IDs to manage individual touch contacts.

### Key Components

#### 1. TouchContact Structure
```rust
pub struct TouchContact {
    pub id: i32,                    // Tracking ID from kernel
    pub x: i32,                     // ABS_MT_POSITION_X
    pub y: i32,                     // ABS_MT_POSITION_Y  
    pub touch_major: i32,           // ABS_MT_TOUCH_MAJOR
    pub touch_minor: i32,           // ABS_MT_TOUCH_MINOR
    pub orientation: i32,           // ABS_MT_ORIENTATION
    pub pressure: f64,              // Calculated from touch area
    pub first_contact_time: Instant,
    pub last_update_time: Instant,
    pub is_active: bool,
}
```

#### 2. Multi-Touch Event Processing

The `MultiTouchProcessor` handles the raw evdev events following the Linux MT protocol:

- **ABS_MT_SLOT**: Switches between contact slots (0-15 for Magic Mouse)
- **ABS_MT_TRACKING_ID**: Creates (-1 = end) or updates contacts
- **ABS_MT_POSITION_X/Y**: Updates contact position
- **ABS_MT_TOUCH_MAJOR/MINOR**: Contact area dimensions
- **ABS_MT_ORIENTATION**: Contact orientation
- **EV_SYN**: Synchronization event to process accumulated changes

## Two-Finger Tap Detection Algorithm

### Detection Criteria

A two-finger tap is detected when all of the following conditions are met:

1. **Exactly 2 active contacts**: Must have precisely two fingers touching
2. **Short duration**: Both contacts must be active for less than `two_finger_tap_timeout_ms` (default: 250ms)
3. **Close proximity**: Distance between contacts < `two_finger_tap_distance_threshold` (default: 30.0mm)
4. **Sufficient pressure**: Both contacts must exceed `contact_pressure_threshold` (default: 50.0)
5. **Simultaneous contact**: Both fingers should start contact within 100ms of each other

### Algorithm Flow

```
1. Event Processing:
   - Parse ABS_MT_* events for each slot
   - Update TouchContact structures
   - Track gesture state (Idle -> SingleTouch -> TwoTouch)

2. On Synchronization (EV_SYN):
   - Move pending contacts to active contacts
   - Run two-finger tap detection
   - Clean up expired contacts

3. Two-Finger Tap Detection:
   - Check contact count == 2
   - Verify both contacts are recent (< timeout)
   - Calculate distance between contact centers
   - Verify pressure thresholds
   - Generate TwoFingerTap event if all criteria met
```

### Configuration Parameters

```json
{
  "gesture": {
    "two_finger_tap_timeout_ms": 250,        // Max tap duration
    "two_finger_tap_distance_threshold": 30.0,  // Max distance between fingers (mm)
    "contact_pressure_threshold": 50.0       // Min pressure for valid contact
  }
}
```

## Magic Mouse Hardware Characteristics

Based on the `evtest` output from the Magic Mouse 2 USB-C 2024:

```
Event type 3 (EV_ABS)
  ABS_MT_SLOT (47): 0-15 slots (16 total contacts supported)
  ABS_MT_TOUCH_MAJOR (48): 0-1020 units, contact area major axis
  ABS_MT_TOUCH_MINOR (49): 0-1020 units, contact area minor axis  
  ABS_MT_ORIENTATION (52): -31 to 32, contact orientation
  ABS_MT_POSITION_X (53): -1100 to 1258, X coordinate
  ABS_MT_POSITION_Y (54): -1589 to 2047, Y coordinate
  ABS_MT_TRACKING_ID (57): 0-65535, unique contact identifier
```

### Coordinate System
- **X Range**: -1100 to 1258 (total: 2358 units)
- **Y Range**: -1589 to 2047 (total: 3636 units)
- **Resolution**: X=26 units/mm, Y=70 units/mm
- **Physical Size**: ~90mm x 52mm touch surface

### Pressure Calculation
```rust
pressure = ((touch_major + touch_minor) / 2.0) / 1020.0 * 100.0
```

This provides a percentage-based pressure value where 100% represents maximum contact area.

## Usage Example

```bash
# Run with auto-detection
sudo ./target/release/mouse-gesture-recognition

# Run with explicit device path  
sudo ./target/release/mouse-gesture-recognition -d /dev/input/event27

# Enable verbose logging to see tap detection
sudo ./target/release/mouse-gesture-recognition -v
```

When a two-finger tap is detected, you'll see log output like:
```
[INFO] Two-finger tap detected: distance=85.32, pressures=(67.45, 72.18)
```

And the configured action will be executed (default: right-click).

## Testing Two-Finger Tap

1. **Light Touch**: Place two fingers gently on the Magic Mouse surface
2. **Quick Tap**: Tap briefly (< 250ms) and lift both fingers
3. **Close Together**: Keep fingers within ~30mm of each other
4. **Sufficient Pressure**: Press firmly enough to register contact

The gesture will be recognized and execute the configured action (`tap_2finger` by default maps to right-click).

## Troubleshooting

### Common Issues

1. **No Detection**: 
   - Check Magic Mouse is connected and paired
   - Verify `/dev/input/event27` exists
   - Ensure `hid-magicmouse` module is loaded
   - Try with verbose logging (`-v` flag)

2. **False Positives**:
   - Increase `two_finger_tap_distance_threshold` 
   - Decrease `two_finger_tap_timeout_ms`
   - Increase `contact_pressure_threshold`

3. **Missed Taps**:
   - Decrease distance threshold
   - Increase timeout duration
   - Decrease pressure threshold
   - Check finger placement (too light/too far apart)

### Debug Information

Enable verbose logging to see detailed multi-touch events:
```bash
sudo ./target/release/mouse-gesture-recognition -v
```

This will show:
- Raw evdev events
- Touch contact creation/updates
- Gesture detection attempts
- Calculated distances and pressures

## References

- [Linux Multi-Touch Protocol](https://www.kernel.org/doc/Documentation/input/multi-touch-protocol.txt)
- [Magic Mouse Driver](https://github.com/mr-cal/Linux-Magic-Trackpad-2-USB-C-Driver)
- [evdev Documentation](https://python-evdev.readthedocs.io/en/latest/tutorial.html)
