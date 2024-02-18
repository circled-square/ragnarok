use std::f32::consts::PI;
use nalgebra_glm as glm;
use glm::{Mat4, Vec3, vec3};
use super::UP;

// compute_mvp is a simple utility function which, given the frame(buffer) size, the camera position
// and the camera direction returns the mvp (model-view-projection) matrix for the world. Note that
// the model matrix is simply the identity matrix since the world is the only rendered object.

pub fn compute_mvp(frame_size: (u32, u32), cam_pos: Vec3, cam_dir: Vec3) -> Mat4 {
    let model = Mat4::identity();
    proj_matrix(frame_size, PI / 3.0) * view_matrix(cam_pos, cam_dir, UP) * model
}

fn view_matrix(cam_pos: Vec3, cam_dir: Vec3, up: Vec3) -> Mat4 {
    let f = cam_dir.normalize();

    let s = up.cross(&f);
    let s_norm = s.normalize();

    let u = f.cross(&s_norm);

    let p = -vec3(cam_pos.dot(&s_norm), cam_pos.dot(&u), cam_pos.dot(&f));


    Mat4::new(
        s_norm.x,      s_norm.y,       s_norm.z,    p.x,
        u.x,           u.y,            u.z,         p.y,
        f.x,           f.y,            f.z,         p.z,
        0.0, 0.0,      0.0,    1.0,
    )
}
fn proj_matrix(frame_size: (u32, u32), fov: f32) -> Mat4 {
    let (width, height) = frame_size;
    let aspect_ratio = width as f32 / height as f32;

    glm::perspective_lh(aspect_ratio, fov, 1.0/32.0, 8192.0)
}
