use std::collections::HashSet;
use std::sync::mpsc::{SyncSender};
use nalgebra_glm::UVec2;
use robotics_lib::energy::Energy;
use robotics_lib::event::events::Event;
use robotics_lib::runner::backpack::BackPack;
use robotics_lib::runner::Runnable;
use robotics_lib::world::coordinates::Coordinate;
use robotics_lib::world::World;
use super::PartialWorld;

// RobotWrapper is a wrapper around Runnable, which itself implements Runnable. It serves the
// purpose of sending world information through the gui->worker channel, since robotics_lib offers
// no way of doing this that wouldn't involve calling gui code in the Runnable. This is simply a
// workaround which allows me to "have a full wine barrel and a drunk wife" (as they would say in
// Italy) by seamlessly wrapping the user's robot in this struct which does all the ugly things
// necessary to communicate with the gui.

pub struct RobotWrapper {
    ai: Box<dyn Runnable>,
    to_worker_tx: SyncSender<PartialWorld>,
    is_first_tick: bool,
}
impl RobotWrapper {
    pub fn new(ai: Box<dyn Runnable>, to_worker_tx: SyncSender<PartialWorld>) -> Self {
        Self { ai, to_worker_tx, is_first_tick: true }
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

        let world_data = PartialWorld {
            world: robotics_lib::interface::robot_map(world).unwrap(),
            tiles_to_refresh: HashSet::new(),
            robot_position: coord_to_robot_position(self.get_coordinate()),
            energy: self.get_energy().get_energy_level(),
            backpack: self.get_backpack().get_contents().clone(),
            env_cond: robotics_lib::interface::look_at_sky(&world),
        };
        let _ = self.to_worker_tx.send(world_data); // do not unwrap, since Err simply means the GUI was closed and this thread is also about to exit
    }

    fn handle_event(&mut self, event: Event) {
        self.ai.handle_event(event.clone());

        match &event {
            //ignore these events
            Event::Ready | Event::Moved(_,_) | Event::DayChanged(_) | Event::EnergyRecharged(_)  | Event::TimeChanged(_) => {}
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
fn coord_to_robot_position(c: &Coordinate) -> UVec2 {
    UVec2::new(c.get_row() as u32, c.get_col() as u32)
}
