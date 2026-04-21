#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub transported_count: u32,
    pub transported_per_sec: f32,
    pub avg_wait_time: f32,
    pub max_wait_time: f32,
    pub move_count: u32,
    pub elapsed_time: f32,
}
