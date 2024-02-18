use std::time::SystemTime;

// FrameDeltaTimer's get_delta_and_reset, if called every frame, returns the delta from the
// frame (calculating time diff since last invocation). get_average_fps returns the average FPS over
// the last few frames, taking into account frames up to 1 second old.

pub struct FrameDeltaTimer {
    last_frame_time: SystemTime,
    circular_queue_deltas: [f32; 600],
    circular_queue_cursor: usize,
}

impl FrameDeltaTimer {
    pub fn new() -> Self {
        Self {
            last_frame_time: SystemTime::now(),
            circular_queue_deltas: [0.0; 600],
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
        let mut sum = 0.0;
        let mut number_of_deltas_summed = 0;
        let iter_1 = self.circular_queue_deltas[..self.circular_queue_cursor].iter().rev();
        let iter_2 = self.circular_queue_deltas[self.circular_queue_cursor..].iter().rev();

        for delta in iter_1.chain(iter_2).cloned() {
            sum += delta;
            number_of_deltas_summed += 1;
            if sum >= 1.0 {
                break;
            }
        }

        (number_of_deltas_summed as f32) / sum
    }
}