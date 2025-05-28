use evdev::{AbsoluteAxisType, EventType, InputEvent};
use log::{debug, trace};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::config::GestureConfig;

#[derive(Debug, Clone)]
pub struct TouchContact {
    pub id: i32,
    pub x: i32,
    pub y: i32,
    pub touch_major: i32,
    pub touch_minor: i32,
    pub orientation: i32,
    pub pressure: f64,
    pub first_contact_time: Instant,
    pub last_update_time: Instant,
    pub is_active: bool,
}

impl TouchContact {
    pub fn new(id: i32, x: i32, y: i32) -> Self {
        let now = Instant::now();
        Self {
            id,
            x,
            y,
            touch_major: 0,
            touch_minor: 0,
            orientation: 0,
            pressure: 0.0,
            first_contact_time: now,
            last_update_time: now,
            is_active: true,
        }
    }

    pub fn update_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        self.last_update_time = Instant::now();
    }

    pub fn update_touch_data(&mut self, major: i32, minor: i32, orientation: i32) {
        self.touch_major = major;
        self.touch_minor = minor;
        self.orientation = orientation;
        self.last_update_time = Instant::now();

        // Calculate pressure based on touch area (simplified)
        self.pressure = ((major + minor) as f64 / 2.0) / 1020.0 * 100.0;
    }

    pub fn distance_to(&self, other: &TouchContact) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn contact_duration(&self) -> Duration {
        self.last_update_time
            .duration_since(self.first_contact_time)
    }
}

#[derive(Debug, Clone)]
pub enum MultiTouchEvent {
    TwoFingerTap {
        finger1: TouchContact,
        finger2: TouchContact,
        duration_ms: u64,
    },
    SingleFingerTap {
        finger: TouchContact,
        duration_ms: u64,
    },
    TwoFingerSwipe {
        finger1: TouchContact,
        finger2: TouchContact,
        delta_x: f64,
        delta_y: f64,
    },
    Scroll {
        delta_x: f64,
        delta_y: f64,
    },
    Pinch {
        center_x: f64,
        center_y: f64,
        scale_factor: f64,
    },
    ContactStart {
        contact: TouchContact,
    },
    ContactUpdate {
        contact: TouchContact,
    },
    ContactEnd {
        contact: TouchContact,
    },
}

pub struct MultiTouchProcessor {
    config: GestureConfig,
    contacts: HashMap<i32, TouchContact>,
    current_slot: i32,
    pending_contacts: HashMap<i32, TouchContact>,
    last_gesture_time: Instant,
    gesture_state: GestureState,
}

#[derive(Debug, Clone)]
enum GestureState {
    Idle,
    SingleTouch {
        contact_id: i32,
    },
    TwoTouch {
        contact1_id: i32,
        contact2_id: i32,
        initial_distance: f64,
    },
    PotentialTap {
        contacts: Vec<i32>,
        start_time: Instant,
    },
}

impl MultiTouchProcessor {
    pub fn new(config: GestureConfig) -> Self {
        Self {
            config,
            contacts: HashMap::new(),
            current_slot: 0,
            pending_contacts: HashMap::new(),
            last_gesture_time: Instant::now(),
            gesture_state: GestureState::Idle,
        }
    }

    pub async fn process_event(&mut self, event: InputEvent) -> Option<Vec<MultiTouchEvent>> {
        trace!("Processing event: {:?}", event);

        match event.event_type() {
            EventType::ABSOLUTE => self.process_absolute_event(event).await,
            EventType::SYNCHRONIZATION => self.process_sync_event().await,
            _ => None,
        }
    }

