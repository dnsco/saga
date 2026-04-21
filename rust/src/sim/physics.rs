//! Elevator motion, ported 1:1 from src/core/Elevator.js so a seeded run
//! here tracks the browser simulation up to f32/f64 rounding.

use rand::Rng;
use rand_chacha::ChaCha8Rng;

pub const ACCELERATION: f32 = 1.1;
pub const DECELERATION: f32 = 1.6;
pub const DOOR_PAUSE_TIME: f32 = 1.2;
pub const ARRIVAL_THRESHOLD: f32 = 0.01;
pub const ACCELERATION_DISTANCE_FACTOR: f32 = 5.0;
pub const STOPPING_DISTANCE_MARGIN: f32 = 1.05;
pub const DECELERATION_CORRECTION: f32 = 1.1;

pub const DEFAULT_ELEVATOR_SPEED: f32 = 2.6;

/// Internal elevator state. Distinct from `crate::game::Elevator`, which is
/// the snapshot/API type the user code sees.
#[derive(Debug)]
pub struct SimElevator {
    pub index: usize,
    pub max_speed: f32,
    pub max_floor: i32,
    pub destination: i32,
    pub velocity: f32,
    pub position: f32,
    pub moves: u32,
    pub buttons: Vec<bool>,
    /// Passenger slots. Each slot is `None` or the passenger's id.
    pub slots: Vec<Option<u64>>,
    pub going_up_indicator: bool,
    pub going_down_indicator: bool,
    pub pause: f32,
}

impl SimElevator {
    pub fn new(index: usize, max_speed: f32, floor_count: i32, capacity: usize) -> Self {
        SimElevator {
            index,
            max_speed,
            max_floor: floor_count,
            destination: 0,
            velocity: 0.0,
            position: 0.0,
            moves: 0,
            buttons: vec![false; floor_count as usize],
            slots: vec![None; capacity],
            going_up_indicator: true,
            going_down_indicator: true,
            pause: DOOR_PAUSE_TIME,
        }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn current_floor(&self) -> i32 {
        self.position.floor() as i32
    }

    pub fn distance_to_destination(&self) -> f32 {
        (self.destination as f32 - self.position).abs()
    }

    pub fn direction(&self) -> i32 {
        let d = self.destination as f32 - self.position;
        if d > 0.0 {
            1
        } else if d < 0.0 {
            -1
        } else {
            0
        }
    }

    pub fn is_moving(&self) -> bool {
        self.direction() != 0
    }

    pub fn destination_floor(&self) -> Option<i32> {
        if self.is_moving() {
            Some(self.destination)
        } else {
            None
        }
    }

    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|s| s.is_some())
    }

    pub fn go_to_floor(&mut self, floor: i32) {
        let clamped = floor.clamp(0, self.max_floor - 1);
        if self.destination != clamped {
            self.destination = clamped;
            self.moves += 1;
        }
    }

    /// Returns true if the elevator has arrived (doors open) or is paused.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.pause = (self.pause - dt).max(0.0);
        if !self.is_moving() || self.pause > 0.0 {
            return true;
        }

        self.position += self.velocity * dt;

        if self.distance_to_destination() < ARRIVAL_THRESHOLD {
            self.position = self.destination as f32;
            self.velocity = 0.0;
            let floor = self.current_floor() as usize;
            if floor < self.buttons.len() {
                self.buttons[floor] = false;
            }
            self.pause = DOOR_PAUSE_TIME;
            return true;
        }

        let new_v = self
            .calculate_velocity(dt)
            .clamp(-self.max_speed, self.max_speed);
        self.velocity = new_v;
        false
    }

    fn calculate_velocity(&self, dt: f32) -> f32 {
        let target_dir = self.direction() as f32;
        let current_dir = self.velocity.signum_or_zero();
        let distance = self.distance_to_destination();

        if self.velocity == 0.0 {
            let acceleration = (distance * ACCELERATION_DISTANCE_FACTOR).min(ACCELERATION);
            return target_dir * acceleration * dt;
        }

        if target_dir != current_dir {
            let new_v = self.velocity - current_dir * DECELERATION * dt;
            if new_v.signum_or_zero() != current_dir {
                return 0.0;
            }
            return new_v;
        }

        let stopping_distance = (self.velocity * self.velocity) / (2.0 * DECELERATION);
        if stopping_distance * STOPPING_DISTANCE_MARGIN < distance {
            let acceleration = (distance * ACCELERATION_DISTANCE_FACTOR).min(ACCELERATION);
            self.velocity + target_dir * acceleration * dt
        } else {
            let required_decel = (self.velocity * self.velocity) / (2.0 * distance);
            let deceleration = (DECELERATION * DECELERATION_CORRECTION).min(required_decel);
            self.velocity - target_dir * deceleration * dt
        }
    }

    /// Places the passenger in a random free slot, returning the slot index,
    /// or `None` if the elevator is full.
    pub fn add_passenger(
        &mut self,
        passenger_id: u64,
        destination_floor: i32,
        rng: &mut ChaCha8Rng,
    ) -> Option<usize> {
        let free: Vec<usize> = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| if s.is_none() { Some(i) } else { None })
            .collect();
        if free.is_empty() {
            return None;
        }
        let pick = rng.gen_range(0..free.len());
        let slot = free[pick];
        self.slots[slot] = Some(passenger_id);
        if (0..self.max_floor).contains(&destination_floor) {
            self.buttons[destination_floor as usize] = true;
        }
        Some(slot)
    }

    pub fn remove_passenger(&mut self, passenger_id: u64) -> bool {
        for s in self.slots.iter_mut() {
            if *s == Some(passenger_id) {
                *s = None;
                return true;
            }
        }
        false
    }
}

trait SignExt {
    fn signum_or_zero(self) -> Self;
}

impl SignExt for f32 {
    fn signum_or_zero(self) -> Self {
        if self > 0.0 {
            1.0
        } else if self < 0.0 {
            -1.0
        } else {
            0.0
        }
    }
}
