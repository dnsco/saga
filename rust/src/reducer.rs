//! The user's elevator algorithm.
//!
//! `tick(&mut state, elevators, floors)` is a free-function reducer: mutate
//! state and issue `go_to_floor` commands on the elevators. The same file is
//! pasted into the browser editor, wrapped with:
//!
//! ```ignore
//! fn main() {
//!     let mut state = State::default();
//!     game::run(|es, fs| tick(&mut state, es, fs));
//! }
//! ```

use crate::game::{Elevator, Floor};

#[derive(Debug)]
pub enum State {
    /// Naive baseline: rotate the first elevator through every floor.
    /// Replace with a smarter variant as your algorithm grows.
    Rotating { next_floor: i32 },
}

impl Default for State {
    fn default() -> Self {
        State::Rotating { next_floor: 1 }
    }
}

pub fn tick(state: &mut State, elevators: &mut [Elevator], floors: &[Floor]) {
    match state {
        State::Rotating { next_floor } => {
            let floor_count = floors.len() as i32;
            if floor_count == 0 {
                return;
            }
            for e in elevators.iter_mut() {
                if e.destination_floor().is_none() {
                    if *next_floor >= floor_count {
                        *next_floor = 0;
                    } else {
                        *next_floor += 1;
                    }
                    e.go_to_floor(*next_floor);
                }
            }
        }
    }
}
