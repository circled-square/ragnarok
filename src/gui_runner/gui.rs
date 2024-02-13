mod world_mesh;
mod shaders;
mod keyboard_event_handler;
mod frame_delta_timer;

use std::f32::consts::PI;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use glium::{Display, Frame, Program, Surface};
use glium::index::PrimitiveType;
use imgui::{Condition, SliderFlags, TreeNodeFlags};
use imgui_winit_support::HiDpiMode;
use nalgebra_glm::{Mat4, Vec3, vec3};
use winit::event_loop::{EventLoop};
use winit::window::WindowBuilder;
use nalgebra_glm as glm;
use winit::platform::unix::EventLoopBuilderExtUnix;
use world_mesh::{WorldMesh};
use crate::gui_runner::gui::frame_delta_timer::FrameDeltaTimer;
use crate::gui_runner::gui::keyboard_event_handler::{KeyboardEventHandler, ProcessedKeyboardInput};
use super::{PartialWorld, RunMode};


const UP : Vec3 = Vec3::new(0.0, 1.0, 0.0);
pub fn view_matrix(position: Vec3, direction: Vec3, up: Vec3) -> Mat4 {
    let f = direction.normalize();

    let s = up.cross(&f);
    let s_norm = s.normalize();

    let u = f.cross(&s_norm);

    let p = -vec3(position.dot(&s_norm), position.dot(&u), position.dot(&f));


    Mat4::new(
        s_norm.x,      s_norm.y,       s_norm.z,    p.x,
        u.x,           u.y,            u.z,         p.y,
        f.x,           f.y,            f.z,         p.z,
        0.0, 0.0,      0.0,    1.0,
    )
}
pub fn proj_matrix(frame: &Frame, fov: f32) -> Mat4 {
    let (width, height) = frame.get_dimensions();
    let aspect_ratio = width as f32 / height as f32;

    glm::perspective_lh(aspect_ratio, fov, 0.05, 1024.0)
}

pub struct GUI {
    world: Arc<Mutex<PartialWorld>>,

    event_loop: EventLoop<()>,
    display: Display,
    imgui_ctx: imgui::Context,
    imgui_platform: imgui_winit_support::WinitPlatform,
    imgui_renderer: imgui_glium_renderer::Renderer,

    world_mesh: WorldMesh,
    shader_program: Program,

    kbd_event_handler: KeyboardEventHandler,
}

impl GUI {
    pub fn new(world: Arc<Mutex<PartialWorld>>, window_title: &str) -> Self {
        let event_loop =
            winit::event_loop::EventLoopBuilder::new()
            .with_any_thread(true)
            .build();

        let window_builder =
            WindowBuilder::new()
                .with_title(window_title);

        let display = Display::new(window_builder, glium::glutin::ContextBuilder::new(), &event_loop).unwrap();

        let mut imgui_ctx = imgui::Context::create();
        imgui_ctx.set_ini_filename(None); //for some reason loading imgui.ini files sometimes causes crashes
        imgui_ctx.fonts().build_alpha8_texture();

        let mut imgui_platform = imgui_winit_support::WinitPlatform::init(&mut imgui_ctx);
        imgui_platform.attach_window(imgui_ctx.io_mut(), &display.gl_window().window(), HiDpiMode::Default);

        let imgui_renderer = imgui_glium_renderer::Renderer::init(&mut imgui_ctx, &display).unwrap();
        let world_size = world.lock().unwrap().world.len();
        let world_mesh = WorldMesh::new(&display, world_size);
        let shader_program = shaders::make_program(&display).unwrap();

        let kbd_event_handler = KeyboardEventHandler::new(50.0, 1.0);

        Self { world, event_loop, display, imgui_ctx, imgui_platform, imgui_renderer, world_mesh, shader_program, kbd_event_handler }
    }

