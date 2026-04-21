//! End-to-end tests: wire the user's reducer into the simulator and
//! assert the outcome is deterministic + wall-clock-decoupled.

use std::time::Instant;

use saga::challenges;
use saga::reducer::{tick, State};
use saga::sim::{run_to_completion, Outcome, RunConfig};

#[test]
fn deterministic_with_same_seed() {
    let list = challenges::all();
    let c = &list[0];
    let mut sa = State::default();
    let mut sb = State::default();
    let ra = run_to_completion(
        c,
        RunConfig {
            seed: 42,
            ..Default::default()
        },
        |es, fs| tick(&mut sa, es, fs),
    );
    let rb = run_to_completion(
        c,
        RunConfig {
            seed: 42,
            ..Default::default()
        },
        |es, fs| tick(&mut sb, es, fs),
    );
    assert_eq!(ra.stats.transported_count, rb.stats.transported_count);
    assert_eq!(ra.stats.move_count, rb.stats.move_count);
    assert!((ra.stats.avg_wait_time - rb.stats.avg_wait_time).abs() < 1e-4);
    assert!((ra.stats.elapsed_time - rb.stats.elapsed_time).abs() < 1e-4);
}

#[test]
fn different_seeds_yield_different_runs() {
    let list = challenges::all();
    let c = &list[0];
    let mut sa = State::default();
    let mut sb = State::default();
    let ra = run_to_completion(
        c,
        RunConfig {
            seed: 1,
            ..Default::default()
        },
        |es, fs| tick(&mut sa, es, fs),
    );
    let rb = run_to_completion(
        c,
        RunConfig {
            seed: 99,
            ..Default::default()
        },
        |es, fs| tick(&mut sb, es, fs),
    );
    // At least one stat should differ. With different passenger streams it
    // would be very surprising if all three matched.
    let same = ra.stats.transported_count == rb.stats.transported_count
        && ra.stats.move_count == rb.stats.move_count
        && (ra.stats.max_wait_time - rb.stats.max_wait_time).abs() < 1e-4;
    assert!(
        !same,
        "distinct seeds produced identical stats — RNG may not be threaded"
    );
}

#[test]
fn reducer_transports_passengers_on_challenge_1() {
    let list = challenges::all();
    let c = &list[0];
    let mut state = State::default();
    let r = run_to_completion(
        c,
        RunConfig {
            seed: 7,
            ..Default::default()
        },
        |es, fs| tick(&mut state, es, fs),
    );
    assert!(
        r.stats.transported_count > 0,
        "reducer should transport at least one passenger — got {:?}",
        r.stats
    );
    assert!(
        matches!(r.outcome, Outcome::Won),
        "expected Won on challenge 1 — got {:?} ({:?})",
        r.outcome,
        r.stats
    );
}

#[test]
fn simulation_is_much_faster_than_wall_clock() {
    let list = challenges::all();
    let c = &list[0]; // 60-sim-second challenge
    let mut state = State::default();
    let t0 = Instant::now();
    let r = run_to_completion(
        c,
        RunConfig {
            seed: 3,
            ..Default::default()
        },
        |es, fs| tick(&mut state, es, fs),
    );
    let wall_ms = t0.elapsed().as_millis();
    assert!(
        r.stats.elapsed_time >= 30.0,
        "expected significant simulated time, got {}s",
        r.stats.elapsed_time
    );
    assert!(
        wall_ms < 500,
        "60s of sim should finish in <500ms wall; took {}ms (sim runs should not be clock-bound)",
        wall_ms
    );
    // Sanity: matches Outcome::Won / Lost, not MaxTimeReached (we're within the cap).
    assert!(matches!(r.outcome, Outcome::Won | Outcome::Lost));
}
