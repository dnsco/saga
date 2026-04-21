// Stubbed until task 5 fills in the 16 challenges.
use crate::sim::Stats;

#[derive(Debug, Clone)]
pub struct Challenge {
    pub floor_count: usize,
    pub elevator_count: usize,
    pub spawn_rate: f32,
    pub elevator_capacities: Vec<usize>,
    pub end_condition: EndCondition,
}

#[derive(Debug, Clone, Copy)]
pub enum EndCondition {
    Demo,
    PassengersWithinTime {
        passengers: u32,
        time_limit: f32,
    },
    PassengersWithMaxWait {
        passengers: u32,
        max_wait: f32,
    },
    PassengersWithinTimeMaxWait {
        passengers: u32,
        time_limit: f32,
        max_wait: f32,
    },
    PassengersWithinMoves {
        passengers: u32,
        move_limit: u32,
    },
}

impl EndCondition {
    pub fn evaluate(self, s: &Stats) -> Option<bool> {
        use EndCondition::*;
        match self {
            Demo => None,
            PassengersWithinTime {
                passengers,
                time_limit,
            } => {
                if s.elapsed_time >= time_limit || s.transported_count >= passengers {
                    Some(s.elapsed_time <= time_limit && s.transported_count >= passengers)
                } else {
                    None
                }
            }
            PassengersWithMaxWait {
                passengers,
                max_wait,
            } => {
                if s.max_wait_time >= max_wait || s.transported_count >= passengers {
                    Some(s.max_wait_time <= max_wait && s.transported_count >= passengers)
                } else {
                    None
                }
            }
            PassengersWithinTimeMaxWait {
                passengers,
                time_limit,
                max_wait,
            } => {
                if s.elapsed_time >= time_limit
                    || s.max_wait_time >= max_wait
                    || s.transported_count >= passengers
                {
                    Some(
                        s.elapsed_time <= time_limit
                            && s.max_wait_time <= max_wait
                            && s.transported_count >= passengers,
                    )
                } else {
                    None
                }
            }
            PassengersWithinMoves {
                passengers,
                move_limit,
            } => {
                if s.move_count >= move_limit || s.transported_count >= passengers {
                    Some(s.move_count <= move_limit && s.transported_count >= passengers)
                } else {
                    None
                }
            }
        }
    }
}

pub fn all() -> Vec<Challenge> {
    use EndCondition::*;
    vec![
        Challenge {
            floor_count: 3,
            elevator_count: 1,
            spawn_rate: 0.3,
            elevator_capacities: vec![4],
            end_condition: PassengersWithinTime {
                passengers: 15,
                time_limit: 60.0,
            },
        },
        Challenge {
            floor_count: 5,
            elevator_count: 1,
            spawn_rate: 0.5,
            elevator_capacities: vec![6],
            end_condition: PassengersWithinTime {
                passengers: 23,
                time_limit: 60.0,
            },
        },
        Challenge {
            floor_count: 8,
            elevator_count: 2,
            spawn_rate: 0.6,
            elevator_capacities: vec![4],
            end_condition: PassengersWithinTime {
                passengers: 26,
                time_limit: 60.0,
            },
        },
        Challenge {
            floor_count: 6,
            elevator_count: 4,
            spawn_rate: 1.7,
            elevator_capacities: vec![4],
            end_condition: PassengersWithinTime {
                passengers: 100,
                time_limit: 68.0,
            },
        },
        Challenge {
            floor_count: 6,
            elevator_count: 2,
            spawn_rate: 0.4,
            elevator_capacities: vec![5],
            end_condition: PassengersWithMaxWait {
                passengers: 50,
                max_wait: 21.0,
            },
        },
        Challenge {
            floor_count: 7,
            elevator_count: 3,
            spawn_rate: 0.6,
            elevator_capacities: vec![4],
            end_condition: PassengersWithMaxWait {
                passengers: 50,
                max_wait: 20.0,
            },
        },
        Challenge {
            floor_count: 13,
            elevator_count: 2,
            spawn_rate: 1.1,
            elevator_capacities: vec![8, 10],
            end_condition: PassengersWithinTime {
                passengers: 50,
                time_limit: 70.0,
            },
        },
        Challenge {
            floor_count: 9,
            elevator_count: 5,
            spawn_rate: 1.1,
            elevator_capacities: vec![4],
            end_condition: PassengersWithMaxWait {
                passengers: 60,
                max_wait: 19.0,
            },
        },
        Challenge {
            floor_count: 9,
            elevator_count: 5,
            spawn_rate: 1.1,
            elevator_capacities: vec![4],
            end_condition: PassengersWithMaxWait {
                passengers: 80,
                max_wait: 17.0,
            },
        },
        Challenge {
            floor_count: 9,
            elevator_count: 6,
            spawn_rate: 1.1,
            elevator_capacities: vec![4],
            end_condition: PassengersWithMaxWait {
                passengers: 100,
                max_wait: 16.0,
            },
        },
        Challenge {
            floor_count: 9,
            elevator_count: 6,
            spawn_rate: 1.0,
            elevator_capacities: vec![5],
            end_condition: PassengersWithMaxWait {
                passengers: 110,
                max_wait: 15.0,
            },
        },
        Challenge {
            floor_count: 8,
            elevator_count: 6,
            spawn_rate: 0.9,
            elevator_capacities: vec![4],
            end_condition: PassengersWithMaxWait {
                passengers: 120,
                max_wait: 15.0,
            },
        },
        Challenge {
            floor_count: 12,
            elevator_count: 4,
            spawn_rate: 1.4,
            elevator_capacities: vec![5, 10],
            end_condition: PassengersWithinTime {
                passengers: 70,
                time_limit: 80.0,
            },
        },
        Challenge {
            floor_count: 21,
            elevator_count: 5,
            spawn_rate: 1.9,
            elevator_capacities: vec![10],
            end_condition: PassengersWithinTime {
                passengers: 110,
                time_limit: 80.0,
            },
        },
        Challenge {
            floor_count: 21,
            elevator_count: 8,
            spawn_rate: 1.5,
            elevator_capacities: vec![6, 8],
            end_condition: PassengersWithinTimeMaxWait {
                passengers: 2675,
                time_limit: 1800.0,
                max_wait: 45.0,
            },
        },
        Challenge {
            floor_count: 21,
            elevator_count: 8,
            spawn_rate: 1.5,
            elevator_capacities: vec![6, 8],
            end_condition: Demo,
        },
    ]
}
