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

/// Usage:
/// ```
///# use rand::random;
///#
///# use robotics_lib::runner::{Runnable};
///# use robotics_lib::energy::Energy;
///# use robotics_lib::event::events::Event;
///# use robotics_lib::interface::Direction;
///# use robotics_lib::runner::backpack::BackPack;
///# use robotics_lib::world::coordinates::Coordinate;
///# use robotics_lib::world::World;
///#
///# pub struct MyRobot {
///#     pub robot: robotics_lib::runner::Robot,
///# }
///# impl MyRobot {
///#     pub fn new() -> Self { Self { robot: robotics_lib::runner::Robot::new() } }
///# }
///# impl Runnable for MyRobot {
///#     fn process_tick(&mut self, world: &mut World) {}
///#     fn handle_event(&mut self, _event: Event) {}
///#     fn get_energy(&self) -> &Energy { &self.robot.energy }
///#     fn get_energy_mut(&mut self) -> &mut Energy { &mut self.robot.energy }
///#     fn get_coordinate(&self) -> &Coordinate { &self.robot.coordinate }
///#     fn get_coordinate_mut(&mut self) -> &mut Coordinate { &mut self.robot.coordinate }
///#     fn get_backpack(&self) -> &BackPack { &self.robot.backpack }
///#     fn get_backpack_mut(&mut self) -> &mut BackPack { &mut self.robot.backpack }
///# }
///fn main() {
///     //MyRobot must implement Runnable
///    let robot = MyRobot::new();
///    let mut world_generator = rip_worldgenerator::MyWorldGen::new();
///
///    // GuiRunner is constructed similarly to Runner
///    let gui_runner = ragnarok::GuiRunner::new(Box::new(robot), &mut world_generator).unwrap();
///    // GuiRunner::run runs the game
///    gui_runner.run().unwrap();
///}
/// ```
pub struct GuiRunner {
    //necessary for running game loop:
    runner: Runner,
    gui_to_game_rx: Receiver<RunMode>,

    //other threads
    worker_thread: WorkerThread,
    gui_thread: GUIThread,
}
impl GuiRunner {
    /// Constructs a GuiRunner, given a Runnable and a Generator (similarly to `Runner::new`).
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

    /// Starts the game loop and the GUI, which will run on different threads. Consumes GuiRunner
    /// and only returns when the user closes the window.
    pub fn run(self) -> Result<(), LibError> {
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
                run_mode = gui_to_game_rx.try_iter().last().unwrap_or(run_mode);
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
