use std::cmp::{max, min};
use std::sync::{Arc, Mutex};
use nalgebra_glm::UVec2;
use robotics_lib::energy::Energy;
use robotics_lib::event::events::Event;
use robotics_lib::runner::backpack::BackPack;
use robotics_lib::runner::Runnable;
use robotics_lib::world::coordinates::Coordinate;
use robotics_lib::world::World;
use crate::utils::coord_to_robot_position;
use super::PartialWorld;

pub struct RobotWrapper {
    pub ai: Box<dyn Runnable>,
    pub world: Arc<Mutex<PartialWorld>>,
    pub is_first_tick: bool
}
impl RobotWrapper {
    pub fn new(ai: Box<dyn Runnable>, world: Arc<Mutex<PartialWorld>>) -> Self {
        Self { ai, world, is_first_tick: true }
    }
}
impl Runnable for RobotWrapper {
    fn process_tick(&mut self, world: &mut World) {
        if !self.is_first_tick {
            self.ai.process_tick(world);
        } else {
            robotics_lib::interface::robot_view(self, world);
            self.is_first_tick = false;
        }

        let mut world_ref = self.world.lock().unwrap();

        world_ref.robot_position = coord_to_robot_position(self.get_coordinate());
        world_ref.world = robotics_lib::interface::robot_map(world).unwrap();
        world_ref.energy = self.get_energy().get_energy_level();
        world_ref.backpack = self.get_backpack().get_contents().clone();

        world_ref.changed = true;

        let max_coord = (world_ref.world.len() - 1) as i32;
        let x = world_ref.robot_position.x as i32;
        let y = world_ref.robot_position.y as i32;

        //tiles to be refreshed in the mesh
        let min_x = max(x-2, 0);
        let min_y = max(y-2, 0);
        let max_x = min(x+2, max_coord);
        let max_y = min(y+2, max_coord);

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                world_ref.tiles_to_refresh.insert(UVec2::new(x as u32, y as u32));
            }
        }
    }

    fn handle_event(&mut self, event: Event) {
        self.ai.handle_event(event.clone());

        match &event {
            //ignore these events
            Event::Ready | Event::DayChanged(_) | Event::EnergyRecharged(_) => {}
            Event::TimeChanged(env_cond) => {
                self.world.lock().unwrap().env_cond = env_cond.clone();
            }
            e => println!("RobotWrapper caught event {e:?}"),
        }
    }

    fn get_energy(&self) -> &Energy { self.ai.get_energy() }
    fn get_energy_mut(&mut self) -> &mut Energy { self.ai.get_energy_mut() }
    fn get_coordinate(&self) -> &Coordinate { self.ai.get_coordinate() }
    fn get_coordinate_mut(&mut self) -> &mut Coordinate { self.ai.get_coordinate_mut() }
    fn get_backpack(&self) -> &BackPack { self.ai.get_backpack() }
    fn get_backpack_mut(&mut self) -> &mut BackPack { self.ai.get_backpack_mut() }
}