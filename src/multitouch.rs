use evdev::{AbsoluteAxisType, EventType, InputEvent, SynchronizationType};
use log::{debug, trace};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::config::GestureConfig;
use crate::gesture::GestureRecognizer;

/// Multi-touch processor that follows the Linux Multi-Touch Protocol Type B
///
/// This processor manages touch contacts using slots and tracking IDs as described in:
/// https://www.kernel.org/doc/Documentation/input/multi-touch-protocol.txt
pub struct MultiTouchProcessor {
    /// Currently active touch contacts, indexed by slot number
    active_contacts: HashMap<i32, TouchContact>,
    /// Pending contact updates during current sync cycle
    pending_contacts: HashMap<i32, TouchContact>,
    /// Current slot being updated
    current_slot: i32,
    /// Gesture recognizer
    gesture_recognizer: GestureRecognizer,
    /// Configuration
    config: GestureConfig,
    /// Last sync time for debouncing
    last_sync_time: Instant,
}

/// Represents a single touch contact with full lifecycle tracking
#[derive(Debug, Clone)]
pub struct TouchContact {
    /// Unique tracking ID assigned by kernel (-1 means contact ended)
    pub id: i32,
    /// Slot number (0-15 for Magic Mouse)
    pub slot: i32,
    /// X position (ABS_MT_POSITION_X)
    pub x: i32,
    /// Y position (ABS_MT_POSITION_Y)
    pub y: i32,
    /// Major axis of contact area (ABS_MT_TOUCH_MAJOR)
    pub touch_major: i32,
    /// Minor axis of contact area (ABS_MT_TOUCH_MINOR)
    pub touch_minor: i32,
    /// Contact orientation (ABS_MT_ORIENTATION)
    pub orientation: i32,
    /// When this contact was first established
    pub first_contact_time: Instant,
    /// Last time this contact was updated
    pub last_update_time: Instant,
    /// Whether this contact is currently active
    pub is_active: bool,
    /// Complete history of position changes for this contact
    pub position_history: Vec<(i32, i32, Instant)>,
}

/// Multi-touch events generated from raw input events
#[derive(Debug, Clone)]
pub enum MultiTouchEvent {
    /// A new contact has been established
    ContactStart { contact: TouchContact },
    /// An existing contact has been updated
    ContactUpdate { contact: TouchContact },
    /// A contact has ended
    ContactEnd { contact: TouchContact },
    /// Single finger tap gesture
    SingleFingerTap {
        finger: TouchContact,
        duration_ms: u64,
    },
    /// Two finger tap gesture
    TwoFingerTap {
        finger1: TouchContact,
        finger2: TouchContact,
        duration_ms: u64,
    },
    /// Two finger swipe gesture
    TwoFingerSwipe {
        finger1: TouchContact,
        finger2: TouchContact,
        delta_x: f64,
        delta_y: f64,
    },
    /// Pinch gesture
    Pinch {
        center_x: f64,
        center_y: f64,
        scale_factor: f64,
    },
}

impl TouchContact {
    /// Create a new touch contact
    fn new(id: i32, slot: i32) -> Self {
        let now = Instant::now();
        Self {
            id,
            slot,
            x: 0,
            y: 0,
            touch_major: 0,
            touch_minor: 0,
            orientation: 0,
            first_contact_time: now,
            last_update_time: now,
            is_active: true,
            position_history: Vec::new(),
        }
    }

    /// Update contact position and add to history
    fn update_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        self.last_update_time = Instant::now();
        self.position_history.push((x, y, self.last_update_time));

