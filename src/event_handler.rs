use crate::config::ActionConfig;
use crate::gesture::{Gesture, SwipeDirection, ScrollDirection};
use anyhow::{Result, Context};
use log::{info, warn, error};
use std::process::Command;
use tokio::process::Command as AsyncCommand;

/// Handles execution of actions based on recognized gestures
pub struct EventHandler {
    actions: ActionConfig,
}

impl EventHandler {
    /// Create a new event handler with the given action configuration
    pub fn new(actions: ActionConfig) -> Self {
        Self { actions }
    }
    
    /// Execute action for a recognized gesture
    pub async fn handle_gesture(&self, gesture: Gesture) -> Result<()> {
        let action = match gesture {
            Gesture::Swipe { direction, fingers } => {
                match (direction, fingers) {
                    (SwipeDirection::Left, 2) => Some(&self.actions.swipe_left_2finger),
                    (SwipeDirection::Right, 2) => Some(&self.actions.swipe_right_2finger),
                    (SwipeDirection::Up, 2) => Some(&self.actions.swipe_up_2finger),
                    (SwipeDirection::Down, 2) => Some(&self.actions.swipe_down_2finger),
                    _ => {
                        info!("Unhandled swipe gesture: {:?} fingers, {:?}", fingers, direction);
                        None
                    }
                }
            }
            
            Gesture::Scroll { direction, delta: _ } => {
                match direction {
                    ScrollDirection::Vertical => Some(&self.actions.scroll_vertical),
                    ScrollDirection::Horizontal => Some(&self.actions.scroll_horizontal),
                }
            }
            
            Gesture::Tap { fingers, position: _ } => {
                match fingers {
                    1 => Some(&self.actions.tap_1finger),
                    2 => Some(&self.actions.tap_2finger),
                    _ => {
                        info!("Unhandled tap gesture: {} fingers", fingers);
                        None
                    }
                }
            }
            
            Gesture::Pinch { scale } => {
                if scale > 1.0 {
                    Some(&self.actions.pinch_out)
                } else {
                    Some(&self.actions.pinch_in)
                }
            }
            
            Gesture::Rotate { angle: _ } => {
                info!("Rotation gesture not yet implemented");
                None
            }
        };
        
        if let Some(action_command) = action {
            self.execute_command(action_command).await?;
        }
        
        Ok(())
    }
    
    /// Execute a shell command
    async fn execute_command(&self, command: &str) -> Result<()> {
        info!("Executing command: {}", command);
        
        // Handle special built-in commands
        match command {
            "click" => {
                self.simulate_click(1).await?;
            }
            "right_click" => {
                self.simulate_click(3).await?;
            }
            "scroll_vertical" => {
                // This would be handled differently - typically you'd send
                // synthetic scroll events to the system
                info!("Vertical scroll (implementation needed)");
            }
            "scroll_horizontal" => {
                info!("Horizontal scroll (implementation needed)");
            }
            _ => {
                // Execute as shell command
                let mut cmd = AsyncCommand::new("sh");
                cmd.arg("-c").arg(command);
                
                let output = cmd.output().await
                    .with_context(|| format!("Failed to execute command: {}", command))?;
                
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("Command failed: {}", stderr);
                } else {
                    info!("Command executed successfully");
                }
            }
        }
        
        Ok(())
    }
    
    /// Simulate a mouse click using xdotool
    async fn simulate_click(&self, button: u8) -> Result<()> {
        let mut cmd = AsyncCommand::new("xdotool");
        cmd.arg("click").arg(button.to_string());
        
        let output = cmd.output().await
            .context("Failed to execute xdotool click")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("xdotool click failed: {}", stderr);
        }
        
        Ok(())
    }
}

/// Check if required system tools are available
pub fn check_system_dependencies() -> Result<()> {
    let tools = ["xdotool"];
    
    for tool in &tools {
        let output = Command::new("which")
            .arg(tool)
            .output()
            .with_context(|| format!("Failed to check for {}", tool))?;
        
        if !output.status.success() {
            warn!("System tool '{}' not found. Some actions may not work.", tool);
            warn!("Install it with: sudo pacman -S xdotool  # On Arch Linux");
        } else {
            info!("Found system tool: {}", tool);
        }
    }
    
    Ok(())
}
