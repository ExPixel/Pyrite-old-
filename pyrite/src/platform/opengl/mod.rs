use pyrite_gba::GbaVideoOutput;
mod globj;

pub struct PyriteGL {
    vertex_array: globj::VertexArray,
    vertex_buffer: globj::Buffer,
    elems_buffer: globj::Buffer,
    program: globj::Program,
    vertex_shader: globj::Shader,
    fragment_shader: globj::Shader,

    texture: globj::Texture,
    texture_data: Box<[u16; 240 * 160]>,
}

impl PyriteGL {
    pub fn new() -> PyriteGL {
        let vertex_array = globj::VertexArray::new();
        vertex_array.bind();

        let vertex_buffer = globj::Buffer::new(globj::BufferType::ArrayBuffer);
        vertex_buffer.bind();

        vertex_buffer.set_data(
            &[
                //  Position          TexCoord
                -1.0f32, 1.0f32, 0.0f32, 0.0f32, // Top Left
                1.0f32, 1.0f32, 1.0f32, 0.0f32, // Top Right
                1.0f32, -1.0f32, 1.0f32, 1.0f32, // Bottom Right
                -1.0f32, -1.0f32, 0.0f32, 1.0f32, // Bottom Left
            ],
            globj::BufferUsage::StaticDraw,
        );

        let elems_buffer = globj::Buffer::new(globj::BufferType::ElementArrayBuffer);
        elems_buffer.bind();
        elems_buffer.set_data(&[0i32, 1, 2, 2, 3, 0], globj::BufferUsage::StaticDraw);

        let vertex_shader = globj::Shader::compile(globj::ShaderType::Vertex, VERTEX_SHADER)
            .expect("faield to compile vertex shader");
        let fragment_shader = globj::Shader::compile(globj::ShaderType::Fragment, FRAGMENT_SHADER)
            .expect("failed to compile fragment shader");
        let program = globj::Program::link(&[&vertex_shader, &fragment_shader])
            .expect("failed to link GL program");
        let attrib_pos = program.attrib_location("Position\0");
        let attrib_texcoord = program.attrib_location("TexCoord\0");
        unsafe {
            let szfloat = std::mem::size_of::<f32>() as i32;
            gl::EnableVertexAttribArray(attrib_pos as _);
            gl::VertexAttribPointer(
                attrib_pos as _,
                2,
                gl::FLOAT,
                gl::FALSE,
                (4 * szfloat) as _,
                std::mem::transmute(0usize),
            );
            gl::EnableVertexAttribArray(attrib_texcoord as _);
            gl::VertexAttribPointer(
                attrib_texcoord as _,
                2,
                gl::FLOAT,
                gl::FALSE,
                (4 * szfloat) as _,
                std::mem::transmute(2usize * szfloat as usize),
            );
        }

        unsafe { gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1) };
        let texture = globj::Texture::new::<&[u8]>(
            240,
            160,
            globj::InternalPixelFormat::RGBA,
            globj::PixelDataFormat::BGRA,
            globj::PixelDataType::UnsignedShort_1_5_5_5_Rev,
            None,
        );

        globj::check_gl_errors(|e| log::error!("GL Error: {}", e));

        PyriteGL {
            vertex_array: vertex_array,
            vertex_buffer: vertex_buffer,
            elems_buffer: elems_buffer,
            program: program,
            vertex_shader: vertex_shader,
            fragment_shader: fragment_shader,
            texture: texture,
            texture_data: Box::new([0; 240 * 160]),
        }
    }

    pub fn build_frame(&mut self) {
        self.texture.bind();
        unsafe {
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 2);
            self.texture.set_pixels::<&[u8]>(
                0,
                0,
                240,
                160,
                globj::PixelDataFormat::RGBA,
                globj::PixelDataType::UnsignedShort_1_5_5_5_Rev,
                std::mem::transmute(&self.texture_data[0..]),
            );
        }
    }

    pub fn render(&mut self) {
        unsafe {
            gl::ClearColor(0.5, 0.5, 0.5, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        self.program.bind();
        self.vertex_array.bind();
        self.vertex_buffer.bind();
        self.elems_buffer.bind();

        unsafe { gl::ActiveTexture(gl::TEXTURE0) };

        globj::check_gl_errors(|e| log::error!("GL Error: {}", e));

        unsafe {
            gl::DrawElements(
                gl::TRIANGLES,
                6,
                gl::UNSIGNED_INT,
                std::mem::transmute(0usize),
            );
        }
    }
}

impl GbaVideoOutput for PyriteGL {
    fn display_line(&mut self, line: u32, pixels: &[u16; 240]) {
        let offset = (line as usize) * 240;
        (&mut self.texture_data[offset..(offset + 240)]).copy_from_slice(pixels);
    }

    fn pre_frame(&mut self) {
        /* NOP */
    }

    fn post_frame(&mut self) {
        /* NOP */
    }
}

// /// Create column-major orthographic matrix.
// fn ortho(b: f32, t: f32, l: f32, r: f32, n: f32, f: f32) -> [[f32; 4]; 4] {
//     let mut m = [[0f32; 4]; 4];
//
//     m[0][0] = 2.0 / (r - l);
//     m[0][1] = 0.0;
//     m[0][2] = 0.0;
//     m[0][3] = 0.0;
//
//     m[1][0] = 0.0;
//     m[1][1] = 2.0 / (t - b);
//     m[1][2] = 0.0;
//     m[1][3] = 0.0;
//
//     m[2][0] = 0.0;
//     m[2][1] = 0.0;
//     m[2][2] = -2.0 / (f - n);
//     m[2][3] = 0.0;
//
//     m[3][0] = -(r + l) / (r - l);
//     m[3][1] = -(t + b) / (t - b);
//     m[3][2] = -(f + n) / (f - n);
//     m[3][3] = 1.0;
//
//     return m;
// }

pub const VERTEX_SHADER: &str = "\
#version 120

attribute vec2 Position;
attribute vec2 TexCoord;
varying vec2 FragUV;

void main() {
    FragUV = TexCoord;
    gl_Position = vec4(Position.xy, 1.0, 1.0);
}\0";

pub const FRAGMENT_SHADER: &str = "\
#version 120

uniform sampler2D Texture;
varying vec2 FragUV;

void main() {
    gl_FragColor = texture2D(Texture, FragUV.st);
}\0";