        // Keep position history manageable (last 100 updates)
        if self.position_history.len() > 100 {
            self.position_history.remove(0);
        }
    }

    /// Update touch area
    fn update_touch_area(&mut self, major: i32, minor: i32) {
        self.touch_major = major;
        self.touch_minor = minor;
        self.last_update_time = Instant::now();
    }

    /// Update orientation
    fn update_orientation(&mut self, orientation: i32) {
        self.orientation = orientation;
        self.last_update_time = Instant::now();
    }

    /// Get duration of this contact
    pub fn contact_duration(&self) -> Duration {
        self.last_update_time
            .duration_since(self.first_contact_time)
    }

    /// Calculate distance to another contact
    pub fn distance_to(&self, other: &TouchContact) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        (dx * dx + dy * dy).sqrt()
    }

    /// Get movement delta from start to current position
    pub fn movement_delta(&self) -> (f64, f64) {
        if let Some((start_x, start_y, _)) = self.position_history.first() {
            ((self.x - start_x) as f64, (self.y - start_y) as f64)
        } else {
            (0.0, 0.0)
        }
    }

    /// Check if this contact represents a tap (short duration, minimal movement)
    pub fn is_tap(&self, max_duration_ms: u64, max_movement: f64) -> bool {
        let duration = self.contact_duration();
        if duration.as_millis() as u64 > max_duration_ms {
            return false;
        }

        let (dx, dy) = self.movement_delta();
        let movement = (dx * dx + dy * dy).sqrt();
        movement <= max_movement
    }
}

impl MultiTouchProcessor {
    pub fn new(config: GestureConfig) -> Self {
        let gesture_recognizer = GestureRecognizer::new(
            config.swipe_threshold,
            config.pinch_threshold,
            config.scroll_threshold,
        );

        Self {
            active_contacts: HashMap::new(),
            pending_contacts: HashMap::new(),
            current_slot: 0,
            gesture_recognizer,
            config,
            last_sync_time: Instant::now(),
        }
    }

    /// Process a single evdev input event according to MT Protocol Type B
    pub async fn process_event(&mut self, event: InputEvent) -> Option<Vec<MultiTouchEvent>> {
        trace!("Processing event: {:?}", event);

        match event.event_type() {
            EventType::ABSOLUTE => self.handle_absolute_event(event),
            EventType::SYNCHRONIZATION => self.handle_sync_event(event).await,
            _ => None,
        }
    }

    /// Handle absolute axis events (ABS_MT_*)
    fn handle_absolute_event(&mut self, event: InputEvent) -> Option<Vec<MultiTouchEvent>> {
        let axis = AbsoluteAxisType(event.code());
        let value = event.value();

        match axis {
            AbsoluteAxisType::ABS_MT_SLOT => {
                // Switch to a different slot for subsequent updates
                self.current_slot = value;
                debug!("Switched to slot {}", value);
            }
            AbsoluteAxisType::ABS_MT_TRACKING_ID => {
                self.handle_tracking_id(value);
            }
            AbsoluteAxisType::ABS_MT_POSITION_X => {
                self.update_contact_x(value);
            }
            AbsoluteAxisType::ABS_MT_POSITION_Y => {
                self.update_contact_y(value);
            }
            AbsoluteAxisType::ABS_MT_TOUCH_MAJOR => {
                self.update_contact_touch_major(value);
            }
            AbsoluteAxisType::ABS_MT_TOUCH_MINOR => {
                self.update_contact_touch_minor(value);
            }
            AbsoluteAxisType::ABS_MT_ORIENTATION => {
                self.update_contact_orientation(value);
            }
            _ => {
                // Other absolute events we don't handle
            }
        }

        None // No events generated until sync
    }

    /// Handle tracking ID updates (contact creation/destruction)
    fn handle_tracking_id(&mut self, tracking_id: i32) {
        if tracking_id == -1 {
            // Contact ended - mark for removal
            if let Some(mut contact) = self.pending_contacts.remove(&self.current_slot) {
                contact.is_active = false;
                contact.last_update_time = Instant::now();
                self.pending_contacts.insert(self.current_slot, contact);
                debug!("Contact ended in slot {}", self.current_slot);
            }
        } else {
            // New contact or update - create or update contact
            let contact = self
                .pending_contacts
                .entry(self.current_slot)
                .or_insert_with(|| {
                    debug!("New contact {} in slot {}", tracking_id, self.current_slot);
                    TouchContact::new(tracking_id, self.current_slot)
                });

            contact.id = tracking_id;
            contact.is_active = true;
        }
    }

