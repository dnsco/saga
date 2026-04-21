//! Native Rust port of `src/core/JSSimulationBackend.js`.
//!
//! A `World` holds all mutable simulation state. `run_to_completion` drives a
//! fixed-timestep loop — no wall-clock coupling — calling the user's reducer
//! once per iteration, then advancing physics by `DT` simulated seconds.

pub mod passenger;
pub mod physics;
pub mod stats;

pub use passenger::{PassengerState, SimPassenger};
pub use physics::{SimElevator, DEFAULT_ELEVATOR_SPEED};
pub use stats::Stats;

use crate::challenges::{Challenge, EndCondition};
use crate::game::{Elevator as ApiElevator, Floor as ApiFloor};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Default physics step in simulated seconds (matches browser FRAME_RATE).
pub const DEFAULT_DT: f32 = 1.0 / 60.0;

/// Matches the JS constant of the same name.
const IMMEDIATE_SPAWN_MULTIPLIER: f32 = 1.001;
const MIN_PASSENGER_WEIGHT: i32 = 55;
const MAX_PASSENGER_WEIGHT: i32 = 100;
const NON_GROUND_DESTINATION_ODDS: i32 = 10;

pub struct RunConfig {
    pub dt: f32,
    pub user_step: u32,
    pub seed: u64,
    pub max_seconds: f32,
}

