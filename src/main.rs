use anyhow::Result;
use clap::Parser;
use log::{error, info, warn};
use std::path::PathBuf;

mod config;
mod device;
mod event_handler;
mod gesture;
mod multitouch;

use config::Config;
use device::MagicMouseDevice;
use event_handler::EventHandler;

#[derive(Parser)]
#[command(name = "mouse-gesture-recognition")]
#[command(about = "Magic Mouse gesture recognition for Linux")]
struct Args {
    /// Device path (e.g., /dev/input/event26)
    #[arg(short, long)]
    device: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Check system dependencies
    #[arg(long)]
    check_deps: bool,

    /// Configuration file path
    #[arg(short, long, default_value = "config.json")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    info!(
        "Magic Mouse Gesture Recognition v{}",
        env!("CARGO_PKG_VERSION")
    );

    if args.check_deps {
        return check_dependencies().await;
    }

    // Load configuration
    let config = Config::load_or_create(&args.config)?;
    info!("Configuration loaded from: {:?}", args.config);

    // Initialize device
    let device_path = if let Some(path) = args.device {
        path
    } else if config.device.auto_detect {
        device::find_magic_mouse_device(&config.device.name_pattern)?
    } else {
        return Err(anyhow::anyhow!(
            "No device path specified and auto-detection is disabled"
        ));
    };

    info!("Using device: {:?}", device_path);

    // Initialize Magic Mouse device
    let mut device = MagicMouseDevice::new(device_path)?;

    // Initialize event handler
    let event_handler = EventHandler::new(config.clone());

    // Start gesture recognition
    info!("Starting gesture recognition...");
    device.start_recognition(event_handler).await?;

    Ok(())
}

async fn check_dependencies() -> Result<()> {
    info!("Checking system dependencies...");

    // Check for xdotool
    match tokio::process::Command::new("which")
        .arg("xdotool")
        .output()
        .await
    {
        Ok(output) if output.status.success() => {
            info!("✓ xdotool found");
        }
        _ => {
            warn!("✗ xdotool not found - install with: sudo pacman -S xdotool");
        }
    }

    // Check for evdev access
    if std::path::Path::new("/dev/input").exists() {
        info!("✓ /dev/input directory accessible");
    } else {
        error!("✗ /dev/input directory not accessible");
    }

    // Check for Magic Mouse devices
    match device::find_magic_mouse_device("Magic Mouse") {
        Ok(path) => {
            info!("✓ Magic Mouse device found at: {:?}", path);
        }
        Err(_) => {
            warn!("✗ Magic Mouse device not found - ensure it's connected and paired");
        }
    }

    Ok(())
}
