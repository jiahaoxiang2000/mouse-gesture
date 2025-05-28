use anyhow::Result;
use clap::Parser;
use log::{info, warn, error};
use std::path::PathBuf;

mod device;
mod gesture;
mod config;
mod event_handler;

use crate::device::MagicMouseDevice;
use crate::gesture::GestureRecognizer;
use crate::config::Config;
use crate::event_handler::{EventHandler, check_system_dependencies};

#[derive(Parser)]
#[command(name = "mouse-gesture-recognition")]
#[command(about = "A mouse gesture recognition tool for Apple Magic Mouse")]
struct Cli {
    /// Path to the input device (e.g., /dev/input/event26)
    #[arg(short, long)]
    device: Option<PathBuf>,
    
    /// Configuration file path
    #[arg(short, long, default_value = "config.json")]
    config: PathBuf,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Check system dependencies and exit
    #[arg(long)]
    check_deps: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::init();
    }
    
    info!("Starting Magic Mouse Gesture Recognition");
    
    // Check system dependencies if requested
    if cli.check_deps {
        check_system_dependencies()?;
        return Ok(());
    }
    
    // Load configuration
    let config = Config::load(&cli.config).unwrap_or_default();
    info!("Configuration loaded from: {}", cli.config.display());
    
    // Check system dependencies
    check_system_dependencies()?;
    
    // Initialize device
    let device_path = cli.device.unwrap_or_else(|| {
        warn!("No device specified, attempting to auto-detect Magic Mouse");
        PathBuf::from("/dev/input/event26") // Default from the docs
    });
    
    let mut device = MagicMouseDevice::new(&device_path)?;
    info!("Magic Mouse device initialized: {}", device_path.display());
    
    // Initialize gesture recognizer and event handler
    let mut gesture_recognizer = GestureRecognizer::new(config.gesture.clone());
    let event_handler = EventHandler::new(config.actions);
    
    // Main event loop
    info!("Starting gesture recognition... Press Ctrl+C to stop.");
    loop {
        match device.read_event().await {
            Ok(event) => {
                if let Some(gesture) = gesture_recognizer.process_event(event) {
                    info!("Gesture recognized: {:?}", gesture);
                    
                    if let Err(e) = event_handler.handle_gesture(gesture).await {
                        error!("Failed to handle gesture: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Error reading event: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}
