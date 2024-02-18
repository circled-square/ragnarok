mod worker_thread;
mod game_runner;
mod gui_thread;

use std::collections::{HashMap, HashSet};
use std::{sync};
use nalgebra_glm::{UVec2};
use robotics_lib::runner::{Runnable};
use robotics_lib::utils::LibError;
use robotics_lib::world::environmental_conditions::EnvironmentalConditions;
use robotics_lib::world::tile::{Content, Tile};
use robotics_lib::world::world_generator::Generator;
use gui_thread::GuiThread;
use worker_thread::WorkerThread;
use game_runner::GameRunner;

// GuiRunner handles spawning and joining the GuiThread and the WorkerThread, and hijacks the main
// thread to use as game thread for GameRunner.

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
    game_runner: GameRunner,
    //other threads
    worker_thread: WorkerThread,
    gui_thread: GuiThread,
}
impl GuiRunner {
    /// Constructs a GuiRunner, given a Runnable and a Generator (similarly to `Runner::new`).
    pub fn new(robot: Box<dyn Runnable>, generator: &mut impl Generator) -> Result<GuiRunner, LibError> {
        // we only allow 1 PartialWorld to be queued between in the game->worker channel to avoid
        // having world information become more and more dated as the execution goes, rather
        // discarding some messages (skipping world versions when the game is going really fast
        // from one to the other)
        let (game_to_worker_tx, game_to_worker_rx) = sync::mpsc::sync_channel::<PartialWorld>(1);
        let (worker_to_gui_tx, worker_to_gui_rx) = sync::mpsc::channel::<PartialWorld>();
        let (gui_to_game_tx, gui_to_game_rx) = sync::mpsc::channel::<RunMode>();

        let game_runner = GameRunner::new(robot, generator, game_to_worker_tx, gui_to_game_rx)?;

        let worker_thread = WorkerThread::new(game_to_worker_rx, worker_to_gui_tx);
        let gui_thread = GuiThread::new(worker_to_gui_rx, gui_to_game_tx);
        Ok(Self { game_runner, worker_thread, gui_thread })
    }

    /// Starts the game loop and the GUI, which will run on different threads. Consumes GuiRunner
    /// and only returns when the user closes the window.
    pub fn run(self) -> Result<(), LibError> {
        let worker_thread_handle = self.worker_thread.start();
        let gui_thread_handle = self.gui_thread.start();

        self.game_runner.run();

        gui_thread_handle.join().expect("failed to join GUI thread");
        worker_thread_handle.join().expect("failed to join worker thread");
        Ok(())
    }
}

// RunMode contains information about how the user wants the simulation to be run.
// it will be sent between different threads: the gui thread will send it to the game thread to
// change run mode
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RunMode {
    SingleTick,
    Continuous(Option<f32>), // if Some it indicates the number of ticks per second the game will be played at
    Paused,
    Terminate,
}

// PartialWorld contains the partial world information available to the robot, including information
// about discovered tiles, the robot itself and the environmental conditions. it also includes the
// tiles_to_refresh field to simplify the job of the gui thread, which can avoid wasting computing
// resources to refresh all other tiles.
// It will be sent through channels between different threads: the game thread will send the raw
// information to the worker thread, which will compute tiles_to_refresh (tiles whose vertices need
// to be created or updated) and send that information, along with what it received from the game
// thread to the gui thread.
#[derive(Clone)]
pub(crate) struct PartialWorld {
    pub world: Vec<Vec<Option<Tile>>>,
    pub tiles_to_refresh: HashSet<UVec2>,
    pub robot_position: UVec2,
    pub energy: usize,
    pub backpack: HashMap<Content, usize>,
    pub env_cond: EnvironmentalConditions,
}
