use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub device: DeviceConfig,
    pub gesture: GestureConfig,
    pub actions: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub path: Option<String>,
    pub auto_detect: bool,
    pub name_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureConfig {
    /// Minimum movement distance in millimeters for scroll gestures
    pub scroll_threshold: f64,
    /// Minimum movement distance in millimeters for swipe gestures
    pub swipe_threshold: f64,
    /// Minimum scale change percentage for pinch gestures (0.1 = 10%)
    pub pinch_threshold: f64,
    /// Maximum duration in milliseconds for tap gestures
    pub tap_timeout_ms: u64,
    /// Debounce time in milliseconds between gesture events
    pub debounce_ms: u64,
    // Multi-touch specific settings
    /// Maximum duration in milliseconds for two-finger tap
    pub two_finger_tap_timeout_ms: u64,
    /// Maximum distance in millimeters between fingers for two-finger tap
    pub two_finger_tap_distance_threshold: f64,
    /// Minimum contact pressure percentage for valid touch (0-100)
    pub contact_pressure_threshold: f64,
    /// Maximum movement distance in millimeters for single-finger tap
    pub single_finger_tap_movement_threshold: f64,
}

impl Default for Config {
    fn default() -> Self {
        let mut actions = HashMap::new();

        // Default action mappings
        actions.insert(
            "swipe_left_2finger".to_string(),
            "xdotool key alt+Right".to_string(),
        );
        actions.insert(
            "swipe_right_2finger".to_string(),
            "xdotool key alt+Left".to_string(),
        );
        actions.insert(
            "swipe_up_2finger".to_string(),
            "xdotool key ctrl+t".to_string(),
        );
        actions.insert(
            "swipe_down_2finger".to_string(),
            "xdotool key ctrl+w".to_string(),
        );
        actions.insert("scroll_vertical".to_string(), "scroll_vertical".to_string());
        actions.insert(
            "scroll_horizontal".to_string(),
            "scroll_horizontal".to_string(),
        );
        actions.insert("tap_1finger".to_string(), "click".to_string());
        actions.insert("tap_2finger".to_string(), "right_click".to_string());
        actions.insert("pinch_in".to_string(), "xdotool key ctrl+minus".to_string());
        actions.insert("pinch_out".to_string(), "xdotool key ctrl+plus".to_string());

        Self {
            device: DeviceConfig {
                path: None,
                auto_detect: true,
                name_pattern: "Magic Mouse".to_string(),
            },
            gesture: GestureConfig {
                scroll_threshold: 2.0, // 2mm movement threshold for scroll
                swipe_threshold: 12.0, // 12mm movement threshold for swipe
                pinch_threshold: 0.1,  // 10% scale change threshold for pinch
                tap_timeout_ms: 300,
                debounce_ms: 100,
                two_finger_tap_timeout_ms: 250,
                two_finger_tap_distance_threshold: 30.0, // 30mm max distance between fingers for tap
                contact_pressure_threshold: 50.0, // Keep pressure threshold as-is (percentage)
                single_finger_tap_movement_threshold: 2.0, // 2mm max movement for single tap
            },
            actions,
        }
    }
}

impl Config {
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if path.exists() {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read config file: {:?}", path))?;

            let config: Config = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {:?}", path))?;

            Ok(config)
        } else {
            let default_config = Config::default();
            let content = serde_json::to_string_pretty(&default_config)
                .context("Failed to serialize default config")?;

            std::fs::write(path, content)
                .with_context(|| format!("Failed to write default config to: {:?}", path))?;

            log::info!("Created default configuration file: {:?}", path);
            Ok(default_config)
        }
    }
}
