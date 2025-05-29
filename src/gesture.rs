use log::debug;

use crate::multitouch::{MultiTouchEvent, TouchContact};
use std::time::{Duration, Instant};

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
        debug!("Analyzing single finger contact: {:?}", contact);

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
            return Some(MultiTouchEvent::TwoFingerTap {
                finger1: contact1.clone(),
                finger2: contact2.clone(),
                duration_ms: max_duration.as_millis() as u64,
            });
        }

        // Check for swipe gesture
        if let Some((delta_x, delta_y)) = self.detect_swipe(contact1, contact2) {
            return Some(MultiTouchEvent::TwoFingerSwipe {
                finger1: contact1.clone(),
                finger2: contact2.clone(),
                delta_x,
                delta_y,
            });
        }

        // Check for pinch gesture
        if let Some(scale_factor) = self.detect_pinch(contact1, contact2) {
            let center_x = (contact1.x + contact2.x) as f64 / 2.0;
            let center_y = (contact1.y + contact2.y) as f64 / 2.0;
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
        // Both contacts must be inactive (completed gestures)
        if contact1.is_active || contact2.is_active {
            return false;
        }

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
        // Only detect swipes on inactive contacts (completed gestures)
        if contact1.is_active || contact2.is_active {
            return None;
        }

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

    /// Detect pinch gestures (placeholder for future implementation)
    fn detect_pinch(&self, _contact1: &TouchContact, _contact2: &TouchContact) -> Option<f64> {
        // Pinch detection requires tracking distance changes over time
        // This would need gesture history to implement properly
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_finger_tap_detection() {
        let mut recognizer = GestureRecognizer::new(
            100.0, // swipe_threshold
            0.1,   // pinch_threshold
            50.0,  // scroll_threshold
            300,   // tap_timeout_ms
            50.0,  // single_finger_tap_movement_threshold
            250,   // two_finger_tap_timeout_ms
            100.0, // two_finger_tap_distance_threshold
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
}
