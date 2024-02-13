use nalgebra_glm::{vec2, Vec2, Vec3, vec3};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};
use crate::gui_runner::gui::UP;
use nalgebra_glm as glm;

pub struct KeyboardEventHandler {
    sprint_pressed: bool,
    direction_pressed: [bool; 6], // w / s / a / d / ctrl / space
    rotation_pressed: [bool; 4], // up / down / left / right
    toggle_continuous_mode: bool,
    single_tick: bool,
    find_robot: bool,
    toggle_follow_robot: bool,

    movement_speed: f32,
    look_speed: f32,

    explanation: String,
}
impl KeyboardEventHandler {
    pub fn get_explanation(&self) -> &str { &self.explanation }
    pub fn new(movement_speed: f32, look_speed: f32) -> Self {
        Self {
            sprint_pressed: false,
            direction_pressed: [false; 6],
            rotation_pressed: [false; 4],
            toggle_continuous_mode: false,
            single_tick: false,
            find_robot: false,
            toggle_follow_robot: false,

            movement_speed,
            look_speed,

            explanation:
"WASD: control camera movement;
arrows: control camera rotation;
N: advance the game by a single tick;
M: toggle continuous execution of the game.
F: find the robot and move the camera to it
G: toggle following the robot with the camera".into()
        }
    }

    pub fn process_input(&mut self, input: KeyboardInput) -> ProcessedKeyboardInput {
        self.handle(input);
        self.get_processed_input()
    }
    fn handle(&mut self, input: KeyboardInput) {
        let pressed = match input.state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };
        if let Some(keycode) = input.virtual_keycode {
            match keycode {
                VirtualKeyCode::W =>        self.direction_pressed[0] = pressed,
                VirtualKeyCode::S =>        self.direction_pressed[1] = pressed,
                VirtualKeyCode::A =>        self.direction_pressed[2] = pressed,
                VirtualKeyCode::D =>        self.direction_pressed[3] = pressed,
                VirtualKeyCode::Space =>    self.direction_pressed[4] = pressed,
                VirtualKeyCode::LControl => self.direction_pressed[5] = pressed,

                VirtualKeyCode::LShift => self.sprint_pressed = pressed,

                VirtualKeyCode::Up =>    self.rotation_pressed[0] = pressed,
                VirtualKeyCode::Down =>  self.rotation_pressed[1] = pressed,
                VirtualKeyCode::Left =>  self.rotation_pressed[2] = pressed,
                VirtualKeyCode::Right => self.rotation_pressed[3] = pressed,

                VirtualKeyCode::M => {
                    if pressed {
                        self.toggle_continuous_mode = true;
                    }
                }
                VirtualKeyCode::N => {
                    if pressed {
                        self.single_tick = true;
                    }
                }
                VirtualKeyCode::F => {
                    if pressed {
                        self.find_robot = true;
                    }
                }
                VirtualKeyCode::G => {
                    if pressed {
                        self.toggle_follow_robot = true;
                    }
                }

                _ => {}
            }
        }
    }

    fn get_processed_input(&mut self) -> ProcessedKeyboardInput {
        let cam_rotation_input_vector = vec2(
            (self.rotation_pressed[3] as i8 - self.rotation_pressed[2] as i8) as f32,
            (self.rotation_pressed[0] as i8 - self.rotation_pressed[1] as i8) as f32,
        );
        let cam_turn_speed = cam_rotation_input_vector * self.look_speed;

        let input_vector = vec3(
            (self.direction_pressed[0] as i8 - self.direction_pressed[1] as i8) as f32,
            (self.direction_pressed[2] as i8 - self.direction_pressed[3] as i8) as f32,
            (self.direction_pressed[4] as i8 - self.direction_pressed[5] as i8) as f32,
        );
        let relative_cam_speed = input_vector * self.movement_speed * if self.sprint_pressed {5.0} else {1.0};

        let toggle_continuous_mode = self.toggle_continuous_mode;
        self.toggle_continuous_mode = false;

        let single_tick = self.single_tick;
        self.single_tick = false;

        let find_robot = self.find_robot;
        self.find_robot = false;

        let toggle_follow_robot = self.toggle_follow_robot;
        self.toggle_follow_robot = false;


        ProcessedKeyboardInput { relative_cam_speed, cam_turn_speed, toggle_continuous_mode, single_tick, find_robot, toggle_follow_robot }
    }
}

#[derive(Default)]
pub struct ProcessedKeyboardInput {
    relative_cam_speed : Vec3,
    cam_turn_speed : Vec2,

    pub toggle_continuous_mode: bool,
    pub single_tick: bool,
    pub find_robot: bool,
    pub toggle_follow_robot: bool,
}

impl ProcessedKeyboardInput {
    pub fn update_cam_dir_and_pos(&self, cam_dir: &mut Vec3, cam_pos: &mut Vec3, delta: f32, up: Vec3) {
        *cam_dir = Self::rotate_camera(*cam_dir, self.cam_turn_speed, delta, up);
        *cam_pos += Self::camera_movement(*cam_dir, self.relative_cam_speed, delta);
    }

    fn rotate_camera(cam_dir: Vec3, cam_turn_speed: Vec2, delta: f32, up: Vec3) -> Vec3 {
        let cam_dir_right = cam_dir.cross(&up);

        let mut cam_dir= cam_dir;
        cam_dir = glm::rotate_vec3(&cam_dir, cam_turn_speed.x * delta, &up);
        cam_dir = glm::rotate_vec3(&cam_dir, cam_turn_speed.y * delta, &cam_dir_right);
        cam_dir = cam_dir.normalize();
        cam_dir.y = cam_dir.y.clamp(-0.7, 0.7);
        cam_dir = cam_dir.normalize();

        cam_dir
    }

    fn camera_movement(cam_dir: Vec3, relative_cam_speed: Vec3, delta: f32) -> Vec3 {
        let cam_dir_right = cam_dir.cross(&UP);
        (relative_cam_speed.x * cam_dir + relative_cam_speed.y * cam_dir_right + relative_cam_speed.z * UP) * delta
    }
}