impl Default for RunConfig {
    fn default() -> Self {
        RunConfig {
            dt: DEFAULT_DT,
            user_step: 1,
            seed: 0,
            max_seconds: 600.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Won,
    Lost,
    MaxTimeReached,
}

pub struct RunResult {
    pub outcome: Outcome,
    pub stats: Stats,
}

pub struct World {
    pub floor_count: usize,
    pub elevators: Vec<SimElevator>,
    pub passengers: Vec<SimPassenger>,
    pub floor_buttons: Vec<FloorButtons>,
    pub stats: Stats,
    elapsed_since_spawn: f32,
    spawn_rate: f32,
    end_condition: EndCondition,
    next_passenger_id: u64,
    rng: ChaCha8Rng,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FloorButtons {
    pub up: bool,
    pub down: bool,
}

impl World {
    pub fn new(challenge: &Challenge, seed: u64) -> Self {
        let floor_count = challenge.floor_count;
        let elevators = (0..challenge.elevator_count)
            .map(|i| {
                let cap = challenge.elevator_capacities
                    [i % challenge.elevator_capacities.len()];
                SimElevator::new(i, DEFAULT_ELEVATOR_SPEED, floor_count as i32, cap)
            })
            .collect();

        World {
            floor_count,
            elevators,
            passengers: Vec::new(),
            floor_buttons: vec![FloorButtons::default(); floor_count],
            stats: Stats::default(),
            elapsed_since_spawn: IMMEDIATE_SPAWN_MULTIPLIER / challenge.spawn_rate,
            spawn_rate: challenge.spawn_rate,
            end_condition: challenge.end_condition,
            next_passenger_id: 0,
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Snapshots for user code. Indexed identically to `self.elevators` /
    /// `self.floor_buttons` so commands map back 1:1.
    pub fn snapshots(&self) -> (Vec<ApiElevator>, Vec<ApiFloor>) {
        let elevators = self
            .elevators
            .iter()
            .map(|e| {
                let pressed: Vec<i32> = e
                    .buttons
                    .iter()
                    .enumerate()
                    .filter_map(|(i, pressed)| if *pressed { Some(i as i32) } else { None })
                    .collect();
                ApiElevator::new(
                    e.index as u32,
                    e.current_floor(),
                    e.destination_floor(),
                    e.capacity_percent_full(&self.passengers),
                    pressed,
                )
            })
            .collect();
        let floors = self
            .floor_buttons
            .iter()
            .enumerate()
            .map(|(i, fb)| ApiFloor::new(i as i32, fb.up, fb.down))
            .collect();
        (elevators, floors)
    }

    /// Drains `take_commands` from each snapshot elevator and applies them.
    pub fn apply_commands(&mut self, mut snapshots: Vec<ApiElevator>) {
        for snap in snapshots.iter_mut() {
            let id = snap.id() as usize;
            let commands = snap.take_commands();
            if let Some(e) = self.elevators.get_mut(id) {
                for (_id, floor) in commands {
                    e.go_to_floor(floor);
                }
            }
        }
    }

    /// One physics step of duration `dt` simulated seconds. Mirrors
    /// `JSSimulationBackend.tick(dt)`: spawn → advance elevators → handle
    /// arrivals → drop exited passengers → recalculate stats.
    pub fn step(&mut self, dt: f32) {
        self.stats.elapsed_time += dt;
        self.elapsed_since_spawn += dt;

        while self.elapsed_since_spawn > 1.0 / self.spawn_rate {
            self.elapsed_since_spawn -= 1.0 / self.spawn_rate;
            self.spawn_passenger();
        }

        for i in 0..self.elevators.len() {
            let doors_open = self.elevators[i].tick(dt);
            if doors_open {
                self.handle_arrival(i);
            }
        }

        self.passengers
            .retain(|p| p.state != PassengerState::Exited);

        self.recalculate_stats();
    }

    fn recalculate_stats(&mut self) {
        let t = self.stats.elapsed_time;
        self.stats.transported_per_sec = if t > 0.0 {
            self.stats.transported_count as f32 / t
        } else {
            0.0
        };
        self.stats.move_count = self.elevators.iter().map(|e| e.moves).sum();
    }

    fn spawn_passenger(&mut self) {
        let (start, dest) = self.random_start_and_destination();
        let weight = self
            .rng
            .gen_range(MIN_PASSENGER_WEIGHT..=MAX_PASSENGER_WEIGHT);
        let id = self.next_passenger_id;
        self.next_passenger_id += 1;

        self.passengers.push(SimPassenger {
            id,
            weight,
            starting_floor: start,
            destination_floor: dest,
            state: PassengerState::Waiting,
            elevator_index: None,
            slot: None,
            spawn_time: self.stats.elapsed_time,
        });

        let fb = &mut self.floor_buttons[start as usize];
        if dest > start {
            fb.up = true;
        } else if dest < start {
            fb.down = true;
        }
    }

    fn random_start_and_destination(&mut self) -> (i32, i32) {
        let fc = self.floor_count as i32;
        let start = if self.rng.gen_range(0..=1) == 0 {
            0
        } else {
            self.rng.gen_range(0..fc)
        };
        let dest = if start == 0 {
            self.rng.gen_range(1..fc)
        } else if self.rng.gen_range(0..=NON_GROUND_DESTINATION_ODDS) == 0 {
            (start + self.rng.gen_range(1..fc)) % fc
        } else {
            0
        };
        (start, dest)
    }

    fn handle_arrival(&mut self, elevator_idx: usize) {
        let current_floor = self.elevators[elevator_idx].current_floor();

        // Passengers exiting
        let slots_copy: Vec<Option<u64>> = self.elevators[elevator_idx].slots.clone();
        for slot in slots_copy.iter().flatten() {
            let pid = *slot;
            let should_exit = self
                .passengers
                .iter()
                .find(|p| p.id == pid)
                .map(|p| p.should_exit_at(current_floor))
                .unwrap_or(false);
            if should_exit {
                self.elevators[elevator_idx].remove_passenger(pid);
                let elapsed = self.stats.elapsed_time;
                if let Some(p) = self.passengers.iter_mut().find(|p| p.id == pid) {
                    p.state = PassengerState::Exited;
                    p.elevator_index = None;
                    p.slot = None;

                    self.stats.transported_count += 1;
                    let wait = elapsed - p.spawn_time;
                    if wait > self.stats.max_wait_time {
                        self.stats.max_wait_time = wait;
                    }
                    let n = self.stats.transported_count as f32;
                    self.stats.avg_wait_time =
                        (self.stats.avg_wait_time * (n - 1.0) + wait) / n;
                }
            }
        }

        // Passengers boarding
        let going_up =
            self.floor_buttons[current_floor as usize].up
                && self.elevators[elevator_idx].going_up_indicator;
        let going_down =
            self.floor_buttons[current_floor as usize].down
                && self.elevators[elevator_idx].going_down_indicator;

        let waiting_ids: Vec<u64> = self
            .passengers
            .iter()
            .filter(|p| {
                p.state == PassengerState::Waiting && p.starting_floor == current_floor
            })
            .map(|p| p.id)
            .collect();

        for pid in waiting_ids.iter().copied() {
            if self.elevators[elevator_idx].is_full() {
                break;
            }
            let Some(p) = self.passengers.iter().find(|p| p.id == pid) else {
                continue;
            };
            let wants_up = p.destination_floor > current_floor;
            let wants_down = p.destination_floor < current_floor;
            let dest = p.destination_floor;
            if (wants_up && going_up) || (wants_down && going_down) {
                if let Some(slot) =
                    self.elevators[elevator_idx].add_passenger(pid, dest, &mut self.rng)
                {
                    if let Some(p) = self.passengers.iter_mut().find(|p| p.id == pid) {
                        p.state = PassengerState::Riding;
                        p.elevator_index = Some(elevator_idx);
                        p.slot = Some(slot);
                    }
                }
            }
        }

        // Clear floor buttons if no more waiters for that direction
        let remaining_up = self.passengers.iter().any(|p| {
            p.state == PassengerState::Waiting
                && p.starting_floor == current_floor
                && p.destination_floor > current_floor
        });
        let remaining_down = self.passengers.iter().any(|p| {
            p.state == PassengerState::Waiting
                && p.starting_floor == current_floor
                && p.destination_floor < current_floor
        });
        if going_up && !remaining_up {
            self.floor_buttons[current_floor as usize].up = false;
        }
        if going_down && !remaining_down {
            self.floor_buttons[current_floor as usize].down = false;
        }
    }

    /// End-condition check. Returns `Some(bool)` on win/lose, `None` otherwise.
    pub fn check_end(&self) -> Option<bool> {
        self.end_condition.evaluate(&self.stats)
    }
}

impl SimElevator {
    fn capacity_percent_full(&self, passengers: &[SimPassenger]) -> f32 {
        let load: i32 = self
            .slots
            .iter()
            .filter_map(|s| s.and_then(|id| passengers.iter().find(|p| p.id == id)))
            .map(|p| p.weight)
            .sum();
        load as f32 / (self.capacity() * 100) as f32
    }
}

/// Drives the simulation to completion (win/lose or max time) using the
/// supplied user reducer.
pub fn run_to_completion<F>(challenge: &Challenge, config: RunConfig, mut user_tick: F) -> RunResult
where
    F: FnMut(&mut [ApiElevator], &[ApiFloor]),
{
    let mut world = World::new(challenge, config.seed);
    let mut step = 0u32;
    loop {
        if step % config.user_step == 0 {
            let (mut es, fs) = world.snapshots();
            user_tick(&mut es, &fs);
            world.apply_commands(es);
        }

        world.step(config.dt);

        if let Some(won) = world.check_end() {
            return RunResult {
                outcome: if won { Outcome::Won } else { Outcome::Lost },
                stats: world.stats,
            };
        }

        if world.stats.elapsed_time >= config.max_seconds {
            // Run one final end-condition check so `Demo` doesn't mask a
            // trailing tick's stats drift.
            let final_stats = world.stats;
            return RunResult {
                outcome: match world.check_end() {
                    Some(true) => Outcome::Won,
                    Some(false) => Outcome::Lost,
                    None => Outcome::MaxTimeReached,
                },
                stats: final_stats,
            };
        }

        step = step.wrapping_add(1);
    }
}
