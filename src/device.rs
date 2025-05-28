use anyhow::{Result, Context};
use evdev::{Device, InputEvent, EventType, AbsoluteAxisType};
use log::{debug, info, warn};
use std::path::Path;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::time::{sleep, Duration};
use lazy_static::lazy_static;

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
        let device = Device::open(device_path.as_ref())
            .with_context(|| format!("Failed to open device: {}", device_path.as_ref().display()))?;
        
        // Verify this is likely a Magic Mouse by checking capabilities
        let name = device.name().unwrap_or("Unknown");
        info!("Opened device: {}", name);
        
        // Check for multi-touch capabilities
        if !device.supported_absolute_axes().map_or(false, |axes| {
            axes.contains(AbsoluteAxisType::ABS_MT_POSITION_X) &&
            axes.contains(AbsoluteAxisType::ABS_MT_POSITION_Y) &&
            axes.contains(AbsoluteAxisType::ABS_MT_TRACKING_ID)
        }) {
            warn!("Device may not be a Magic Mouse - missing expected multi-touch capabilities");
        }
        
        Ok(Self { device })
    }
    
    /// Read the next input event from the device
    pub async fn read_event(&mut self) -> Result<MouseEvent> {
        loop {
            let events = self.device.fetch_events()
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
    fn process_raw_event(
        event: InputEvent,
    ) -> Result<Option<MouseEvent>> {
        let mut current_slot = CURRENT_SLOT.lock().unwrap();
        let mut touch_points = TOUCH_POINTS.lock().unwrap();
        
        match event.event_type() {
            EventType::ABSOLUTE => {
                match event.code() {
                    // Multi-touch slot selection
                    47 => { // ABS_MT_SLOT
                        *current_slot = event.value();
                        debug!("Switched to slot: {}", *current_slot);
                    }
                    
                    // Touch tracking ID (touch start/end)
                    57 => { // ABS_MT_TRACKING_ID
                        let tracking_id = event.value();
                        if tracking_id == -1 {
                            // Touch end
                            if let Some(point) = touch_points.remove(&*current_slot) {
                                return Ok(Some(MouseEvent::TouchEnd { tracking_id: point.tracking_id }));
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
                    53 => { // ABS_MT_POSITION_X
                        if let Some(point) = touch_points.get_mut(&*current_slot) {
                            point.x = event.value();
                            return Ok(Some(MouseEvent::TouchMove { point: point.clone() }));
                        }
                    }
                    
                    54 => { // ABS_MT_POSITION_Y
                        if let Some(point) = touch_points.get_mut(&*current_slot) {
                            point.y = event.value();
                            return Ok(Some(MouseEvent::TouchMove { point: point.clone() }));
                        }
                    }
                    
                    48 => { // ABS_MT_TOUCH_MAJOR
                        if let Some(point) = touch_points.get_mut(&*current_slot) {
                            point.touch_major = event.value();
                        }
                    }
                    
                    49 => { // ABS_MT_TOUCH_MINOR
                        if let Some(point) = touch_points.get_mut(&*current_slot) {
                            point.touch_minor = event.value();
                        }
                    }
                    
                    52 => { // ABS_MT_ORIENTATION
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
                    0 => { // REL_X
                        return Ok(Some(MouseEvent::Movement { 
                            dx: event.value(), 
                            dy: 0 
                        }));
                    }
                    1 => { // REL_Y
                        return Ok(Some(MouseEvent::Movement { 
                            dx: 0, 
                            dy: event.value() 
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
