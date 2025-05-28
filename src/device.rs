use anyhow::{Context, Result};
use evdev::{AbsoluteAxisType, Device, EventType, InputEvent};
use lazy_static::lazy_static;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use tokio::time::{sleep, Duration};

lazy_static! {
    /// Static variables to track touch state across function calls
    static ref CURRENT_SLOT: Mutex<i32> = Mutex::new(0);
    static ref TOUCH_POINTS: Mutex<HashMap<i32, TouchPoint>> = Mutex::new(HashMap::new());
}

/// Represents a Magic Mouse device for reading input events
pub struct MagicMouseDevice {
    device: Device,
}

/// Raw touch data from Magic Mouse
#[derive(Debug, Clone)]
pub struct TouchPoint {
    pub tracking_id: i32,
    pub x: i32,
    pub y: i32,
    pub touch_major: i32,
    pub touch_minor: i32,
    pub orientation: i32,
    pub slot: i32,
}

/// Processed mouse event from Magic Mouse
#[derive(Debug, Clone)]
pub enum MouseEvent {
    TouchStart { point: TouchPoint },
    TouchMove { point: TouchPoint },
    TouchEnd { tracking_id: i32 },
    Button { button: u16, pressed: bool },
    Movement { dx: i32, dy: i32 },
}

impl MagicMouseDevice {
    /// Create a new Magic Mouse device instance
    pub fn new<P: AsRef<Path>>(device_path: P) -> Result<Self> {
        let device = Device::open(device_path.as_ref()).with_context(|| {
            format!("Failed to open device: {}", device_path.as_ref().display())
        })?;

        // Verify this is likely a Magic Mouse by checking capabilities
        let name = device.name().unwrap_or("Unknown");
        info!("Opened device: {}", name);

        // Check for multi-touch capabilities
        if !device.supported_absolute_axes().map_or(false, |axes| {
            axes.contains(AbsoluteAxisType::ABS_MT_POSITION_X)
                && axes.contains(AbsoluteAxisType::ABS_MT_POSITION_Y)
                && axes.contains(AbsoluteAxisType::ABS_MT_TRACKING_ID)
        }) {
            warn!("Device may not be a Magic Mouse - missing expected multi-touch capabilities");
        }

        Ok(Self { device })
    }

    /// Read the next input event from the device
    pub async fn read_event(&mut self) -> Result<MouseEvent> {
        loop {
            let events = self
                .device
                .fetch_events()
                .context("Failed to fetch events from device")?;

            for event in events {
                debug!("Raw event: {:?}", event);

                if let Some(mouse_event) = Self::process_raw_event(event)? {
                    return Ok(mouse_event);
                }
            }

            // Small delay to prevent busy waiting
            sleep(Duration::from_millis(1)).await;
        }
    }

