use midgard::world_generator::{WorldGenerator, WorldGeneratorParameters};
use rand::random;

use robotics_lib::runner::{Runnable};
use robotics_lib::energy::Energy;
use robotics_lib::event::events::Event;
use robotics_lib::interface::Direction;
use robotics_lib::runner::backpack::BackPack;
use robotics_lib::world::coordinates::Coordinate;
use robotics_lib::world::World;
use visualizer::GuiRunner;


pub struct ExampleRobot {
    pub robot: robotics_lib::runner::Robot,
}
impl ExampleRobot {
    pub fn new() -> Self {
        Self { robot: robotics_lib::runner::Robot::new() }
    }
}
impl Runnable for ExampleRobot {
    fn process_tick(&mut self, world: &mut World) {
        let directions = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];
        let random_direction = random::<usize>() % 4;
        let random_direction = directions[random_direction].clone();

        robotics_lib::interface::go(self, world, random_direction).map(|_| 0).unwrap_or(0);
    }

    fn handle_event(&mut self, _event: Event) {}

    fn get_energy(&self) -> &Energy { &self.robot.energy }
    fn get_energy_mut(&mut self) -> &mut Energy { &mut self.robot.energy }
    fn get_coordinate(&self) -> &Coordinate { &self.robot.coordinate }
    fn get_coordinate_mut(&mut self) -> &mut Coordinate { &mut self.robot.coordinate }
    fn get_backpack(&self) -> &BackPack { &self.robot.backpack }
    fn get_backpack_mut(&mut self) -> &mut BackPack { &mut self.robot.backpack }
}

fn main() {
    let robot = ExampleRobot::new();
    let mut world_gen = WorldGenerator::new(WorldGeneratorParameters {
        elevation_multiplier: Some(100.0),
        world_scale: 0.3,
        world_size: 100,
        amount_of_rivers: Some(3.0),

        ..Default::default()
    });

    let gui_runner = GuiRunner::new(Box::new(robot), &mut world_gen).unwrap();

    gui_runner.run().unwrap();
}