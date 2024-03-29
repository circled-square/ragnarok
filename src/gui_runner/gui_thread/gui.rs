mod world_mesh;
mod shaders;
mod keyboard_event_handler;
mod frame_delta_timer;
mod compute_mvp;

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use glium::index::PrimitiveType;
use glium::Surface;
use imgui::{Condition, SliderFlags, StyleColor, TreeNodeFlags};
use imgui_winit_support::HiDpiMode;
use winit::window::WindowBuilder;
use nalgebra_glm as glm;
use glm::{Vec3, vec3};
use world_mesh::WorldMesh;
use frame_delta_timer::FrameDeltaTimer;
use keyboard_event_handler::{KeyboardEventHandler, ProcessedKeyboardInput};
use super::{PartialWorld, RunMode};

//extension that allows running winit on a thread that isn't the main thread. necessary since it's hard to run runner outside of main thread (it's not Send)
#[cfg(target_os = "linux")] use winit::platform::unix::EventLoopBuilderExtUnix;
#[cfg(target_os = "windows")] use winit::platform::windows::EventLoopBuilderExtWindows;
#[cfg(target_os = "macos")] use winit::platform::macos::EventLoopBuilderExtMacOS;

// GUI handles:
// - rendering:
//   - world rendering through WorldMesh
//   - widget rendering through imgui
// - user input:
//   - keyboard input through KeyboardEventHandler (which uses winit events)
//   - graphical input through imgui
// - inter thread communication:
//   - receives PartialWorld through worker->gui (uses feeds it to WorldMesh to turn it into a mesh)
//   - sends RunMode through gui->game (when the user requests it with keyboard or graphical input)

pub struct GUI {
    rx_from_worker: Receiver<PartialWorld>,
    tx_to_game: Sender<RunMode>,
    world_copy: PartialWorld,

    event_loop: winit::event_loop::EventLoop<()>,
    display: glium::Display,
    imgui_ctx: imgui::Context,
    imgui_platform: imgui_winit_support::WinitPlatform,
    imgui_renderer: imgui_glium_renderer::Renderer,

    world_mesh: WorldMesh,
    shader_program: glium::Program,

    kbd_event_handler: KeyboardEventHandler,
}
impl GUI {
    pub fn new(window_title: &str, rx_from_worker: Receiver<PartialWorld>, tx_to_game: Sender<RunMode>) -> Self {
        let event_loop =
            winit::event_loop::EventLoopBuilder::new()
            .with_any_thread(true)
            .build();

        let window_builder =
            WindowBuilder::new()
                .with_title(window_title);

        let display = glium::Display::new(window_builder, glium::glutin::ContextBuilder::new(), &event_loop).unwrap();

        let mut imgui_ctx = imgui::Context::create();
        imgui_ctx.set_ini_filename(None); //for some reason loading imgui.ini files sometimes causes crashes
        imgui_ctx.fonts().build_alpha8_texture();

        let mut imgui_platform = imgui_winit_support::WinitPlatform::init(&mut imgui_ctx);
        imgui_platform.attach_window(imgui_ctx.io_mut(), &display.gl_window().window(), HiDpiMode::Default);

        let imgui_renderer = imgui_glium_renderer::Renderer::init(&mut imgui_ctx, &display).unwrap();
        let world_copy = rx_from_worker.recv().unwrap();
        let world_mesh = WorldMesh::new(world_copy.world.len(), 10, &display);
        let shader_program = shaders::make_program(&display).unwrap();

        let kbd_event_handler = KeyboardEventHandler::new(50.0, 1.0);

        Self { rx_from_worker, tx_to_game, world_copy, event_loop, display, imgui_ctx, imgui_platform, imgui_renderer, world_mesh, shader_program, kbd_event_handler }
    }

