use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::multitouch::{MultiTouchEvent, TouchContact};

pub struct GestureRecognizer {
    swipe_threshold: f64,
    pinch_threshold: f64,
    scroll_threshold: f64,
    gesture_history: VecDeque<GestureEvent>,
    max_history_size: usize,
}

#[derive(Debug, Clone)]
struct GestureEvent {
    timestamp: Instant,
    event_type: String,
    contacts: Vec<TouchContact>,
}

impl GestureRecognizer {
    pub fn new(swipe_threshold: f64, pinch_threshold: f64, scroll_threshold: f64) -> Self {
        Self {
            swipe_threshold,
            pinch_threshold,
            scroll_threshold,
            gesture_history: VecDeque::new(),
            max_history_size: 100,
        }
    }

    pub fn analyze_gesture(&mut self, contacts: &[TouchContact]) -> Option<MultiTouchEvent> {
        match contacts.len() {
            1 => self.analyze_single_finger(contacts),
            2 => self.analyze_two_finger(contacts),
            _ => None,
        }
    }

    fn analyze_single_finger(&self, contacts: &[TouchContact]) -> Option<MultiTouchEvent> {
        let contact = &contacts[0];

        // Check for single tap
        let duration = contact.contact_duration();
        if duration < Duration::from_millis(300) && !contact.is_active {
            return Some(MultiTouchEvent::SingleFingerTap {
                finger: contact.clone(),
                duration_ms: duration.as_millis() as u64,
            });
        }

        None
    }

    fn analyze_two_finger(&self, contacts: &[TouchContact]) -> Option<MultiTouchEvent> {
        let contact1 = &contacts[0];
        let contact2 = &contacts[1];

        // Calculate center point
        let center_x = (contact1.x + contact2.x) as f64 / 2.0;
        let center_y = (contact1.y + contact2.y) as f64 / 2.0;

        // Calculate distance between fingers
        let distance = contact1.distance_to(contact2);

        // Check for two-finger tap
        if self.is_two_finger_tap(contact1, contact2) {
            let max_duration = contact1.contact_duration().max(contact2.contact_duration());
            return Some(MultiTouchEvent::TwoFingerTap {
                finger1: contact1.clone(),
                finger2: contact2.clone(),
                duration_ms: max_duration.as_millis() as u64,
            });
        }

        // Check for pinch gesture
        if let Some(scale_factor) = self.detect_pinch(contact1, contact2) {
            return Some(MultiTouchEvent::Pinch {
                center_x,
                center_y,
                scale_factor,
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

        // Check for scroll gesture
        if let Some((delta_x, delta_y)) = self.detect_scroll(contact1, contact2) {
            return Some(MultiTouchEvent::Scroll { delta_x, delta_y });
        }

        None
    }

    fn is_two_finger_tap(&self, contact1: &TouchContact, contact2: &TouchContact) -> bool {
        let duration1 = contact1.contact_duration();
        let duration2 = contact2.contact_duration();
        let max_tap_duration = Duration::from_millis(250);

        // Both contacts should be short-lived
        if duration1 > max_tap_duration || duration2 > max_tap_duration {
            return false;
        }

        // Contacts should be close together
        let distance = contact1.distance_to(contact2);
        if distance > 100.0 {
            return false;
        }

        // Both contacts should have sufficient pressure
        if contact1.pressure < 50.0 || contact2.pressure < 50.0 {
            return false;
        }

        // Contacts should start around the same time
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

    fn detect_pinch(&self, contact1: &TouchContact, contact2: &TouchContact) -> Option<f64> {
        // For now, return None - would need gesture history to implement properly
        // This would require tracking the initial distance and comparing to current distance
        None
    }

    fn detect_swipe(&self, contact1: &TouchContact, contact2: &TouchContact) -> Option<(f64, f64)> {
        // For now, return None - would need gesture history to implement properly
        // This would require tracking movement over time
        None
    }

    fn detect_scroll(
        &self,
        contact1: &TouchContact,
        contact2: &TouchContact,
    ) -> Option<(f64, f64)> {
        // For now, return None - would need gesture history to implement properly
        // This would require tracking small movements over time
        None
    }

    fn add_to_history(&mut self, event_type: String, contacts: Vec<TouchContact>) {
        let event = GestureEvent {
            timestamp: Instant::now(),
            event_type,
            contacts,
        };

        self.gesture_history.push_back(event);

        // Keep history size manageable
        while self.gesture_history.len() > self.max_history_size {
            self.gesture_history.pop_front();
        }
    }

    pub fn clear_history(&mut self) {
        self.gesture_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_finger_tap_detection() {
        let mut recognizer = GestureRecognizer::new(100.0, 0.1, 50.0);

        // Create two close contacts with short duration
        let contact1 = TouchContact {
            id: 1,
            x: 100,
            y: 100,
            touch_major: 100,
            touch_minor: 100,
            orientation: 0,
            pressure: 75.0,
            first_contact_time: Instant::now(),
            last_update_time: Instant::now(),
            is_active: false,
        };

        let contact2 = TouchContact {
            id: 2,
            x: 120,
            y: 110,
            touch_major: 90,
            touch_minor: 90,
            orientation: 0,
            pressure: 80.0,
            first_contact_time: Instant::now(),
            last_update_time: Instant::now(),
            is_active: false,
        };

        let contacts = vec![contact1, contact2];

        if let Some(MultiTouchEvent::TwoFingerTap { .. }) = recognizer.analyze_gesture(&contacts) {
            // Test passed
        } else {
            panic!("Expected two-finger tap detection");
        }
    }
}
