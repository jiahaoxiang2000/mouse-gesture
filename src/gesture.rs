use crate::device::{MouseEvent, TouchPoint};
use crate::config::GestureConfig;
use log::debug;
use nalgebra::{Point2, Vector2};
use std::collections::HashMap;
use std::time::{Instant, Duration};

/// Recognized gesture types
#[derive(Debug, Clone, PartialEq)]
pub enum Gesture {
    Swipe { direction: SwipeDirection, fingers: u8 },
    Scroll { direction: ScrollDirection, delta: f32 },
    Tap { fingers: u8, position: Point2<f32> },
    Pinch { scale: f32 },
    Rotate { angle: f32 },
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum SwipeDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
}

/// Tracks touch state for gesture recognition
#[derive(Debug, Clone)]
struct TouchState {
    start_position: Point2<f32>,
    current_position: Point2<f32>,
    start_time: Instant,
    last_update: Instant,
}

/// Main gesture recognizer
pub struct GestureRecognizer {
    config: GestureConfig,
    active_touches: HashMap<i32, TouchState>,
    gesture_start_time: Option<Instant>,
    last_gesture: Option<Gesture>,
}

impl GestureRecognizer {
    /// Create a new gesture recognizer
    pub fn new(config: GestureConfig) -> Self {
        Self {
            config,
            active_touches: HashMap::new(),
            gesture_start_time: None,
            last_gesture: None,
        }
    }
    
    /// Process a mouse event and potentially return a recognized gesture
    pub fn process_event(&mut self, event: MouseEvent) -> Option<Gesture> {
        match event {
            MouseEvent::TouchStart { point } => {
                self.handle_touch_start(point);
                None
            }
            
            MouseEvent::TouchMove { point } => {
                self.handle_touch_move(point)
            }
            
            MouseEvent::TouchEnd { tracking_id } => {
                self.handle_touch_end(tracking_id)
            }
            
            MouseEvent::Button { button: _, pressed: _ } => {
                // Handle button events if needed
                None
            }
            
            MouseEvent::Movement { dx: _, dy: _ } => {
                // Handle relative movement if needed
                None
            }
        }
    }
    
    fn handle_touch_start(&mut self, point: TouchPoint) {
        let position = Point2::new(point.x as f32, point.y as f32);
        let now = Instant::now();
        
        let touch_state = TouchState {
            start_position: position,
            current_position: position,
            start_time: now,
            last_update: now,
        };
        
        self.active_touches.insert(point.tracking_id, touch_state);
        
        if self.gesture_start_time.is_none() {
            self.gesture_start_time = Some(now);
        }
        
        debug!("Touch started: ID {}, position: {:?}", point.tracking_id, position);
    }
    
    fn handle_touch_move(&mut self, point: TouchPoint) -> Option<Gesture> {
        let position = Point2::new(point.x as f32, point.y as f32);
        let now = Instant::now();
        
        if let Some(touch_state) = self.active_touches.get_mut(&point.tracking_id) {
            touch_state.current_position = position;
            touch_state.last_update = now;
            
            debug!("Touch moved: ID {}, position: {:?}", point.tracking_id, position);
            
            // Check for ongoing gestures
            self.detect_gesture()
        } else {
            None
        }
    }
    
    fn handle_touch_end(&mut self, tracking_id: i32) -> Option<Gesture> {
        if let Some(_touch_state) = self.active_touches.remove(&tracking_id) {
            debug!("Touch ended: ID {}", tracking_id);
            
            // If this was the last touch, finalize gesture detection
            if self.active_touches.is_empty() {
                let gesture = self.finalize_gesture();
                self.gesture_start_time = None;
                return gesture;
            }
        }
        
        None
    }
    
    fn detect_gesture(&mut self) -> Option<Gesture> {
        let touch_count = self.active_touches.len();
        
        match touch_count {
            1 => self.detect_single_finger_gesture(),
            2 => self.detect_two_finger_gesture(),
            _ => None,
        }
    }
    
    fn detect_single_finger_gesture(&self) -> Option<Gesture> {
        if let Some((_, touch)) = self.active_touches.iter().next() {
            let movement = touch.current_position - touch.start_position;
            let distance = movement.magnitude();
            
            // Scroll detection
            if distance > self.config.scroll_threshold {
                let direction = if movement.y.abs() > movement.x.abs() {
                    ScrollDirection::Vertical
                } else {
                    ScrollDirection::Horizontal
                };
                
                return Some(Gesture::Scroll {
                    direction,
                    delta: distance,
                });
            }
        }
        
        None
    }
    
    fn detect_two_finger_gesture(&self) -> Option<Gesture> {
        if self.active_touches.len() != 2 {
            return None;
        }
        
        let touches: Vec<&TouchState> = self.active_touches.values().collect();
        let touch1 = touches[0];
        let touch2 = touches[1];
        
        // Calculate center point movement for swipe detection
        let start_center = (touch1.start_position + touch2.start_position.coords) / 2.0;
        let current_center = (touch1.current_position + touch2.current_position.coords) / 2.0;
        let center_movement = current_center - start_center;
        let center_distance = center_movement.magnitude();
        
        // Swipe detection
        if center_distance > self.config.swipe_threshold {
            let direction = self.classify_swipe_direction(center_movement);
            return Some(Gesture::Swipe {
                direction,
                fingers: 2,
            });
        }
        
        // Pinch/zoom detection
        let start_distance = (touch1.start_position - touch2.start_position).magnitude();
        let current_distance = (touch1.current_position - touch2.current_position).magnitude();
        let scale_change = current_distance / start_distance;
        
        if (scale_change - 1.0).abs() > self.config.pinch_threshold {
            return Some(Gesture::Pinch {
                scale: scale_change,
            });
        }
        
        None
    }
    
    fn classify_swipe_direction(&self, movement: Vector2<f32>) -> SwipeDirection {
        if movement.x.abs() > movement.y.abs() {
            if movement.x > 0.0 {
                SwipeDirection::Right
            } else {
                SwipeDirection::Left
            }
        } else {
            if movement.y > 0.0 {
                SwipeDirection::Down
            } else {
                SwipeDirection::Up
            }
        }
    }
    
    fn finalize_gesture(&self) -> Option<Gesture> {
        let gesture_duration = self.gesture_start_time?
            .elapsed();
        
        // Check for tap gesture
        if gesture_duration < Duration::from_millis(self.config.tap_timeout_ms) {
            let finger_count = self.active_touches.len() as u8;
            
            // Calculate average position for tap
            if !self.active_touches.is_empty() {
                let avg_position = self.active_touches.values()
                    .map(|t| t.start_position)
                    .fold(Point2::origin(), |acc, pos| acc + pos.coords)
                    / self.active_touches.len() as f32;
                
                return Some(Gesture::Tap {
                    fingers: finger_count,
                    position: avg_position,
                });
            }
        }
        
        None
    }
}