    fn toggle_continuous_mode(run_mode: &mut RunMode, tx_to_game: &Sender<RunMode>, last_was_uncapped: bool, last_ticks_per_second_cap: f32) {
        *run_mode = match run_mode {
            RunMode::Continuous(_) => RunMode::Paused,
            _ => {
                let cap = if last_was_uncapped { None } else { Some(last_ticks_per_second_cap) };
                RunMode::Continuous(cap)
            }
        };
        let _ = tx_to_game.send(*run_mode);
    }
    fn request_single_tick(run_mode: &mut RunMode, tx_to_game: &Sender<RunMode>) {
        *run_mode = RunMode::SingleTick;
        let _ = tx_to_game.send(*run_mode);
    }
    pub fn run(mut self) -> () {
        let mut kbd_input = ProcessedKeyboardInput::default();
        let (mut cam_dir, mut cam_pos) = {
            let robot_pos = self.world_copy.robot_position;
            let elevation = {
                let w = &self.world_copy;
                w.world[w.robot_position.x as usize][w.robot_position.y as usize].as_ref().unwrap().elevation
            };
            let cam_dir = vec3(-1.0, -1.0, -1.0).normalize();
            let cam_pos = vec3(robot_pos.x as f32, world_mesh::elevation_to_mesh_space_y(elevation as f32), robot_pos.y as f32) - cam_dir * 30.0;
            (cam_dir, cam_pos)
        };

        let mut frame_delta_timer = FrameDeltaTimer::new();

        let mut last_ticks_per_second_cap = 5.0_f32;
        let mut last_was_uncapped = false;
        let mut follow_robot = false;
        let mut find_robot = false;
        let mut enable_skybox = true;

        let mut run_mode = RunMode::Paused;

        self.event_loop.run(move |ev, _window_target, _control_flow| {
            self.imgui_platform.handle_event(self.imgui_ctx.io_mut(), &self.display.gl_window().window(), &ev);
            match ev {
                //close requests and keyboard input
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::CloseRequested => {
                        run_mode = RunMode::Terminate;
                        let _ = self.tx_to_game.send(run_mode);

                        _control_flow.set_exit();
                    },
                    winit::event::WindowEvent::KeyboardInput{ input, .. } => {
                        kbd_input = self.kbd_event_handler.process_input(input);

                        if kbd_input.toggle_continuous_mode {
                            Self::toggle_continuous_mode(&mut run_mode, &self.tx_to_game, last_was_uncapped, last_ticks_per_second_cap);
                        } else if kbd_input.single_tick {
                            Self::request_single_tick(&mut run_mode, &self.tx_to_game);
                        }

                        if kbd_input.toggle_follow_robot {
                            follow_robot = !follow_robot;
                        }
                        find_robot = find_robot || kbd_input.find_robot;
                    }
                    _ => {}
                },
                // MainEventsCleared can be used for rendering since we don't lock the framerate
                winit::event::Event::MainEventsCleared => {
                    let delta = frame_delta_timer.get_delta_and_reset();

                    // update world_copy
                    {
                        let mut tiles_to_refresh = HashSet::new();
                        let mut new_world = None;
                        for mut received_world in self.rx_from_worker.try_iter() {
                            tiles_to_refresh.extend(received_world.tiles_to_refresh.drain());

                            new_world = Some(received_world);
                        }

                        if let Some(new_world) = new_world {
                            self.world_copy = new_world;
                            self.world_copy.tiles_to_refresh = tiles_to_refresh;
                        }
                    }


                    // move/rotate camera
                    kbd_input.update_cam_dir_and_pos(&mut cam_dir, &mut cam_pos, delta);

                    // make the camera go to the robot if needed
                    if find_robot || follow_robot {
                        let w = &self.world_copy;
                        let elevation = w.world[w.robot_position.x as usize][w.robot_position.y as usize].as_ref().unwrap().elevation;
                        cam_pos = vec3(self.world_copy.robot_position.x as f32, world_mesh::elevation_to_mesh_space_y(elevation as f32), self.world_copy.robot_position.y as f32) - cam_dir * 30.0;

                        find_robot = false;
                    }
                    let world_size = self.world_copy.world.len() as f32;
                    cam_pos.x = cam_pos.x.clamp(-10.0, world_size+10.0);
                    cam_pos.y = cam_pos.y.clamp(-world_size / 2.0 - 10.0, world_size / 2.0 + 10.0);
                    cam_pos.z = cam_pos.z.clamp(-10.0, world_size+10.0);

                    // rendering
                    {
                        let mut target = self.display.draw();

                        let mvp = compute_mvp::compute_mvp(target.get_dimensions(), cam_pos, cam_dir);

                        let draw_params = glium::DrawParameters {
                            depth: glium::Depth {
                                test: glium::draw_parameters::DepthTest::IfLess,
                                write: true,
                                .. Default::default()
                            },
                            multisampling: false,
                            dithering: false,

                            .. Default::default()
                        };

                        target.clear_color_and_depth((0.2, 0.2, 0.2, 1.0), 1.0);

                        //render world
                        {
                            // update vbo with new world information
                            self.world_mesh.update(&mut self.world_copy, &self.display, enable_skybox);
                            self.world_copy.tiles_to_refresh.clear();

                            target.draw(&self.world_mesh.vbo, &glium::index::NoIndices(PrimitiveType::TrianglesList),
                                        &self.shader_program,&uniform! { mvp:  *mvp.as_ref() }, &draw_params).unwrap();
                        }

                        //render imgui
                        {
                            self.imgui_platform.prepare_frame(self.imgui_ctx.io_mut(), self.display.gl_window().window()).unwrap();
                            let ui = self.imgui_ctx.new_frame();
                            self.imgui_platform.prepare_render(&ui, self.display.gl_window().window());

                            ui.window("Ragnarok")
                                .size([300.0, 550.0], Condition::FirstUseEver)
                                .build(|| {
                                    if ui.collapsing_header("Simulation settings", TreeNodeFlags::DEFAULT_OPEN) {
                                        ui.indent();

                                        let continuous = match run_mode {
                                            RunMode::Continuous(_) => true,
                                            _ => false,
                                        };

                                        let btn_text = if continuous {"Stop"} else {"Run"};
                                        if ui.button(btn_text) {
                                            Self::toggle_continuous_mode(&mut run_mode, &self.tx_to_game, last_was_uncapped, last_ticks_per_second_cap);
                                        }

                                        ui.same_line();

                                        ui.disabled(continuous, || {
                                            if ui.button("Run single tick") {
                                                Self::request_single_tick(&mut run_mode, &self.tx_to_game);
                                            }
                                        });

                                        let mut changed = false;

                                        let greyed_out_text_if_not_continuous = if !continuous {
                                            Some(ui.push_style_color(StyleColor::Text, [0.4, 0.4, 0.4, 1.0]))
                                        } else { None };

                                        changed = changed || ui.checkbox("Uncapped?", &mut last_was_uncapped);

                                        let greyed_out_text_if_uncapped = if last_was_uncapped {
                                            Some(ui.push_style_color(StyleColor::Text, [0.4, 0.4, 0.4, 1.0]))
                                        } else { None };

                                       changed = changed || ui.slider_config("speed", 1.0, 200.0)
                                           .flags(SliderFlags::LOGARITHMIC)
                                           .build(&mut last_ticks_per_second_cap);
                                        if let Some(t) = greyed_out_text_if_uncapped { t.pop(); }
                                        if let Some(t) = greyed_out_text_if_not_continuous { t.pop(); }

                                        let cap = if last_was_uncapped { None } else { Some(last_ticks_per_second_cap) };
                                        if changed && continuous {
                                            run_mode = RunMode::Continuous(cap);
                                            let _ = self.tx_to_game.send(run_mode);
                                        }

                                        ui.unindent();
                                    }

                                    ui.separator();

                                    if ui.collapsing_header("Robot", TreeNodeFlags::DEFAULT_OPEN) {
                                        ui.indent();

                                        ui.checkbox("Follow robot", &mut follow_robot);
                                        ui.disabled(follow_robot, || {
                                            ui.same_line();
                                            find_robot = find_robot || ui.button("Find robot");
                                        });

                                        ui.text_wrapped(format!("Position: {:?}", self.world_copy.robot_position.as_ref()));

                                        ui.text_wrapped("Energy:");
                                        ui.same_line();
                                        imgui::ProgressBar::new(self.world_copy.energy as f32 / 1000.0)
                                            .overlay_text(format!("{}", self.world_copy.energy))
                                            .build(&ui);

                                        let mut backpack_is_empty = true;
                                        if ui.collapsing_header("Backpack:", TreeNodeFlags::DEFAULT_OPEN) {
                                            ui.indent();

                                            for (k, v) in self.world_copy.backpack.iter() {
                                                if *v != 0 {
                                                    ui.text_wrapped(format!("{k}: {v}"));
                                                    backpack_is_empty = false;
                                                }
                                            }
                                            if backpack_is_empty {
                                                ui.text_wrapped("(empty)");
                                            }

                                            ui.unindent();
                                        }

                                        ui.unindent()
                                    }

                                    ui.separator();

                                    if ui.collapsing_header("Environmental conditions", TreeNodeFlags::DEFAULT_OPEN) {
                                        ui.indent();
                                        let env = &self.world_copy.env_cond;
                                        ui.text_wrapped(format!("Time of day: {}, {:?}", env.get_time_of_day_string(), env.get_time_of_day()));
                                        ui.text_wrapped(format!("Weather: {:?}", env.get_weather_condition()));
                                        ui.checkbox("Enable skybox", &mut enable_skybox);

                                        ui.unindent();
                                    }

                                    ui.separator();

                                    if ui.collapsing_header("Controls", TreeNodeFlags::empty()) {
                                        ui.indent();
                                        ui.text_wrapped(self.kbd_event_handler.get_explanation());
                                        ui.unindent();
                                    }

                                    ui.separator();

                                    ui.text_wrapped(format!("FPS: {}", frame_delta_timer.get_average_fps() as u32));
                                });

                            let draw_data = self.imgui_ctx.render();
                            self.imgui_renderer.render(&mut target, draw_data).unwrap();
                        }

                        target.finish().unwrap();
                    }
                },
                _ => {}
            }
        });
    }
}

const UP : Vec3 = Vec3::new(0.0, 1.0, 0.0);
