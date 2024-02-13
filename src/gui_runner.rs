mod robot_wrapper;
mod gui;
mod interface;


use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use nalgebra_glm::UVec2;
use robotics_lib::runner::{Runnable, Runner};
use robotics_lib::utils::LibError;
use robotics_lib::world::environmental_conditions::{EnvironmentalConditions, WeatherType};
use robotics_lib::world::tile::{Content, Tile};
use robotics_lib::world::world_generator::Generator;
use robot_wrapper::RobotWrapper;


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
    pub robot_position: UVec2,
    pub energy: usize,
    pub backpack: HashMap<Content, usize>,
    pub env_cond: EnvironmentalConditions,
    pub changed: bool,

    pub run_mode: RunMode,
}
impl PartialWorld {
    pub fn new() -> Self {
        let ret = Self {
            world: vec![],
            changed: false,
            robot_position: UVec2::default(),
            run_mode: RunMode::Paused,
            energy: 0,
            backpack: HashMap::new(),
            env_cond: EnvironmentalConditions::new(&[WeatherType::Sunny], 0, 0).unwrap(),
        };
        ret
    }
    pub fn is_null(&self) -> bool {
        self.world.len() == 0
    }
}

pub struct GuiRunner {
    runner: Runner,
    partial_world: Arc<Mutex<PartialWorld>>,
}
impl GuiRunner {
    pub fn new(robot: Box<dyn Runnable>, generator: &mut impl Generator) -> Result<GuiRunner, LibError> {
        let partial_world = Arc::new(Mutex::new(PartialWorld::new()));

        let robot_wrapper = RobotWrapper::new(robot, partial_world.clone());
        let mut runner = Runner::new(Box::new(robot_wrapper), generator)?;
        runner.game_tick()?; // first tick needed to fully init partial_world

        Ok(Self { runner, partial_world })
    }

    pub fn run(mut self) -> Result<(), LibError> {
        let gui_partial_world = self.partial_world.clone();
        let gui_thread_handle = thread::spawn(move || {
            let gui = gui::GUI::new(gui_partial_world, "Visualizer");
            gui.run();
        });

        let mut last_tick_begin = std::time::Instant::now();
        'main_game_loop: loop {

            loop {
                let run_mode = {
                    let mut world = self.partial_world.lock().unwrap();
                    match world.run_mode {
                        RunMode::SingleTick => {
                            world.run_mode = RunMode::Paused;
                            break;
                        }
                        RunMode::Continuous(None) => break,
                        RunMode::Terminate => break 'main_game_loop,
                        run_mode => run_mode, //run_mode is either Continuous(Some(_)) or Paused; either way release the lock
                    }
                };
                // capped continuous mode and paused are handled separately bacause we need to release the lock before we call thread::sleep
                match run_mode {
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
                    _ => unreachable!()
                }
            }

            last_tick_begin = std::time::Instant::now();
            self.runner.game_tick().unwrap();
        }

        gui_thread_handle.join().expect("failed to join thread");
        Ok(())
    }
}