    /// Update X position for current slot
    fn update_contact_x(&mut self, x: i32) {
        if let Some(contact) = self.pending_contacts.get_mut(&self.current_slot) {
            let old_y = contact.y;
            contact.update_position(x, old_y);
        }
    }

    /// Update Y position for current slot
    fn update_contact_y(&mut self, y: i32) {
        if let Some(contact) = self.pending_contacts.get_mut(&self.current_slot) {
            let old_x = contact.x;
            contact.update_position(old_x, y);
        }
    }

    /// Update touch major axis for current slot
    fn update_contact_touch_major(&mut self, major: i32) {
        if let Some(contact) = self.pending_contacts.get_mut(&self.current_slot) {
            let minor = contact.touch_minor;
            contact.update_touch_area(major, minor);
        }
    }

    /// Update touch minor axis for current slot
    fn update_contact_touch_minor(&mut self, minor: i32) {
        if let Some(contact) = self.pending_contacts.get_mut(&self.current_slot) {
            let major = contact.touch_major;
            contact.update_touch_area(major, minor);
        }
    }

    /// Update orientation for current slot
    fn update_contact_orientation(&mut self, orientation: i32) {
        if let Some(contact) = self.pending_contacts.get_mut(&self.current_slot) {
            contact.update_orientation(orientation);
        }
    }

    /// Handle synchronization events (process accumulated changes)
    async fn handle_sync_event(&mut self, event: InputEvent) -> Option<Vec<MultiTouchEvent>> {
        if event.code() != SynchronizationType::SYN_REPORT.0 {
            return None;
        }

        // Debounce rapid sync events
        let now = Instant::now();
        if now.duration_since(self.last_sync_time).as_millis() < self.config.debounce_ms as u128 {
            return None;
        }
        self.last_sync_time = now;

        let mut events = Vec::new();

        // Process pending contacts and generate events
        for (slot, pending_contact) in self.pending_contacts.drain() {
            if pending_contact.is_active {
                // Contact is active
                if let Some(existing_contact) = self.active_contacts.get(&slot) {
                    // Update existing contact
                    if existing_contact.id != pending_contact.id
                        || existing_contact.x != pending_contact.x
                        || existing_contact.y != pending_contact.y
                    {
                        events.push(MultiTouchEvent::ContactUpdate {
                            contact: pending_contact.clone(),
                        });
                    }
                } else {
                    // New contact
                    events.push(MultiTouchEvent::ContactStart {
                        contact: pending_contact.clone(),
                    });
                }
                self.active_contacts.insert(slot, pending_contact);
            } else {
                // Contact ended
                if let Some(ended_contact) = self.active_contacts.remove(&slot) {
                    events.push(MultiTouchEvent::ContactEnd {
                        contact: ended_contact,
                    });
                }
            }
        }

        // Run gesture recognition on current active contacts
        let active_contact_list: Vec<TouchContact> =
            self.active_contacts.values().cloned().collect();
        if let Some(gesture_event) = self
            .gesture_recognizer
            .analyze_gesture(&active_contact_list)
        {
            events.push(gesture_event);
        }

        debug!("Generated {} events from sync", events.len());
        debug!("Active contacts: {}", self.active_contacts.len());

        if events.is_empty() {
            None
        } else {
            Some(events)
        }
    }

    /// Get current active contacts (for debugging)
    pub fn get_active_contacts(&self) -> &HashMap<i32, TouchContact> {
        &self.active_contacts
    }