    pub fn run(mut self) -> () {
        let mut kbd_input = ProcessedKeyboardInput::default();

        let (initial_robot_pos, elevation) = {
            let world = self.world.lock().unwrap();
            (world.robot_position, world.world[world.robot_position.x as usize][world.robot_position.y as usize].as_ref().unwrap().elevation)
        };

        let mut cam_dir = vec3(-1.0, -1.0, -1.0).normalize();
        let mut cam_pos = vec3(initial_robot_pos.x as f32, world_mesh::elevation_to_mesh_space_y(elevation as f32), initial_robot_pos.y as f32) - cam_dir * 30.0;

        let mut frame_delta_timer = FrameDeltaTimer::new();

        let mut last_ticks_per_second_cap = 60.0_f32;
        let mut last_was_uncapped = false;
        let mut follow_robot = false;
        let mut find_robot = false;

        let mut world_copy = PartialWorld::new();

        self.event_loop.run(move |ev, _window_target, _control_flow| {
            self.imgui_platform.handle_event(self.imgui_ctx.io_mut(), &self.display.gl_window().window(), &ev);
            match ev {
                //close requests and keyboard input
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::CloseRequested => {
                        self.world.lock().unwrap().run_mode = RunMode::Terminate;

                        _control_flow.set_exit();
                    },
                    winit::event::WindowEvent::KeyboardInput{ input, .. } => {
                        kbd_input = self.kbd_event_handler.process_input(input);

                        if kbd_input.toggle_continuous_mode {
                            let mut world = self.world.lock().unwrap();
                            world.run_mode = match &world.run_mode {
                                RunMode::Continuous(_) => RunMode::Paused,
                                _ => {
                                    let cap = if last_was_uncapped { None } else { Some(last_ticks_per_second_cap) };
                                    RunMode::Continuous(cap)
                                }
                            };
                        } else if kbd_input.single_tick {
                            self.world.lock().unwrap().run_mode = RunMode::SingleTick;
                        }
                        if kbd_input.toggle_follow_robot {
                            follow_robot = !follow_robot;
                        }
                        find_robot = find_robot || kbd_input.find_robot;
                    }
                    _ => {}
                },
                //MainEventsCleared can be used for rendering since we don't lock the framerate
                winit::event::Event::MainEventsCleared => {
                    let delta = frame_delta_timer.get_delta_and_reset();
                    if find_robot || follow_robot {
                        cam_dir = vec3(-1.0, -1.0, -1.0).normalize();
                        let robot = self.world.lock().unwrap().robot_position;
                        cam_pos = vec3(robot.x as f32, world_mesh::elevation_to_mesh_space_y(elevation as f32), robot.y as f32) - cam_dir * 30.0;

                        find_robot = false;
                    }
                    kbd_input.update_cam_dir_and_pos(&mut cam_dir, &mut cam_pos, delta, UP);

                    //rendering
                    {
                        let mut target = self.display.draw();

                        let mvp = {
                            let model = Mat4::identity();
                            proj_matrix(&target, PI / 3.0) * view_matrix(cam_pos, cam_dir, UP) * model
                        };

                        let params = glium::DrawParameters {
                            depth: glium::Depth {
                                test: glium::draw_parameters::DepthTest::IfLess,
                                write: true,
                                .. Default::default()
                            },
                            .. Default::default()
                        };

                        target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

                        //render world
                        {
                            let _ignore_if_not_locked = self.world.try_lock().and_then(|mut world_ref| {
                                // update vbo with new world information
                                if world_ref.changed {
                                    self.world_mesh.update(&mut world_ref, &self.display);

                                    world_ref.changed = false;
                                }
                                Ok(())
                            });

                            target.draw(&self.world_mesh.vbo, &glium::index::NoIndices(PrimitiveType::TrianglesList), &self.shader_program,&uniform! { mvp:  *mvp.as_ref() }, &params).unwrap();
                        }

                        //render imgui
                        {
                            self.imgui_platform.prepare_frame(self.imgui_ctx.io_mut(), self.display.gl_window().window()).unwrap();
                            let ui = self.imgui_ctx.new_frame();
                            self.imgui_platform.prepare_render(&ui, self.display.gl_window().window());

                            ui.window("Info")
                                .size([300.0, 450.0], Condition::FirstUseEver)
                                .build(|| {
                                    let (run_mode, energy, backpack, env_cond) = {
                                        let _ignore_if_not_locked = self.world.try_lock().and_then(|w| {
                                            world_copy = w.deref().clone();
                                            Ok(())
                                        });
                                        let run_mode = world_copy.run_mode;
                                        let energy = world_copy.energy;
                                        let backpack = world_copy.backpack.clone();
                                        let env_cond = world_copy.env_cond.clone();
                                        (run_mode, energy, backpack, env_cond)
                                    };

                                    if ui.collapsing_header("Simulation settings", TreeNodeFlags::DEFAULT_OPEN) {
                                        ui.indent();

                                        match run_mode {
                                            RunMode::Continuous(_) => {
                                                if ui.button("Stop") {
                                                    self.world.lock().unwrap().run_mode = RunMode::Paused;
                                                } else {
                                                    let mut changed = false;
                                                    changed = changed || ui.checkbox("Uncapped?", &mut last_was_uncapped);

                                                    ui.disabled(last_was_uncapped, || {
                                                        changed = changed || ui.slider_config("speed", 0.1, 1000.0)
                                                            .flags(SliderFlags::LOGARITHMIC)
                                                            .build(&mut last_ticks_per_second_cap);
                                                    });

                                                    let cap = if last_was_uncapped { None } else { Some(last_ticks_per_second_cap) };
                                                    if changed {
                                                        self.world.lock().unwrap().run_mode = RunMode::Continuous(cap);
                                                    }
                                                }
                                            }
                                            RunMode::SingleTick | RunMode::Paused => {
                                                if ui.button("Run") {
                                                    let cap = if last_was_uncapped { None } else { Some(last_ticks_per_second_cap) };
                                                    self.world.lock().unwrap().run_mode = RunMode::Continuous(cap);
                                                } else {
                                                    ui.same_line();
                                                    if ui.button("Run single tick") {
                                                        self.world.lock().unwrap().run_mode = RunMode::SingleTick;
                                                    }
                                                }
                                            }
                                            RunMode::Terminate => { ui.text_wrapped("Simulation terminated."); }
                                        }

                                        ui.unindent();
                                    }

                                    if ui.collapsing_header("Robot", TreeNodeFlags::DEFAULT_OPEN) {
                                        ui.indent();

                                        ui.checkbox("Follow robot", &mut follow_robot);
                                        ui.disabled(follow_robot, || {
                                            ui.same_line();
                                            find_robot = find_robot || ui.button("Find robot");
                                        });

                                        ui.text_wrapped("Energy:");
                                        ui.same_line();
                                        imgui::ProgressBar::new(energy as f32 / 1000.0)
                                            .overlay_text(format!("{}", energy))
                                            .build(&ui);

                                        let mut backpack_is_empty = true;
                                        if ui.collapsing_header("Backpack:", TreeNodeFlags::DEFAULT_OPEN) {
                                            ui.indent();

                                            for (k, v) in backpack {
                                                if v != 0 {
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

                                        ui.text_wrapped(format!("Time of day: {:?}", env_cond.get_time_of_day()));
                                        ui.text_wrapped(format!("Weather: {:?}", env_cond.get_weather_condition()));

                                        ui.unindent();
                                    }

                                    ui.separator();

                                    if ui.collapsing_header("Controls", TreeNodeFlags::DEFAULT_OPEN) {
                                        ui.indent();
                                        ui.text_wrapped(self.kbd_event_handler.get_explanation());
                                        ui.unindent();
                                    }

                                    ui.separator();

                                    ui.text(format!("FPS: {}", frame_delta_timer.get_average_fps() as u32));
                                });

                            let draw_data = self.imgui_ctx.render();
                            self.imgui_renderer.render(&mut target, draw_data).unwrap();
                        }

                        target.finish().unwrap();
                    }
                },
                _ => (),
            }
        });
    }
}
