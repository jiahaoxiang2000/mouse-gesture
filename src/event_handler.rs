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
                finger1: _,
                finger2: _,
                duration_ms,
            } => {
                info!("Two-finger tap detected ({}ms)", duration_ms);
                self.execute_action("tap_2finger").await?;
            }
            MultiTouchEvent::SingleFingerTap {
                finger: _,
                duration_ms,
            } => {
                info!("Single-finger tap detected ({}ms)", duration_ms);
                self.execute_action("tap_1finger").await?;
            }
            MultiTouchEvent::TwoFingerSwipe {
                finger1: _,
                finger2: _,
                delta_x,
                delta_y,
            } => {
                let direction = self.determine_swipe_direction(delta_x, delta_y);
                info!("Two-finger swipe detected: {}", direction);
                self.execute_action(&format!("swipe_{}_2finger", direction))
                    .await?;
            }
            MultiTouchEvent::Pinch {
                center_x: _,
                center_y: _,
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
                    "Contact updated: id={}, pos=({}, {})",
                    contact.id, contact.x, contact.y
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
