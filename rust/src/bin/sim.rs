//! CLI harness that runs one challenge with a seeded RNG and prints stats.
//!
//! Usage:
//!   cargo run --bin sim -- [options]
//!
//! Options (all have sensible defaults):
//!   --challenge <1-16>      Challenge number (default 1)
//!   --seed <u64>            RNG seed (default 0)
//!   --dt <f32>              Physics step in simulated seconds (default 1/60)
//!   --user-step <u32>       Call user tick every N physics steps (default 1)
//!   --max-seconds <f32>     Cap simulated time (default: challenge time
//!                           limit or 600s for demo)
//!   --verbose               Dump per-simulated-second trace
//!   --list                  List challenges and exit
//!   -h, --help              Print help

use std::env;
use std::time::Instant;

use saga::challenges::{self, Challenge, EndCondition};
use saga::reducer::{tick, State};
use saga::sim::{Outcome, World, DEFAULT_DT};

fn print_help() {
    println!(
        "Saga native simulator

Usage: sim [options]

Options:
  --challenge <1-16>     Challenge number (default 1)
  --seed <u64>           RNG seed (default 0)
  --dt <f32>             Physics step, simulated seconds (default 1/60)
  --user-step <u32>      Call user tick every N physics steps (default 1)
  --max-seconds <f32>    Cap simulated time
  --verbose              Per-simulated-second trace
  --list                 List challenges and exit
  -h, --help             Print help"
    );
}

fn list_challenges(list: &[Challenge]) {
    for (i, c) in list.iter().enumerate() {
        println!(
            "{:>2}. floors={:>2} elevators={} spawn={:.1}/s cap={:?} end={}",
            i + 1,
            c.floor_count,
            c.elevator_count,
            c.spawn_rate,
            c.elevator_capacities,
            describe_end(c.end_condition),
        );
    }
}

fn describe_end(c: EndCondition) -> String {
    match c {
        EndCondition::Demo => "demo (never ends)".to_string(),
        EndCondition::PassengersWithinTime {
            passengers,
            time_limit,
        } => format!("transport {passengers} in <={time_limit:.0}s"),
        EndCondition::PassengersWithMaxWait {
            passengers,
            max_wait,
        } => format!("transport {passengers} with max wait <={max_wait:.1}s"),
        EndCondition::PassengersWithinTimeMaxWait {
            passengers,
            time_limit,
            max_wait,
        } => format!(
            "transport {passengers} in <={time_limit:.0}s with max wait <={max_wait:.1}s"
        ),
        EndCondition::PassengersWithinMoves {
            passengers,
            move_limit,
        } => format!("transport {passengers} in <={move_limit} moves"),
    }
}

fn default_max_seconds(c: EndCondition) -> f32 {
    match c {
        EndCondition::PassengersWithinTime { time_limit, .. }
        | EndCondition::PassengersWithinTimeMaxWait { time_limit, .. } => time_limit + 1.0,
        _ => 600.0,
    }
}

struct Args {
    challenge: usize,
    seed: u64,
    dt: f32,
    user_step: u32,
    max_seconds: Option<f32>,
    verbose: bool,
    list: bool,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            challenge: 1,
            seed: 0,
            dt: DEFAULT_DT,
            user_step: 1,
            max_seconds: None,
            verbose: false,
            list: false,
        }
    }
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut a = Args::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "--list" => a.list = true,
            "--verbose" => a.verbose = true,
            "--challenge" => {
                a.challenge = args
                    .next()
                    .ok_or("missing value for --challenge")?
                    .parse()
                    .map_err(|_| "invalid --challenge")?;
            }
            "--seed" => {
                a.seed = args
                    .next()
                    .ok_or("missing value for --seed")?
                    .parse()
                    .map_err(|_| "invalid --seed")?;
            }
            "--dt" => {
                a.dt = args
                    .next()
                    .ok_or("missing value for --dt")?
                    .parse()
                    .map_err(|_| "invalid --dt")?;
            }
            "--user-step" => {
                a.user_step = args
                    .next()
                    .ok_or("missing value for --user-step")?
                    .parse()
                    .map_err(|_| "invalid --user-step")?;
                if a.user_step == 0 {
                    return Err("--user-step must be >= 1".into());
                }
            }
            "--max-seconds" => {
                a.max_seconds = Some(
                    args.next()
                        .ok_or("missing value for --max-seconds")?
                        .parse()
                        .map_err(|_| "invalid --max-seconds")?,
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(a)
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            print_help();
            std::process::exit(2);
        }
    };

    let list = challenges::all();

    if args.list {
        list_challenges(&list);
        return;
    }

    if args.challenge == 0 || args.challenge > list.len() {
        eprintln!("error: --challenge must be between 1 and {}", list.len());
        std::process::exit(2);
    }

    let challenge = &list[args.challenge - 1];
    let max_seconds = args
        .max_seconds
        .unwrap_or_else(|| default_max_seconds(challenge.end_condition));

    println!(
        "challenge {} — floors={} elevators={} spawn={:.1}/s | {}",
        args.challenge,
        challenge.floor_count,
        challenge.elevator_count,
        challenge.spawn_rate,
        describe_end(challenge.end_condition),
    );

    let wall_start = Instant::now();
    let mut world = World::new(challenge, args.seed);
    let mut state = State::default();
    let mut step = 0u32;
    let mut last_logged_sec: i32 = -1;

    let outcome = loop {
        if step % args.user_step == 0 {
            let (mut es, fs) = world.snapshots();
            tick(&mut state, &mut es, &fs);
            if args.verbose {
                let sec = world.stats.elapsed_time.floor() as i32;
                if sec > last_logged_sec {
                    last_logged_sec = sec;
                    print!("t={:>4}s ", sec);
                    for e in es.iter() {
                        print!(
                            "[e{} {}→{:?} {:.0}%] ",
                            e.id(),
                            e.current_floor(),
                            e.destination_floor(),
                            e.percent_full() * 100.0
                        );
                    }
                    println!();
                }
            }
            world.apply_commands(es);
        }

        world.step(args.dt);

        if let Some(won) = world.check_end() {
            break if won { Outcome::Won } else { Outcome::Lost };
        }
        if world.stats.elapsed_time >= max_seconds {
            break match world.check_end() {
                Some(true) => Outcome::Won,
                Some(false) => Outcome::Lost,
                None => Outcome::MaxTimeReached,
            };
        }
        step = step.wrapping_add(1);
    };

    let wall_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
    let label = match outcome {
        Outcome::Won => "PASS",
        Outcome::Lost => "FAIL",
        Outcome::MaxTimeReached => "MAX-TIME",
    };
    let s = world.stats;
    println!(
        "{label} simulated={:.2}s wall={:.1}ms | transported={} avg_wait={:.2}s max_wait={:.2}s moves={}",
        s.elapsed_time, wall_ms, s.transported_count, s.avg_wait_time, s.max_wait_time, s.move_count,
    );

    if matches!(outcome, Outcome::Lost | Outcome::MaxTimeReached) {
        std::process::exit(1);
    }
}