    /// Get number of active contacts
    pub fn contact_count(&self) -> usize {
        self.active_contacts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> GestureConfig {
        GestureConfig {
            scroll_threshold: 50.0,
            swipe_threshold: 100.0,
            pinch_threshold: 0.1,
            tap_timeout_ms: 300,
            debounce_ms: 10,
            two_finger_tap_timeout_ms: 250,
            two_finger_tap_distance_threshold: 100.0,
            contact_pressure_threshold: 50.0,
        }
    }

    #[test]
    fn test_touch_contact_creation() {
        let contact = TouchContact::new(42, 0);
        assert_eq!(contact.id, 42);
        assert_eq!(contact.slot, 0);
        assert_eq!(contact.is_active, true);
    }

    #[test]
    fn test_distance_calculation() {
        let contact1 = TouchContact {
            x: 0,
            y: 0,
            ..TouchContact::new(1, 0)
        };
        let contact2 = TouchContact {
            x: 3,
            y: 4,
            ..TouchContact::new(2, 1)
        };

        assert_eq!(contact1.distance_to(&contact2), 5.0);
    }

    #[test]
    fn test_movement_delta() {
        let mut contact = TouchContact::new(1, 0);
        contact.update_position(10, 20);
        contact.update_position(15, 25);

        let (dx, dy) = contact.movement_delta();
        assert_eq!(dx, 5.0);
        assert_eq!(dy, 5.0);
    }

    #[tokio::test]
    async fn test_slot_switching() {
        let mut processor = MultiTouchProcessor::new(create_test_config());

        // Switch to slot 1
        let slot_event = InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_MT_SLOT.0, 1);
        processor.process_event(slot_event).await;
        assert_eq!(processor.current_slot, 1);

        // Switch to slot 0
        let slot_event = InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_MT_SLOT.0, 0);
        processor.process_event(slot_event).await;
        assert_eq!(processor.current_slot, 0);
    }

    #[tokio::test]
    async fn test_contact_lifecycle() {
        let mut processor = MultiTouchProcessor::new(create_test_config());

        // Start contact in slot 0
        let tracking_event = InputEvent::new(
            EventType::ABSOLUTE,
            AbsoluteAxisType::ABS_MT_TRACKING_ID.0,
            100,
        );
        processor.process_event(tracking_event).await;

        // Update position
        let x_event = InputEvent::new(
            EventType::ABSOLUTE,
            AbsoluteAxisType::ABS_MT_POSITION_X.0,
            500,
        );
        processor.process_event(x_event).await;
        let y_event = InputEvent::new(
            EventType::ABSOLUTE,
            AbsoluteAxisType::ABS_MT_POSITION_Y.0,
            300,
        );
        processor.process_event(y_event).await;

        // Sync to process changes
        let sync_event = InputEvent::new(
            EventType::SYNCHRONIZATION,
            SynchronizationType::SYN_REPORT.0,
            0,
        );
        let events = processor.process_event(sync_event).await.unwrap();

        // Should have contact start event
        assert_eq!(events.len(), 1);
        match &events[0] {
            MultiTouchEvent::ContactStart { contact } => {
                assert_eq!(contact.id, 100);
                assert_eq!(contact.x, 500);
                assert_eq!(contact.y, 300);
            }
            _ => panic!("Expected ContactStart event"),
        }

        // End contact
        let end_tracking_event = InputEvent::new(
            EventType::ABSOLUTE,
            AbsoluteAxisType::ABS_MT_TRACKING_ID.0,
            -1,
        );
        processor.process_event(end_tracking_event).await;

        // Sync to process changes
        let sync_event = InputEvent::new(
            EventType::SYNCHRONIZATION,
            SynchronizationType::SYN_REPORT.0,
            0,
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await; // Wait for debounce
        let events = processor.process_event(sync_event).await.unwrap();

        // Should have contact end event
        assert_eq!(events.len(), 1);
        match &events[0] {
            MultiTouchEvent::ContactEnd { contact } => {
                assert_eq!(contact.id, 100);
            }
            _ => panic!("Expected ContactEnd event"),
        }
    }
}
