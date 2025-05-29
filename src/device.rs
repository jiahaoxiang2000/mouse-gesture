use anyhow::{Context, Result};
use evdev::Device;
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

use crate::event_handler::EventHandler;
use crate::multitouch::MultiTouchProcessor;

pub struct MagicMouseDevice {
    device: Device,
    path: PathBuf,
}

impl MagicMouseDevice {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let device =
            Device::open(&path).with_context(|| format!("Failed to open device: {:?}", path))?;

        info!("Opened Magic Mouse device: {:?}", path);
        info!("Device name: {}", device.name().unwrap_or("Unknown"));

        // Log device capabilities for debugging
        debug!("Device capabilities:");
        debug!(
            "  Device supports absolute events: {}",
            device
                .supported_events()
                .contains(evdev::EventType::ABSOLUTE)
        );
        debug!(
            "  Device supports multi-touch: {}",
            device.supported_absolute_axes().map_or(false, |axes| {
                axes.contains(evdev::AbsoluteAxisType::ABS_MT_SLOT)
            })
        );

        Ok(Self { device, path })
    }

    pub async fn start_recognition(&mut self, event_handler: EventHandler) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(1000);

        // Create multi-touch processor
        let mut mt_processor = MultiTouchProcessor::new(event_handler.config.gesture.clone());

        // Spawn event reader task
        let device_path = self.path.clone();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            let mut device = match Device::open(&device_path) {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to open device in reader task: {}", e);
                    return;
                }
            };

            loop {
                match device.fetch_events() {
                    Ok(events) => {
                        for event in events {
                            if let Err(e) = tx_clone.send(event).await {
                                error!("Failed to send event: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to fetch events: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }

                // Small delay to prevent busy waiting
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        });

        // Process events
        while let Some(event) = rx.recv().await {
            // Only process ABS_* events through multi-touch processor
            if event.event_type() == evdev::EventType::ABSOLUTE {
                debug!("Raw event: {:?}", event);
                if let Some(mt_events) = mt_processor.process_event(event).await {
                    for mt_event in mt_events {
                        // Handle the multi-touch event
                        if let Err(e) = event_handler.handle_multitouch_event(mt_event).await {
                            warn!("Failed to handle multi-touch event: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Find Magic Mouse device automatically
pub fn find_magic_mouse_device(name_pattern: &str) -> Result<PathBuf> {
    let input_dir = Path::new("/dev/input");

    if !input_dir.exists() {
        return Err(anyhow::anyhow!("/dev/input directory not found"));
    }

    for entry in std::fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only check event devices
        if let Some(filename) = path.file_name() {
            if let Some(filename_str) = filename.to_str() {
                if filename_str.starts_with("event") {
                    // Try to open the device and check its name
                    if let Ok(device) = Device::open(&path) {
                        if let Some(device_name) = device.name() {
                            if device_name.contains(name_pattern) {
                                info!("Found Magic Mouse device: {} at {:?}", device_name, path);
                                return Ok(path);
                            }
                        }
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!("Magic Mouse device not found. Ensure it's connected and the hid-magicmouse module is loaded."))
}
