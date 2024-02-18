use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;
use std::time::Duration;
use robotics_lib::runner::{Runnable, Runner};
use robotics_lib::utils::LibError;
use robotics_lib::world::world_generator::Generator;
use robot_wrapper::RobotWrapper;
use super::{PartialWorld, RunMode};

pub mod robot_wrapper;

// GameRunner handles creating the Runner and running it at the correct rate based on the RunMode
// last received through the gui->game channel

pub struct GameRunner {
    runner: Runner,
    gui_to_game_rx: Receiver<RunMode>,
}
impl GameRunner {
    pub fn new(robot: Box<dyn Runnable>, world_generator: &mut impl Generator, game_to_worker_tx: SyncSender<PartialWorld>, gui_to_game_rx: Receiver<RunMode>) -> Result<Self, LibError> {
        let robot_wrapper = RobotWrapper::new(robot, game_to_worker_tx);

        let mut runner = Runner::new(Box::new(robot_wrapper), world_generator)?;
        runner.game_tick()?; // first tick needed to fully init partial_world

        Ok(Self{ runner, gui_to_game_rx })
    }

    pub fn run(mut self) {
        let mut last_tick_begin = std::time::Instant::now();
        let mut run_mode = RunMode::Paused;
        'main_game_loop:
        loop {
            loop {
                run_mode = self.gui_to_game_rx.try_iter().last().unwrap_or(run_mode);
                match run_mode {
                    RunMode::SingleTick => {
                        run_mode = RunMode::Paused;
                        break;
                    }
                    RunMode::Continuous(None) => break,
                    RunMode::Terminate => break 'main_game_loop,
                    RunMode::Continuous(Some(cap)) => {
                        let interval = Duration::from_secs_f32(1.0 / cap);
                        let elapsed = last_tick_begin.elapsed();
                        if interval > elapsed {
                            thread::sleep(interval - elapsed);
                        }
                        break;
                    }
                    RunMode::Paused => {
                        thread::sleep(Duration::from_millis(5));
                    }
                }
            }

            last_tick_begin = std::time::Instant::now();
            self.runner.game_tick().unwrap();
        }
    }
}