    async fn process_absolute_event(&mut self, event: InputEvent) -> Option<Vec<MultiTouchEvent>> {
        let axis_type = AbsoluteAxisType(event.code());
        let value = event.value();

        match axis_type {
            AbsoluteAxisType::ABS_MT_SLOT => {
                self.current_slot = value;
                debug!("Switched to slot: {}", self.current_slot);
            }
            AbsoluteAxisType::ABS_MT_TRACKING_ID => {
                if value == -1 {
                    // Contact lifted
                    self.handle_contact_end(self.current_slot).await;
                } else {
                    // New contact
                    self.handle_contact_start(self.current_slot, value).await;
                }
            }
            AbsoluteAxisType::ABS_MT_POSITION_X => {
                self.update_contact_x(self.current_slot, value).await;
            }
            AbsoluteAxisType::ABS_MT_POSITION_Y => {
                self.update_contact_y(self.current_slot, value).await;
            }
            AbsoluteAxisType::ABS_MT_TOUCH_MAJOR => {
                self.update_contact_touch_major(self.current_slot, value)
                    .await;
            }
            AbsoluteAxisType::ABS_MT_TOUCH_MINOR => {
                self.update_contact_touch_minor(self.current_slot, value)
                    .await;
            }
            AbsoluteAxisType::ABS_MT_ORIENTATION => {
                self.update_contact_orientation(self.current_slot, value)
                    .await;
            }
            _ => {}
        }

        None
    }

    async fn process_sync_event(&mut self) -> Option<Vec<MultiTouchEvent>> {
        // Process all pending contact updates
        let mut events = Vec::new();

        // Check for gesture recognition
        if let Some(gesture_events) = self.detect_gestures().await {
            events.extend(gesture_events);
        }

        // Clean up expired contacts
        self.cleanup_expired_contacts().await;

        if events.is_empty() {
            None
        } else {
            Some(events)
        }
    }

    async fn handle_contact_start(&mut self, slot: i32, tracking_id: i32) {
        debug!("Contact start: slot={}, tracking_id={}", slot, tracking_id);

        let contact = TouchContact::new(tracking_id, 0, 0);
        self.pending_contacts.insert(slot, contact.clone());

        // Update gesture state
        match &self.gesture_state {
            GestureState::Idle => {
                self.gesture_state = GestureState::SingleTouch {
                    contact_id: tracking_id,
                };
            }
            GestureState::SingleTouch { contact_id } => {
                self.gesture_state = GestureState::TwoTouch {
                    contact1_id: *contact_id,
                    contact2_id: tracking_id,
                    initial_distance: 0.0, // Will be calculated when positions are available
                };
            }
            _ => {
                // More than 2 contacts or already in a gesture
                self.gesture_state = GestureState::Idle;
            }
        }
    }

    async fn handle_contact_end(&mut self, slot: i32) -> Option<Vec<MultiTouchEvent>> {
        debug!("Contact end: slot={}", slot);

        let mut events = Vec::new();

        if let Some(mut contact) = self.contacts.remove(&slot) {
            contact.is_active = false;

            // Check for tap gesture before contact ends
            if let Some(tap_events) = self.check_for_tap_gesture(&contact).await {
                events.extend(tap_events);
            }

            events.push(MultiTouchEvent::ContactEnd { contact });
        }

        self.pending_contacts.remove(&slot);

        // Update gesture state
        match &self.gesture_state {
            GestureState::SingleTouch { .. } | GestureState::TwoTouch { .. } => {
                if self.contacts.is_empty() {
                    self.gesture_state = GestureState::Idle;
                }
            }
            _ => {}
        }

        if events.is_empty() {
            None
        } else {
            Some(events)
        }
    }

    async fn update_contact_x(&mut self, slot: i32, x: i32) {
        if let Some(contact) = self.pending_contacts.get_mut(&slot) {
            contact.x = x;
        }
    }

    async fn update_contact_y(&mut self, slot: i32, y: i32) {
        if let Some(contact) = self.pending_contacts.get_mut(&slot) {
            contact.y = y;
        }
    }

