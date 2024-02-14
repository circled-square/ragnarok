use std::cmp::{max, min};
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use nalgebra_glm::vec2;
use robotics_lib::world::tile::Tile;
use crate::gui_runner::PartialWorld;

pub struct WorkerThread {
    game_to_worker_rx: Receiver<PartialWorld>,
    worker_to_gui_tx: Sender<PartialWorld>,
}
impl WorkerThread {
    pub fn new(game_to_worker_rx: Receiver<PartialWorld>, worker_to_gui_tx: Sender<PartialWorld>) -> Self {
        Self { game_to_worker_rx, worker_to_gui_tx }
    }

    pub fn start(self) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut world_copy = Option::<Vec<Vec<Option<Tile>>>>::None;

        loop {
            let new_world = match self.game_to_worker_rx.recv() {
                Ok(w) => w,
                Err(_) => return, // if the other end is closed simply terminate this thread
            };

            let mut tiles_to_refresh = HashSet::new();

            if let Some(world_copy) = &mut world_copy {
                for x in 0..new_world.world.len() {
                    for y in 0..new_world.world.len() {
                        if world_copy[x][y] != new_world.world[x][y] {
                            world_copy[x][y] = new_world.world[x][y].clone();

                            for x in max(x, 1) - 1..=min(x + 1, new_world.world.len() - 1) {
                                for y in max(y, 1) - 1..=min(y + 1, new_world.world.len() - 1) {
                                    tiles_to_refresh.insert(vec2(x as u32, y as u32));
                                }
                            }
                        }
                    }
                }
            } else {
                world_copy = Some(new_world.world.clone());
                for x in max(new_world.robot_position.x, 1) - 1..=min(new_world.robot_position.x + 1, new_world.world.len() as u32 - 1) {
                    for y in max(new_world.robot_position.y, 1) - 1..=min(new_world.robot_position.y + 1, new_world.world.len() as u32 - 1) {
                        tiles_to_refresh.insert(vec2(x, y));
                    }
                }
            }

            let mut new_world = new_world;
            new_world.tiles_to_refresh = tiles_to_refresh;
            match self.worker_to_gui_tx.send(new_world) {
                Ok(()) => {}
                Err(_) => return, // if the other end is closed simply terminate this thread
            }
        }
    })
}
}