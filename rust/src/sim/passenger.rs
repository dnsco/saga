#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassengerState {
    Waiting,
    Riding,
    Exited,
}

#[derive(Debug)]
pub struct SimPassenger {
    pub id: u64,
    pub weight: i32,
    pub starting_floor: i32,
    pub destination_floor: i32,
    pub state: PassengerState,
    pub elevator_index: Option<usize>,
    pub slot: Option<usize>,
    pub spawn_time: f32,
}

impl SimPassenger {
    pub fn should_exit_at(&self, floor: i32) -> bool {
        floor == self.destination_floor
    }
}