    /// Process a raw input event into a mouse event
    fn process_raw_event(event: InputEvent) -> Result<Option<MouseEvent>> {
        let mut current_slot = CURRENT_SLOT.lock().unwrap();
        let mut touch_points = TOUCH_POINTS.lock().unwrap();

        match event.event_type() {
            EventType::ABSOLUTE => {
                match event.code() {
                    // Multi-touch slot selection
                    47 => {
                        // ABS_MT_SLOT
                        *current_slot = event.value();
                        debug!("Switched to slot: {}", *current_slot);
                    }

                    // Touch tracking ID (touch start/end)
                    57 => {
                        // ABS_MT_TRACKING_ID
                        let tracking_id = event.value();
                        if tracking_id == -1 {
                            // Touch end
                            if let Some(point) = touch_points.remove(&*current_slot) {
                                return Ok(Some(MouseEvent::TouchEnd {
                                    tracking_id: point.tracking_id,
                                }));
                            }
                        } else {
                            // Touch start - create new touch point
                            let point = TouchPoint {
                                tracking_id,
                                slot: *current_slot,
                                x: 0,
                                y: 0,
                                touch_major: 0,
                                touch_minor: 0,
                                orientation: 0,
                            };
                            touch_points.insert(*current_slot, point.clone());
                            return Ok(Some(MouseEvent::TouchStart { point }));
                        }
                    }

                    // Position and touch data
                    53 => {
                        // ABS_MT_POSITION_X
                        if let Some(point) = touch_points.get_mut(&*current_slot) {
                            point.x = event.value();
                            return Ok(Some(MouseEvent::TouchMove {
                                point: point.clone(),
                            }));
                        }
                    }

                    54 => {
                        // ABS_MT_POSITION_Y
                        if let Some(point) = touch_points.get_mut(&*current_slot) {
                            point.y = event.value();
                            return Ok(Some(MouseEvent::TouchMove {
                                point: point.clone(),
                            }));
                        }
                    }

                    48 => {
                        // ABS_MT_TOUCH_MAJOR
                        if let Some(point) = touch_points.get_mut(&*current_slot) {
                            point.touch_major = event.value();
                        }
                    }

                    49 => {
                        // ABS_MT_TOUCH_MINOR
                        if let Some(point) = touch_points.get_mut(&*current_slot) {
                            point.touch_minor = event.value();
                        }
                    }

                    52 => {
                        // ABS_MT_ORIENTATION
                        if let Some(point) = touch_points.get_mut(&*current_slot) {
                            point.orientation = event.value();
                        }
                    }

                    _ => {}
                }
            }

            EventType::KEY => {
                // Mouse button events
                return Ok(Some(MouseEvent::Button {
                    button: event.code(),
                    pressed: event.value() != 0,
                }));
            }

            EventType::RELATIVE => {
                // Mouse movement events
                match event.code() {
                    0 => {
                        // REL_X
                        return Ok(Some(MouseEvent::Movement {
                            dx: event.value(),
                            dy: 0,
                        }));
                    }
                    1 => {
                        // REL_Y
                        return Ok(Some(MouseEvent::Movement {
                            dx: 0,
                            dy: event.value(),
                        }));
                    }
                    _ => {}
                }
            }

            _ => {}
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evdev::{EventType, InputEvent};

    #[test]
    fn test_touchpoint_struct() {
        let tp = TouchPoint {
            tracking_id: 42,
            x: 100,
            y: 200,
            touch_major: 10,
            touch_minor: 5,
            orientation: 1,
            slot: 2,
        };
        assert_eq!(tp.tracking_id, 42);
        assert_eq!(tp.x, 100);
        assert_eq!(tp.y, 200);
        assert_eq!(tp.touch_major, 10);
        assert_eq!(tp.touch_minor, 5);
        assert_eq!(tp.orientation, 1);
        assert_eq!(tp.slot, 2);
    }

    #[test]
    fn test_mouse_event_enum() {
        let tp = TouchPoint {
            tracking_id: 1,
            x: 10,
            y: 20,
            touch_major: 3,
            touch_minor: 2,
            orientation: 0,
            slot: 0,
        };
        let e1 = MouseEvent::TouchStart { point: tp.clone() };
        let e2 = MouseEvent::TouchMove { point: tp.clone() };
        let e3 = MouseEvent::TouchEnd { tracking_id: 1 };
        let e4 = MouseEvent::Button {
            button: 272,
            pressed: true,
        };
        let e5 = MouseEvent::Movement { dx: 5, dy: -3 };
        match e1 {
            MouseEvent::TouchStart { point } => assert_eq!(point.x, 10),
            _ => panic!("Wrong variant"),
        }
        match e2 {
            MouseEvent::TouchMove { point } => assert_eq!(point.y, 20),
            _ => panic!("Wrong variant"),
        }
        match e3 {
            MouseEvent::TouchEnd { tracking_id } => assert_eq!(tracking_id, 1),
            _ => panic!("Wrong variant"),
        }
        match e4 {
            MouseEvent::Button { button, pressed } => {
                assert_eq!(button, 272);
                assert!(pressed);
            }
            _ => panic!("Wrong variant"),
        }
        match e5 {
            MouseEvent::Movement { dx, dy } => {
                assert_eq!(dx, 5);
                assert_eq!(dy, -3);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_process_raw_event_slot_and_tracking_id() {
        // Set up ABS_MT_SLOT event (code 47)
        let slot_event = InputEvent::new(EventType::ABSOLUTE, 47, 1);
        let _ = MagicMouseDevice::process_raw_event(slot_event).unwrap();
        // Set up ABS_MT_TRACKING_ID event (code 57, value != -1)
        let tracking_event = InputEvent::new(EventType::ABSOLUTE, 57, 123);
        let evt = MagicMouseDevice::process_raw_event(tracking_event).unwrap();
        match evt {
            Some(MouseEvent::TouchStart { point }) => {
                assert_eq!(point.tracking_id, 123);
                assert_eq!(point.slot, 1);
            }
            _ => panic!("Expected TouchStart event"),
        }
        // Set up ABS_MT_TRACKING_ID event (code 57, value == -1)
        let end_event = InputEvent::new(EventType::ABSOLUTE, 57, -1);
        let evt = MagicMouseDevice::process_raw_event(end_event).unwrap();
        match evt {
            Some(MouseEvent::TouchEnd { tracking_id }) => {
                assert_eq!(tracking_id, 123);
            }
            _ => panic!("Expected TouchEnd event"),
        }
    }

    #[test]
    fn test_process_raw_event_position() {
        // Set up slot and tracking id
        let _ = MagicMouseDevice::process_raw_event(InputEvent::new(EventType::ABSOLUTE, 47, 2))
            .unwrap();
        let _ = MagicMouseDevice::process_raw_event(InputEvent::new(EventType::ABSOLUTE, 57, 99))
            .unwrap();
        // Set up ABS_MT_POSITION_X event (code 53)
        let x_event = InputEvent::new(EventType::ABSOLUTE, 53, 321);
        let evt = MagicMouseDevice::process_raw_event(x_event).unwrap();
        match evt {
            Some(MouseEvent::TouchMove { point }) => {
                assert_eq!(point.x, 321);
                assert_eq!(point.slot, 2);
            }
            _ => panic!("Expected TouchMove event for X"),
        }
        // Set up ABS_MT_POSITION_Y event (code 54)
        let y_event = InputEvent::new(EventType::ABSOLUTE, 54, 654);
        let evt = MagicMouseDevice::process_raw_event(y_event).unwrap();
        match evt {
            Some(MouseEvent::TouchMove { point }) => {
                assert_eq!(point.y, 654);
                assert_eq!(point.slot, 2);
            }
            _ => panic!("Expected TouchMove event for Y"),
        }
    }

    #[test]
    fn test_process_raw_event_button_and_movement() {
        // Button event
        let btn_event = InputEvent::new(EventType::KEY, 272, 1);
        let evt = MagicMouseDevice::process_raw_event(btn_event).unwrap();
        match evt {
            Some(MouseEvent::Button { button, pressed }) => {
                assert_eq!(button, 272);
                assert!(pressed);
            }
            _ => panic!("Expected Button event"),
        }
        // Movement event REL_X
        let rel_x_event = InputEvent::new(EventType::RELATIVE, 0, 7);
        let evt = MagicMouseDevice::process_raw_event(rel_x_event).unwrap();
        match evt {
            Some(MouseEvent::Movement { dx, dy }) => {
                assert_eq!(dx, 7);
                assert_eq!(dy, 0);
            }
            _ => panic!("Expected Movement event for REL_X"),
        }
        // Movement event REL_Y
        let rel_y_event = InputEvent::new(EventType::RELATIVE, 1, -4);
        let evt = MagicMouseDevice::process_raw_event(rel_y_event).unwrap();
        match evt {
            Some(MouseEvent::Movement { dx, dy }) => {
                assert_eq!(dx, 0);
                assert_eq!(dy, -4);
            }
            _ => panic!("Expected Movement event for REL_Y"),
        }
    }
}
