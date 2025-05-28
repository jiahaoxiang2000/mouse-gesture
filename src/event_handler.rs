use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::process::Stdio;
use tokio::process::Command;

use crate::config::Config;
use crate::multitouch::MultiTouchEvent;

pub struct EventHandler {
    pub config: Config,
}

impl EventHandler {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn handle_multitouch_event(&self, event: MultiTouchEvent) -> Result<()> {
        match event {
            MultiTouchEvent::TwoFingerTap {
                finger1,
                finger2,
                duration_ms,
            } => {
                info!("Two-finger tap detected ({}ms)", duration_ms);
                self.execute_action("tap_2finger").await?;
            }
            MultiTouchEvent::SingleFingerTap {
                finger,
                duration_ms,
            } => {
                info!("Single-finger tap detected ({}ms)", duration_ms);
                self.execute_action("tap_1finger").await?;
            }
            MultiTouchEvent::TwoFingerSwipe {
                finger1,
                finger2,
                delta_x,
                delta_y,
            } => {
                let direction = self.determine_swipe_direction(delta_x, delta_y);
                info!("Two-finger swipe detected: {}", direction);
                self.execute_action(&format!("swipe_{}_2finger", direction))
                    .await?;
            }
            MultiTouchEvent::Scroll { delta_x, delta_y } => {
                if delta_y.abs() > delta_x.abs() {
                    self.execute_scroll("vertical", delta_y).await?;
                } else {
                    self.execute_scroll("horizontal", delta_x).await?;
                }
            }
            MultiTouchEvent::Pinch {
                center_x,
                center_y,
                scale_factor,
            } => {
                let action = if scale_factor > 1.0 {
                    "pinch_out"
                } else {
                    "pinch_in"
                };
                info!("Pinch gesture detected: scale={:.2}", scale_factor);
                self.execute_action(action).await?;
            }
            MultiTouchEvent::ContactStart { contact } => {
                debug!(
                    "Contact started: id={}, pos=({}, {})",
                    contact.id, contact.x, contact.y
                );
            }
            MultiTouchEvent::ContactUpdate { contact } => {
                debug!(
                    "Contact updated: id={}, pos=({}, {}), pressure={:.2}",
                    contact.id, contact.x, contact.y, contact.pressure
                );
            }
            MultiTouchEvent::ContactEnd { contact } => {
                debug!(
                    "Contact ended: id={}, duration={}ms",
                    contact.id,
                    contact.contact_duration().as_millis()
                );
            }
        }

        Ok(())
    }

    async fn execute_action(&self, action_name: &str) -> Result<()> {
        if let Some(command) = self.config.actions.get(action_name) {
            match command.as_str() {
                "click" => self.simulate_click(1).await?,
                "right_click" => self.simulate_click(3).await?,
                "middle_click" => self.simulate_click(2).await?,
                _ => self.execute_shell_command(command).await?,
            }
        } else {
            warn!("No action configured for: {}", action_name);
        }

        Ok(())
    }

    async fn execute_scroll(&self, direction: &str, delta: f64) -> Result<()> {
        let action_name = format!("scroll_{}", direction);

        if let Some(command) = self.config.actions.get(&action_name) {
            if command == &action_name {
                // Built-in scroll handling
                self.simulate_scroll(direction, delta).await?;
            } else {
                // Custom command
                self.execute_shell_command(command).await?;
            }
        } else {
            // Default scroll behavior
            self.simulate_scroll(direction, delta).await?;
        }

        Ok(())
    }

    async fn simulate_click(&self, button: u8) -> Result<()> {
        debug!("Simulating mouse click: button {}", button);

        let output = Command::new("xdotool")
            .args(&["click", &button.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute xdotool click")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("xdotool click failed: {}", stderr);
        }

        Ok(())
    }

    async fn simulate_scroll(&self, direction: &str, delta: f64) -> Result<()> {
        debug!("Simulating scroll: {} delta={:.2}", direction, delta);

        let (button, steps) = match direction {
            "vertical" => {
                if delta > 0.0 {
                    ("4", (delta / 50.0).ceil() as i32) // Scroll up
                } else {
                    ("5", (-delta / 50.0).ceil() as i32) // Scroll down
                }
            }
            "horizontal" => {
                if delta > 0.0 {
                    ("6", (delta / 50.0).ceil() as i32) // Scroll right
                } else {
                    ("7", (-delta / 50.0).ceil() as i32) // Scroll left
                }
            }
            _ => return Ok(()),
        };

        for _ in 0..steps.min(10) {
            // Limit to 10 steps to prevent spam
            let output = Command::new("xdotool")
                .args(&["click", button])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
                .await
                .context("Failed to execute xdotool scroll")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("xdotool scroll failed: {}", stderr);
                break;
            }

            // Small delay between scroll steps
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    async fn execute_shell_command(&self, command: &str) -> Result<()> {
        debug!("Executing shell command: {}", command);

        let output = Command::new("sh")
            .args(&["-c", command])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute shell command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Shell command failed: {} - Error: {}", command, stderr);
        }

        Ok(())
    }

    fn determine_swipe_direction(&self, delta_x: f64, delta_y: f64) -> &'static str {
        if delta_x.abs() > delta_y.abs() {
            if delta_x > 0.0 {
                "right"
            } else {
                "left"
            }
        } else {
            if delta_y > 0.0 {
                "down"
            } else {
                "up"
            }
        }
    }
}
