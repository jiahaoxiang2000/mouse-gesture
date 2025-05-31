use log::{debug, trace};

use crate::multitouch::{MultiTouchEvent, TouchContact};
use std::time::{Duration, Instant};

// Magic Mouse 2 USB-C 2024 hardware specifications
// Based on evtest output showing resolution values
const MAGIC_MOUSE_X_RESOLUTION: f64 = 26.0; // units per mm
const MAGIC_MOUSE_Y_RESOLUTION: f64 = 70.0; // units per mm

/// Convert Magic Mouse X coordinate units to millimeters
fn units_to_mm_x(units: i32) -> f64 {
    units as f64 / MAGIC_MOUSE_X_RESOLUTION
}

/// Convert Magic Mouse Y coordinate units to millimeters
fn units_to_mm_y(units: i32) -> f64 {
    units as f64 / MAGIC_MOUSE_Y_RESOLUTION
}

/// Gesture recognizer focused on multi-touch tap and swipe detection
pub struct GestureRecognizer {
    swipe_threshold: f64,
    pinch_threshold: f64,
    tap_timeout_ms: u64,
    single_finger_tap_movement_threshold: f64,
    two_finger_tap_timeout_ms: u64,
    two_finger_tap_distance_threshold: f64,
}

impl GestureRecognizer {
    pub fn new(
        swipe_threshold: f64,
        pinch_threshold: f64,
        _scroll_threshold: f64,
        tap_timeout_ms: u64,
        single_finger_tap_movement_threshold: f64,
        two_finger_tap_timeout_ms: u64,
        two_finger_tap_distance_threshold: f64,
    ) -> Self {
        Self {
            swipe_threshold,
            pinch_threshold,
            tap_timeout_ms,
            single_finger_tap_movement_threshold,
            two_finger_tap_timeout_ms,
            two_finger_tap_distance_threshold,
        }
    }

    /// Analyze contacts and detect gestures
    pub fn analyze_gesture(&mut self, contacts: &[TouchContact]) -> Option<MultiTouchEvent> {
        debug!("Analyzing {} contacts for gestures", contacts.len());
        match contacts.len() {
            1 => self.analyze_single_finger(contacts),
            2 => self.analyze_two_finger(contacts),
            _ => None,
        }
    }

    /// Detect single finger gestures (primarily taps)
    fn analyze_single_finger(&self, contacts: &[TouchContact]) -> Option<MultiTouchEvent> {
        let contact = &contacts[0];
        // Check for single tap - short duration and contact is no longer active
        if !contact.is_active
            && contact.is_tap(
                self.tap_timeout_ms,
                self.single_finger_tap_movement_threshold,
            )
        {
            return Some(MultiTouchEvent::SingleFingerTap {
                finger: contact.clone(),
                duration_ms: contact.contact_duration().as_millis() as u64,
            });
        }

        None
    }

    /// Detect two finger gestures (taps, swipes, pinch)
    fn analyze_two_finger(&self, contacts: &[TouchContact]) -> Option<MultiTouchEvent> {
        let contact1 = &contacts[0];
        let contact2 = &contacts[1];

        // Check for two-finger tap first (highest priority)
        if self.is_two_finger_tap(contact1, contact2) {
            let max_duration = contact1.contact_duration().max(contact2.contact_duration());
            trace!(
                "Detected two-finger tap: duration_ms = {}",
                max_duration.as_millis()
            );
            return Some(MultiTouchEvent::TwoFingerTap {
                finger1: contact1.clone(),
                finger2: contact2.clone(),
                duration_ms: max_duration.as_millis() as u64,
            });
        }

        // Check for swipe gesture
        if let Some((delta_x, delta_y)) = self.detect_swipe(contact1, contact2) {
            trace!(
                "Detected two-finger swipe: delta_x = {}, delta_y = {}",
                delta_x,
                delta_y
            );
            return Some(MultiTouchEvent::TwoFingerSwipe {
                finger1: contact1.clone(),
                finger2: contact2.clone(),
                delta_x,
                delta_y,
            });
        }

        // Check for pinch gesture
        if let Some(scale_factor) = self.detect_pinch(contact1, contact2) {
            let center_x = (units_to_mm_x(contact1.x) + units_to_mm_x(contact2.x)) / 2.0;
            let center_y = (units_to_mm_y(contact1.y) + units_to_mm_y(contact2.y)) / 2.0;
            trace!(
                "Detected pinch gesture: center_x = {}, center_y = {}, scale_factor = {}",
                center_x,
                center_y,
                scale_factor
            );
            return Some(MultiTouchEvent::Pinch {
                center_x,
                center_y,
                scale_factor,
            });
        }

        None
    }

