use evdev::{AbsoluteAxisType, EventType, InputEvent, Synchronization};
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
    /// Pending contact updates during current sync cycle
    pending_contacts: HashMap<i32, TouchContact>,
    /// Completed contacts waiting for gesture recognition
    completed_contacts: Vec<TouchContact>,
    /// Number of currently active contacts
    active_contact_count: usize,
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
            pending_contacts: HashMap::new(),
            completed_contacts: Vec::new(),
            active_contact_count: 0,
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
                return self.handle_tracking_id(value);
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
    fn handle_tracking_id(&mut self, tracking_id: i32) -> Option<Vec<MultiTouchEvent>> {
        if tracking_id == -1 {
            // Contact ended - immediately trigger gesture recognition
            if let Some(mut contact) = self.pending_contacts.remove(&self.current_slot) {
                contact.is_active = false;
                contact.last_update_time = Instant::now();
                self.completed_contacts.push(contact);
                self.active_contact_count = self.active_contact_count.saturating_sub(1);

                debug!(
                    "Contact ended in slot {}, active contacts: {}",
                    self.current_slot, self.active_contact_count
                );

                // Trigger gesture recognition immediately if no more active contacts
                if self.active_contact_count == 0 && !self.completed_contacts.is_empty() {
                    debug!(
                        "All contacts ended, running gesture recognition on {} contacts",
                        self.completed_contacts.len()
                    );

                    if let Some(gesture_event) = self
                        .gesture_recognizer
                        .analyze_gesture(&self.completed_contacts)
                    {
                        self.completed_contacts.clear();
                        return Some(vec![gesture_event]);
                    }

                    // Clear completed contacts even if no gesture was recognized
                    self.completed_contacts.clear();
                }
            }
        } else {
            // New contact or update
            let is_new_contact = !self.pending_contacts.contains_key(&self.current_slot);
            let contact = self
                .pending_contacts
                .entry(self.current_slot)
                .or_insert_with(|| {
                    debug!("New contact {} in slot {}", tracking_id, self.current_slot);
                    TouchContact::new(tracking_id, self.current_slot)
                });

            contact.id = tracking_id;
            contact.is_active = true;

            if is_new_contact {
                self.active_contact_count += 1;
                debug!(
                    "New contact started, active contacts: {}",
                    self.active_contact_count
                );
            }
        }

        None
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
        if event.code() != Synchronization::SYN_REPORT.0 {
            return None;
        }
        // Note: here we logic justing is based on the Track ID and Slot.
        let now = Instant::now();
        self.last_sync_time = now;

        None
    }
}

#[cfg(test)]
mod tests {}