    async fn update_contact_touch_major(&mut self, slot: i32, major: i32) {
        if let Some(contact) = self.pending_contacts.get_mut(&slot) {
            contact.touch_major = major;
            Self::update_pressure(contact);
        }
    }

    async fn update_contact_touch_minor(&mut self, slot: i32, minor: i32) {
        if let Some(contact) = self.pending_contacts.get_mut(&slot) {
            contact.touch_minor = minor;
            Self::update_pressure(contact);
        }
    }

    async fn update_contact_orientation(&mut self, slot: i32, orientation: i32) {
        if let Some(contact) = self.pending_contacts.get_mut(&slot) {
            contact.orientation = orientation;
            Self::update_pressure(contact);
        }
    }

    fn update_pressure(contact: &mut TouchContact) {
        // Calculate pressure based on touch area (simplified)
        if contact.touch_major > 0 || contact.touch_minor > 0 {
            contact.pressure = ((contact.touch_major + contact.touch_minor) as f64 / 2.0) / 10.0;
        }
    }

    async fn detect_gestures(&mut self) -> Option<Vec<MultiTouchEvent>> {
        let mut events = Vec::new();

        // Move pending contacts to active contacts
        for (slot, contact) in self.pending_contacts.drain() {
            self.contacts.insert(slot, contact.clone());
            events.push(MultiTouchEvent::ContactUpdate { contact });
        }

        // Detect two-finger tap
        if let Some(tap_events) = self.detect_two_finger_tap().await {
            events.extend(tap_events);
        }

        if events.is_empty() {
            None
        } else {
            Some(events)
        }
    }

    async fn detect_two_finger_tap(&mut self) -> Option<Vec<MultiTouchEvent>> {
        if self.contacts.len() != 2 {
            return None;
        }

        let contacts: Vec<&TouchContact> = self.contacts.values().collect();
        let contact1 = contacts[0];
        let contact2 = contacts[1];

        // Check if both contacts are recent (potential tap)
        let tap_timeout = Duration::from_millis(self.config.two_finger_tap_timeout_ms);

        let contact1_duration = contact1.contact_duration();
        let contact2_duration = contact2.contact_duration();

        // Both contacts should be short-lived for a tap
        if contact1_duration < tap_timeout && contact2_duration < tap_timeout {
            // Check distance between contacts
            let distance = contact1.distance_to(contact2);

            if distance < self.config.two_finger_tap_distance_threshold {
                // Check pressure threshold
                if contact1.pressure > self.config.contact_pressure_threshold
                    && contact2.pressure > self.config.contact_pressure_threshold
                {
                    debug!(
                        "Two-finger tap detected: distance={:.2}, pressures=({:.2}, {:.2})",
                        distance, contact1.pressure, contact2.pressure
                    );

                    return Some(vec![MultiTouchEvent::TwoFingerTap {
                        finger1: contact1.clone(),
                        finger2: contact2.clone(),
                        duration_ms: contact1_duration
                            .as_millis()
                            .max(contact2_duration.as_millis())
                            as u64,
                    }]);
                }
            }
        }

        None
    }

    async fn check_for_tap_gesture(&self, contact: &TouchContact) -> Option<Vec<MultiTouchEvent>> {
        let duration = contact.contact_duration();
        let tap_timeout = Duration::from_millis(self.config.tap_timeout_ms);

        if duration < tap_timeout && contact.pressure > self.config.contact_pressure_threshold {
            debug!(
                "Single-finger tap detected: duration={}ms, pressure={:.2}",
                duration.as_millis(),
                contact.pressure
            );

            return Some(vec![MultiTouchEvent::SingleFingerTap {
                finger: contact.clone(),
                duration_ms: duration.as_millis() as u64,
            }]);
        }

        None
    }

    async fn cleanup_expired_contacts(&mut self) {
        let now = Instant::now();
        let max_age = Duration::from_secs(1);

        self.contacts
            .retain(|_, contact| now.duration_since(contact.last_update_time) < max_age);
    }
}
