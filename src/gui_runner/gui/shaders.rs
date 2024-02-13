
pub fn make_program(display: &glium::Display) -> Result<glium::Program, glium::ProgramCreationError> {
    let vtx_shader_src = {r#"
            #version 150

            in vec3 position;
            in vec3 color;

            out vec3 v_color;

            uniform mat4 mvp;

            void main() {
                v_color = color;
                gl_Position = mvp * vec4(position, 1.0);
            }
        "#};

    let frag_shader_src = {r#"
            #version 150

            in vec3 v_color;
            out vec4 color;
            uniform vec3 u_light;

            void main() {
                color = vec4(v_color, 1.0);//vec4(mix(dark_color, regular_color, brightness), 1.0);
            }
        "#};

    glium::Program::from_source(display, vtx_shader_src, frag_shader_src, None)
}