mod robot_wrapper;
mod gui;
mod worker_thread;

use std::collections::{HashMap, HashSet};
use std::{sync, thread};
use std::sync::mpsc::{Receiver};
use std::time::Duration;
use nalgebra_glm::{UVec2};
use robotics_lib::runner::{Runnable, Runner};
use robotics_lib::utils::LibError;
use robotics_lib::world::environmental_conditions::EnvironmentalConditions;
use robotics_lib::world::tile::{Content, Tile};
use robotics_lib::world::world_generator::Generator;
use robot_wrapper::RobotWrapper;
use crate::gui_runner::gui::{GUIThread};
use worker_thread::WorkerThread;


#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RunMode {
    SingleTick,
    Continuous(Option<f32>), // if Some it indicates the number of ticks per second the game will be played at
    Paused,
    Terminate,
}

#[derive(Clone)]
pub(crate) struct PartialWorld {
    pub world: Vec<Vec<Option<Tile>>>,
    pub tiles_to_refresh: HashSet<UVec2>,
    pub robot_position: UVec2,
    pub energy: usize,
    pub backpack: HashMap<Content, usize>,
    pub env_cond: EnvironmentalConditions,
}
pub struct GuiRunner {
    //necessary for running game loop:
    runner: Runner,
    gui_to_game_rx: Receiver<RunMode>,

    //other threads
    worker_thread: WorkerThread,
    gui_thread: GUIThread,
}
impl GuiRunner {
    pub fn new(robot: Box<dyn Runnable>, generator: &mut impl Generator) -> Result<GuiRunner, LibError> {
        let (game_to_worker_tx, game_to_worker_rx) = sync::mpsc::sync_channel::<PartialWorld>(1);
        let (worker_to_gui_tx, worker_to_gui_rx) = sync::mpsc::channel::<PartialWorld>();
        let (gui_to_game_tx, gui_to_game_rx) = sync::mpsc::channel::<RunMode>();

        let robot_wrapper = RobotWrapper::new(robot, game_to_worker_tx);

        let mut runner = Runner::new(Box::new(robot_wrapper), generator)?;
        runner.game_tick()?; // first tick needed to fully init partial_world

        let worker_thread = WorkerThread::new(game_to_worker_rx, worker_to_gui_tx);
        let gui_thread = GUIThread::new(worker_to_gui_rx, gui_to_game_tx);
        Ok(Self { runner, worker_thread, gui_thread, gui_to_game_rx })
    }

    pub fn run(mut self) -> Result<(), LibError> {
        let worker_thread_handle = self.worker_thread.start();
        let gui_thread_handle = self.gui_thread.start();

        Self::run_game_loop(self.runner, self.gui_to_game_rx);

        gui_thread_handle.join().expect("failed to join GUI thread");
        worker_thread_handle.join().expect("failed to join worker thread");
        Ok(())
    }

    fn run_game_loop(runner: Runner, gui_to_game_rx: Receiver<RunMode>) {
        let mut runner = runner;
        let mut last_tick_begin = std::time::Instant::now();
        let mut run_mode = RunMode::Paused;
        'main_game_loop:
        loop {
            loop {
                run_mode = gui_to_game_rx.try_recv().unwrap_or(run_mode);
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
            runner.game_tick().unwrap();
        }
    }
}
