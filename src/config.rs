use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub device: DeviceConfig,
    pub gesture: GestureConfig,
    pub actions: ActionConfig,
}

/// Device-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Device path (e.g., /dev/input/event26)
    pub path: Option<String>,
    /// Auto-detect device by name pattern
    pub auto_detect: bool,
    /// Device name pattern for auto-detection
    pub name_pattern: String,
}

/// Gesture recognition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureConfig {
    /// Minimum distance for scroll gesture (in device units)
    pub scroll_threshold: f32,
    /// Minimum distance for swipe gesture (in device units)
    pub swipe_threshold: f32,
    /// Minimum scale change for pinch gesture (ratio)
    pub pinch_threshold: f32,
    /// Maximum time for tap gesture (milliseconds)
    pub tap_timeout_ms: u64,
    /// Minimum time between gesture detections (milliseconds)
    pub debounce_ms: u64,
}

/// Action configuration for gestures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfig {
    pub swipe_left_2finger: String,
    pub swipe_right_2finger: String,
    pub swipe_up_2finger: String,
    pub swipe_down_2finger: String,
    pub scroll_vertical: String,
    pub scroll_horizontal: String,
    pub tap_1finger: String,
    pub tap_2finger: String,
    pub pinch_in: String,
    pub pinch_out: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device: DeviceConfig {
                path: None,
                auto_detect: true,
                name_pattern: "Magic Mouse".to_string(),
            },
            gesture: GestureConfig {
                scroll_threshold: 50.0,
                swipe_threshold: 100.0,
                pinch_threshold: 0.1,
                tap_timeout_ms: 300,
                debounce_ms: 100,
            },
            actions: ActionConfig {
                swipe_left_2finger: "xdotool key alt+Right".to_string(),  // Browser back
                swipe_right_2finger: "xdotool key alt+Left".to_string(),   // Browser forward
                swipe_up_2finger: "xdotool key ctrl+t".to_string(),        // New tab
                swipe_down_2finger: "xdotool key ctrl+w".to_string(),      // Close tab
                scroll_vertical: "scroll_vertical".to_string(),
                scroll_horizontal: "scroll_horizontal".to_string(),
                tap_1finger: "click".to_string(),
                tap_2finger: "right_click".to_string(),
                pinch_in: "xdotool key ctrl+minus".to_string(),            // Zoom out
                pinch_out: "xdotool key ctrl+plus".to_string(),            // Zoom in
            },
        }
    }
}

impl Config {
    /// Load configuration from file, or return default if file doesn't exist
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            log::info!("Configuration file not found, creating default: {}", path.display());
            let default_config = Self::default();
            default_config.save(path)?;
            return Ok(default_config);
        }
        
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file: {}", path.display()))?;
        
        let config: Config = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse configuration file: {}", path.display()))?;
        
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        
        fs::write(path, content)
            .with_context(|| format!("Failed to write configuration file: {}", path.display()))?;
        
        log::info!("Configuration saved to: {}", path.display());
        Ok(())
    }
}