    /// Detect two-finger tap based on Linux Multi-Touch Protocol requirements
    fn is_two_finger_tap(&self, contact1: &TouchContact, contact2: &TouchContact) -> bool {
        // Short duration requirement
        let max_tap_duration = Duration::from_millis(self.two_finger_tap_timeout_ms);
        if contact1.contact_duration() > max_tap_duration
            || contact2.contact_duration() > max_tap_duration
        {
            return false;
        }

        // Close proximity requirement
        let distance = contact1.distance_to(contact2);
        if distance > self.two_finger_tap_distance_threshold {
            return false;
        }

        // Simultaneous start requirement
        let time_diff = if contact1.first_contact_time > contact2.first_contact_time {
            contact1
                .first_contact_time
                .duration_since(contact2.first_contact_time)
        } else {
            contact2
                .first_contact_time
                .duration_since(contact1.first_contact_time)
        };

        time_diff < Duration::from_millis(100)
    }

    /// Detect swipe gestures based on movement delta
    fn detect_swipe(&self, contact1: &TouchContact, contact2: &TouchContact) -> Option<(f64, f64)> {
        let (dx1, dy1) = contact1.movement_delta();
        let (dx2, dy2) = contact2.movement_delta();

        // Average movement of both fingers
        let avg_dx = (dx1 + dx2) / 2.0;
        let avg_dy = (dy1 + dy2) / 2.0;

        let movement_magnitude = (avg_dx * avg_dx + avg_dy * avg_dy).sqrt();

        if movement_magnitude > self.swipe_threshold {
            Some((avg_dx, avg_dy))
        } else {
            None
        }
    }

