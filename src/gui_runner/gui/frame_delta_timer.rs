use std::time::SystemTime;

pub struct FrameDeltaTimer {
    last_frame_time: SystemTime,
    circular_queue_deltas: [f32; 30],
    circular_queue_cursor: usize,
}

impl FrameDeltaTimer {
    pub fn new() -> Self {
        Self {
            last_frame_time: SystemTime::now(),
            circular_queue_deltas: [0.0; 30],
            circular_queue_cursor: 0,
        }
    }

    pub fn get_delta_and_reset(&mut self) -> f32 {
        let curr_time = SystemTime::now();
        let delta = curr_time.duration_since(self.last_frame_time).expect("clock error").as_secs_f32();
        self.last_frame_time = curr_time;

        self.circular_queue_deltas[self.circular_queue_cursor] = delta;
        self.circular_queue_cursor = (self.circular_queue_cursor + 1) % self.circular_queue_deltas.len();

        delta
    }

    pub fn get_average_fps(&self) -> f32 {
        1.0 / (self.circular_queue_deltas.iter().sum::<f32>() / self.circular_queue_deltas.len() as f32)
    }
}