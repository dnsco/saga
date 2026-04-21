//! Unit tests for the user's reducer. These demonstrate how to build
//! `Elevator` and `Floor` fixtures by hand and assert on the commands a
//! single `tick` issues. Add cases as you extend your algorithm.

use saga::game::{Elevator, Floor};
use saga::reducer::{tick, State};

fn floor(level: i32, up: bool, down: bool) -> Floor {
    Floor::new(level, up, down)
}

fn elevator(
    id: u32,
    current: i32,
    dest: Option<i32>,
    pct_full: f32,
    pressed: &[i32],
) -> Elevator {
    Elevator::new(id, current, dest, pct_full, pressed.to_vec())
}

#[test]
fn idle_elevator_moves_to_next_floor() {
    let mut state = State::default();
    let mut elevators = vec![elevator(0, 0, None, 0.0, &[])];
    let floors = vec![floor(0, false, false), floor(1, false, false), floor(2, false, false)];

    tick(&mut state, &mut elevators, &floors);

    let cmds = elevators[0].take_commands();
    assert_eq!(cmds, vec![(0, 2)], "baseline rotator should send elevator 0 to floor 2 first");
}

#[test]
fn moving_elevator_is_not_commanded_again() {
    let mut state = State::default();
    let mut elevators = vec![elevator(0, 1, Some(3), 0.25, &[3])];
    let floors = vec![
        floor(0, false, false),
        floor(1, true, false),
        floor(2, false, false),
        floor(3, false, false),
    ];

    tick(&mut state, &mut elevators, &floors);

    assert!(
        elevators[0].take_commands().is_empty(),
        "reducer should not re-command an elevator that already has a destination"
    );
}

#[test]
fn multiple_idle_elevators_are_dispatched_independently() {
    let mut state = State::default();
    let mut elevators = vec![
        elevator(0, 0, None, 0.0, &[]),
        elevator(1, 0, None, 0.0, &[]),
    ];
    let floors = vec![floor(0, true, false), floor(1, false, false), floor(2, false, false)];

    tick(&mut state, &mut elevators, &floors);

    let cmds0 = elevators[0].take_commands();
    let cmds1 = elevators[1].take_commands();
    assert_eq!(cmds0.len(), 1);
    assert_eq!(cmds1.len(), 1);
}

#[test]
fn empty_building_is_a_noop() {
    let mut state = State::default();
    let mut elevators: Vec<Elevator> = vec![];
    let floors: Vec<Floor> = vec![];

    // Must not panic.
    tick(&mut state, &mut elevators, &floors);
}
