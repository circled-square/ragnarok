use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use nalgebra_glm::vec2;
use robotics_lib::world::tile::Tile;
use super::PartialWorld;

// WorkerThread handles a thread which receives the world information from the game->worker channel
// and relays it through the worker->gui channel after populating the PartialWorld::tiles_to_refresh
// field with the positions of tiles that changed since the last PartialWorld received through the
// game->worker channel.
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

                                for dx in -1..=1 {
                                    for dy in -1..=1 {
                                        let x = (x as i32 + dx).clamp(0, new_world.world.len() as i32 - 1) as u32;
                                        let y = (y as i32 + dy).clamp(0, new_world.world.len() as i32 - 1) as u32;
                                        tiles_to_refresh.insert(vec2(x, y));
                                    }
                                }
                            }
                        }
                    }
                } else {
                    world_copy = Some(new_world.world.clone());
                    let world_size = new_world.world.len();
                    let x = new_world.robot_position.x;
                    let y = new_world.robot_position.y;
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            let x = (x as i32 + dx).clamp(0, world_size as i32 - 1) as u32;
                            let y = (y as i32 + dy).clamp(0, world_size as i32 - 1) as u32;
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