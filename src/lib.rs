//! # Ragnarok
//!
//! `ragnarok` is a multi-threaded 3D world visualizer with a feature loaded GUI and fast performance even with big worlds.
//!
//! Usage:
//! ```
//!# use rand::random;
//!#
//!# use robotics_lib::runner::{Runnable};
//!# use robotics_lib::energy::Energy;
//!# use robotics_lib::event::events::Event;
//!# use robotics_lib::interface::Direction;
//!# use robotics_lib::runner::backpack::BackPack;
//!# use robotics_lib::world::coordinates::Coordinate;
//!# use robotics_lib::world::World;
//!#
//!# pub struct MyRobot {
//!#     pub robot: robotics_lib::runner::Robot,
//!# }
//!# impl MyRobot {
//!#     pub fn new() -> Self { Self { robot: robotics_lib::runner::Robot::new() } }
//!# }
//!# impl Runnable for MyRobot {
//!#     fn process_tick(&mut self, world: &mut World) {}
//!#     fn handle_event(&mut self, _event: Event) {}
//!#     fn get_energy(&self) -> &Energy { &self.robot.energy }
//!#     fn get_energy_mut(&mut self) -> &mut Energy { &mut self.robot.energy }
//!#     fn get_coordinate(&self) -> &Coordinate { &self.robot.coordinate }
//!#     fn get_coordinate_mut(&mut self) -> &mut Coordinate { &mut self.robot.coordinate }
//!#     fn get_backpack(&self) -> &BackPack { &self.robot.backpack }
//!#     fn get_backpack_mut(&mut self) -> &mut BackPack { &mut self.robot.backpack }
//!# }
//!fn main() {
//!     //MyRobot must implement Runnable
//!    let robot = MyRobot::new();
//!    let mut world_generator = rip_worldgenerator::MyWorldGen::new();
//!
//!    // GuiRunner is constructed similarly to Runner
//!    let gui_runner = ragnarok::GuiRunner::new(Box::new(robot), &mut world_generator).unwrap();
//!    // GuiRunner::run runs the game
//!    gui_runner.run().unwrap();
//!}
//! ```
mod gui_runner;
mod utils;
#[macro_use]
extern crate glium;

/// A wrapper of the Runner struct which runs the game and visualizes it in a GUI.
///
pub use gui_runner::GuiRunner;