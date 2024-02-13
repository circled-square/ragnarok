use nalgebra_glm::UVec2;
use robotics_lib::world::coordinates::Coordinate;

pub(crate) fn coord_to_robot_position(c: &Coordinate) -> UVec2 {
    UVec2::new(c.get_row() as u32, c.get_col() as u32)
}
