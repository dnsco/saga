# CLI Rust algorithm development for Saga

## Context

Saga's Rust runtime currently runs user code exclusively in the browser via Miri compiled to WebAssembly (`public/rust/miri.wasm` + stage1 rlibs in `public/rust/lib/`). Each edit requires a page reload → rlib fetch → Miri recompile → SharedArrayBuffer tick loop. There is **no native Cargo project** in the repo — `public/rust/game.rs` is a standalone file that the browser inlines as `mod game { ... }` before the user's source and runs under Miri.

The user wants to iterate on a better elevator algorithm with `cargo test`, and also run full challenge simulations headlessly to measure real stats (avg wait, transported count, pass/fail). The new workflow must not break the browser path.

## Approach

Add a native Cargo crate at `rust/` that (a) re-uses `public/rust/game.rs` unchanged-in-spirit so the browser and CLI share one library, (b) extracts the algorithm into a testable free-function reducer (`tick(&mut State, ...)`), and (c) provides a Rust port of the simulation (physics, challenges, passenger spawn) with a seeded RNG for deterministic runs.

### Layout (all new unless noted)

```
rust/
  Cargo.toml                   # edition 2021, deps: rand, clap (optional)
  .gitignore                   # target/
  src/
    lib.rs                     # pub mod game; pub mod reducer; pub mod sim; pub mod challenges;
    game.rs                    # #[path = "../../public/rust/game.rs"] pub mod game_impl; pub use game_impl::*;
    reducer.rs              # User's algorithm — free `fn tick` + State type
    challenges.rs              # Port of src/game/challenges.js (16 challenges + end conditions)
    sim/
      mod.rs                   # World::tick() orchestrator
      physics.rs               # Elevator motion — mirrors src/core/Elevator.js constants exactly
      passenger.rs             # Passenger spawn + wait/transport tracking
      stats.rs                 # avgWaitTime, maxWaitTime, transportedCount, moveCount
    bin/
      sim.rs                   # CLI: `cargo run --bin sim -- --challenge N --seed S`
  tests/
    reducer.rs              # Unit tests: call tick with hand-built fixtures
    simulation.rs              # Integration: run challenge with fixed seed, assert pass
```

### Algorithm shape: free `fn tick` with `&mut State`

`rust/src/reducer.rs` exposes a `State` type (starts as an enum; leaves room to move to typestate later) and a free `tick` that mutates state in place:

```rust
use crate::game::{Elevator, Floor};

#[derive(Debug, Default)]
pub enum State {
    #[default]
    Idle,
    // enum variants carry whatever per-run data the algorithm needs
}

pub fn tick(state: &mut State, elevators: &mut [Elevator], floors: &[Floor]) {
    // algorithm mutates state, elevators
}
```

- Free function, not a method on a struct — keeps the "reducer over state" model without locking in a framework shape. If the user later moves to typestate, the enum becomes a wrapper type around type-parameterized states and `tick` dispatches via `match`.
- **CLI unit tests** call `tick` directly with hand-built `Elevator` / `Floor` fixtures and assert on both the returned `State` (via the `&mut`) and the commands captured from the elevators (`Elevator::take_commands`).
- **CLI simulator** plumbs it in one line: `world.run(|es, fs| tick(&mut state, es, fs));`
- **Browser** keeps working: paste `reducer.rs` contents into the editor and wrap with:
  ```rust
  fn main() {
      let mut state = State::default();
      game::run(|es, fs| tick(&mut state, es, fs));
  }
  ```

### Minimal, browser-safe edits to `public/rust/game.rs`

The current `Elevator` and `Floor` have private fields and no constructors. To test them natively we need public builders. Edits:

1. Add `impl Elevator { pub fn new(id: u32, current_floor: i32, destination_floor: Option<i32>, percent_full: f32, pressed_buttons: Vec<i32>) -> Self { ... } }`
2. Add `impl Floor { pub fn new(level: i32, up: bool, down: bool) -> Self { ... } }`
3. Add `impl Elevator { pub fn take_commands(&mut self) -> Vec<(u32, i32)> { std::mem::take(&mut self.commands) } }` — the native simulator drains commands after each tick.

These are additive. Miri compiles them fine. The browser path (inlined as `mod game`) is unaffected because nothing *calls* the new APIs from the user template.

### Native simulator parity with `src/core/`

Port exactly, not re-invent:

- **Physics constants** (`src/core/Elevator.js:5-21`): `ACCELERATION = 1.1`, `DECELERATION = 1.6`, `DOOR_PAUSE_TIME = 1.2`, `ARRIVAL_THRESHOLD = 0.01`, `ACCELERATION_DISTANCE_FACTOR = 5`, `STOPPING_DISTANCE_MARGIN = 1.05`, `DECELERATION_CORRECTION = 1.1`. Default capacity `4`.
- **Tick flow** (`src/core/JSSimulationBackend.js:301+`): spawn → elevator physics → arrival/boarding → build API snapshots → call user `tick` → drain commands via `Elevator::take_commands` → stats → end-condition check.
- **Passenger spawn / weights / start+dest selection** (`src/core/Passenger.js` and JSSimulationBackend spawn logic): port the same biased RNG draws, but backed by `rand::rngs::StdRng::seed_from_u64(seed)` so runs are reproducible.
- **Challenges** (`src/game/challenges.js:158-268`): 16 entries. End-condition variants (`requirePassengerCountWithinTime`, `WithMaxWaitTime`, `WithinMoves`, combined, `requireDemo`) become an enum `EndCondition` with an `evaluate(&Stats) -> Option<bool>` method. Tests lock in expected floor/elevator counts and spawn rates per challenge.

### Decoupling from wall-clock time

The browser path is tied to `requestAnimationFrame` by design, but the native simulator is not — a 60-second challenge should finish in milliseconds. The loop is a pure **fixed-timestep integrator** driven as fast as the CPU allows:

```rust
// simplified sim/mod.rs
const DT: f32 = 1.0 / 60.0;        // simulated seconds per physics step
const USER_STEP: u32 = 1;           // call user tick every N physics steps

let mut frame = 0u32;
loop {
    world.advance_physics(DT);       // elevator motion, doors, boarding
    world.spawn_passengers(DT);      // RNG-driven, no sleep
    if frame % USER_STEP == 0 {
        let (mut es, fs) = world.snapshots();
        tick(&mut state, &mut es, &fs);
        world.apply_commands(es);
    }
    world.update_stats(DT);
    if let Some(result) = world.check_end_condition() { return result; }
    frame = frame.wrapping_add(1);
}
```

No `std::thread::sleep`, no async, no RAF. `DT` is simulated time, and each iteration just advances counters and integrates physics. On a modern CPU a 300-second challenge completes in single-digit milliseconds.

Knobs exposed by the `sim` binary (all optional; defaults match the in-browser feel):
- `--dt` — physics step size in simulated seconds (default `1/60`)
- `--user-step` — call user `tick` every N physics steps (default `1`, matching browser rAF cadence at 1× time-scale)
- `--seed` — RNG seed
- `--max-seconds` — safety cap on simulated time (so a broken algorithm can't loop forever for open-ended `requireDemo` challenges). Default `600`.
- `--verbose` — per-tick trace for debugging

### CLI UX

```
cargo run --bin sim -- --challenge 3 --seed 42
# → Challenge 3 (... floors, ... elevators, ... p/s)
# → PASS simulated=178.4s wall=4ms | transported=120 | avg_wait=9.21s | max_wait=22.3s | moves=418

cargo run --bin sim -- --challenge 3 --seed 42 --verbose
# per-tick log of elevator positions and commands

cargo test                  # all unit + integration tests
cargo test --test reducer  # just unit
```

Logging both simulated and wall-clock elapsed makes it obvious when the simulator itself regresses in speed.

### Files to create / modify

**New:**
- `rust/Cargo.toml`, `rust/.gitignore`, `rust/src/lib.rs`, `rust/src/game.rs`, `rust/src/reducer.rs`, `rust/src/challenges.rs`
- `rust/src/sim/{mod.rs, physics.rs, passenger.rs, stats.rs}`
- `rust/src/bin/sim.rs`
- `rust/tests/reducer.rs`, `rust/tests/simulation.rs`

**Modified:**
- `public/rust/game.rs` — add `pub fn new` on `Elevator` and `Floor`, add `pub fn take_commands`. Keep everything else byte-identical so the browser build stays stable.
- Root `.gitignore` — append `rust/target/`.

### Verification

1. **Unit tests pass**: `cd rust && cargo test --test reducer` — fixtures verify `tick(&mut state, &mut elevators, &floors)` mutates `state` and issues the expected commands (via `Elevator::take_commands`) for: idle elevators with pressed buttons, floors with up-buttons and no free elevator, full elevator with more requests.
2. **Integration test passes**: `cd rust && cargo test --test simulation` — runs challenge 1 with `seed=1` and asserts the starter `tick` finishes within the challenge's time budget. Test should complete in under ~100ms wall-clock despite simulating multiple minutes (sanity check on the clock-decoupling).
3. **CLI run**: `cd rust && cargo run --bin sim -- --challenge 1 --seed 1` → prints PASS/FAIL + both simulated and wall-clock elapsed.
4. **Browser unaffected**: `npm run dev`, switch to Rust runtime, load default template, run challenge 1 — still loads and ticks. Then paste `rust/src/reducer.rs` contents into the editor and wrap with the `fn main() { ... }` shim shown above to confirm the algorithm runs identically.