    /// Detect pinch gestures based on distance changes between two contacts over time
    fn detect_pinch(&self, contact1: &TouchContact, contact2: &TouchContact) -> Option<f64> {
        // Need at least 3 position samples to calculate meaningful distance changes
        if contact1.position_history.len() < 3 || contact2.position_history.len() < 3 {
            return None;
        }

        // Calculate initial distance (using early positions, skipping the (0,0) initialization)
        let initial_pos1 = if contact1.position_history.len() >= 3 {
            contact1.position_history[2] // Skip the (0,0) initialization
        } else {
            contact1.position_history[1]
        };

        let initial_pos2 = if contact2.position_history.len() >= 3 {
            contact2.position_history[2] // Skip the (0,0) initialization
        } else {
            contact2.position_history[1]
        };

        let initial_distance = {
            let dx_mm = units_to_mm_x(initial_pos1.0) - units_to_mm_x(initial_pos2.0);
            let dy_mm = units_to_mm_y(initial_pos1.1) - units_to_mm_y(initial_pos2.1);
            (dx_mm * dx_mm + dy_mm * dy_mm).sqrt()
        };

        // Calculate current distance
        let current_distance = contact1.distance_to(contact2);

        // Avoid division by zero and ensure minimum meaningful distance
        if initial_distance < 0.5 {
            // 0.5mm minimum distance
            return None;
        }

        // Calculate scale factor (ratio of current to initial distance)
        let scale_factor = current_distance / initial_distance;

        // Check if the scale change is significant enough to be considered a pinch
        // Scale factor < 1.0 means pinch in (zoom out)
        // Scale factor > 1.0 means pinch out (zoom in)
        let scale_change = (scale_factor - 1.0).abs();

        if scale_change > self.pinch_threshold {
            Some(scale_factor)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_finger_tap_detection() {
        let mut recognizer = GestureRecognizer::new(
            12.0, // swipe_threshold (mm)
            0.1,  // pinch_threshold
            2.0,  // scroll_threshold (mm)
            300,  // tap_timeout_ms
            2.0,  // single_finger_tap_movement_threshold (mm)
            250,  // two_finger_tap_timeout_ms
            30.0, // two_finger_tap_distance_threshold (mm)
        );

        // Create two close contacts with short duration
        let contact1 = TouchContact {
            id: 1,
            slot: 0,
            x: 100,
            y: 100,
            touch_major: 100,
            touch_minor: 100,
            orientation: 0,
            first_contact_time: Instant::now(),
            last_update_time: Instant::now(),
            is_active: false,
            position_history: vec![(100, 100, Instant::now())],
        };

        let contact2 = TouchContact {
            id: 2,
            slot: 1,
            x: 120,
            y: 110,
            touch_major: 90,
            touch_minor: 90,
            orientation: 0,
            first_contact_time: Instant::now(),
            last_update_time: Instant::now(),
            is_active: false,
            position_history: vec![(120, 110, Instant::now())],
        };

        let contacts = vec![contact1, contact2];

        if let Some(MultiTouchEvent::TwoFingerTap { .. }) = recognizer.analyze_gesture(&contacts) {
            // Test passed
        } else {
            panic!("Expected two-finger tap detection");
        }
    }

    #[test]
    fn test_pinch_detection() {
        // Initialize debug logging for the test
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

        let mut recognizer = GestureRecognizer::new(
            12.0, // swipe_threshold (mm)
            0.2,  // pinch_threshold (20% change)
            2.0,  // scroll_threshold (mm)
            300,  // tap_timeout_ms
            2.0,  // single_finger_tap_movement_threshold (mm)
            250,  // two_finger_tap_timeout_ms
            30.0, // two_finger_tap_distance_threshold (mm)
        );

        let now = Instant::now();
        let time1 = now;
        let time2 = now + Duration::from_millis(250);
        let time3 = now + Duration::from_millis(500);
        let time4 = now + Duration::from_millis(750);

        // Create two contacts that start close and move apart (pinch out)
        let contact1 = TouchContact {
            id: 1,
            slot: 0,
            x: 150, // Moved further apart
            y: 150,
            touch_major: 100,
            touch_minor: 100,
            orientation: 0,
            first_contact_time: time1,
            last_update_time: time4,
            is_active: true,
            position_history: vec![
                (0, 0, time1),     // Initial (0,0) position
                (100, 100, time2), // First real position
                (110, 110, time3), // Early position
                (150, 150, time4), // Final position (moved apart)
            ],
        };

        let contact2 = TouchContact {
            id: 2,
            slot: 1,
            x: 50, // Moved in opposite direction
            y: 50,
            touch_major: 90,
            touch_minor: 90,
            orientation: 0,
            first_contact_time: time1,
            last_update_time: time4,
            is_active: true,
            position_history: vec![
                (0, 0, time1),     // Initial (0,0) position
                (100, 100, time2), // First real position (same as contact1)
                (90, 90, time3),   // Early position
                (50, 50, time4),   // Final position (moved apart)
            ],
        };

        let contacts = vec![contact1.clone(), contact2.clone()];

        if let Some(MultiTouchEvent::Pinch { scale_factor, .. }) =
            recognizer.analyze_gesture(&contacts)
        {
            // Should detect pinch out (scale_factor > 1.0)
            assert!(
                scale_factor > 1.0,
                "Expected pinch out with scale factor > 1.0, got {}",
                scale_factor
            );
        } else {
            // Debug: let's see what the actual distances are
            let initial_pos1 = contact1.position_history[2];
            let initial_pos2 = contact2.position_history[2];
            let initial_dx = units_to_mm_x(initial_pos1.0) - units_to_mm_x(initial_pos2.0);
            let initial_dy = units_to_mm_y(initial_pos1.1) - units_to_mm_y(initial_pos2.1);
            let initial_distance = (initial_dx * initial_dx + initial_dy * initial_dy).sqrt();
            let current_distance = contact1.distance_to(&contact2);
            let scale_factor = current_distance / initial_distance;
            let scale_change = (scale_factor - 1.0).abs();

            panic!("Expected pinch detection. Initial distance: {:.3}mm, Current distance: {:.3}mm, Scale factor: {:.3}, Scale change: {:.3}, Threshold: {:.3}", 
                   initial_distance, current_distance, scale_factor, scale_change, recognizer.pinch_threshold);
        }
    }
}
