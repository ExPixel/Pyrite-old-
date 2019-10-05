use crate::api as imgui;
use crate::sys::{ImDrawData, ImDrawVert, ImDrawList, ImVec2, ImDrawIdx, ImDrawCmd};
use gl;
use gl::types::{GLint, GLuint, GLenum, GLboolean, GLsizei};
use std::mem::{transmute};
use std::ptr;

macro_rules! offset_of {
    ($Struct:path, $field:ident) => ({
        let u: $Struct = std::mem::uninitialized::<$Struct>();
        // use pattern matching to avoid accidentally going through Deref
        let &$Struct{ $field: ref f, .. } = &u;
        let o = (f as *const _ as usize).wrapping_sub(&u as *const _ as usize);
        // check that we are still within u
        debug_assert!( o < std::mem::size_of_val(&u) );
        o
    })
}

macro_rules! size_of {
    ($Type:path) => (
        std::mem::size_of::<$Type>()
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GLSLVersion(u32);
pub const GLSL_VERSION_120: GLSLVersion = GLSLVersion(120);
pub const GLSL_VERSION_130: GLSLVersion = GLSLVersion(130);
pub const GLSL_VERSION_300_ES: GLSLVersion = GLSLVersion(300);
pub const GLSL_VERSION_410_CORE: GLSLVersion = GLSLVersion(410);

pub const GL_SAMPLER_BINDING: bool = false;
pub const GL_POLYGON_MODE: bool = false;
pub const GL_CLIP_ORIGIN: bool = false;

// Global Open GL variables are kept here.
struct OpenGLGlobals {
    glsl_version: GLSLVersion,
    font_texture: GLuint,
    shader_handle: GLuint,
    vert_handle: GLuint,
    frag_handle: GLuint,
    attrib_location_tex: GLint,
    attrib_location_proj_mtx: GLint,
    attrib_location_position: GLint,
    attrib_location_uv: GLint,
    attrib_location_color: GLint,
    vbo_handle: GLuint,
    elements_handle: GLuint,

    // only used if this is in single context mode.
    vao_handle: GLuint,
    single_context_mode: bool,
}

#[allow(non_upper_case_globals)]
static mut g: OpenGLGlobals = OpenGLGlobals {
    glsl_version: GLSLVersion(0),
    font_texture: 0,
    shader_handle: 0,
    vert_handle: 0,
    frag_handle: 0,
    attrib_location_tex: 0,
    attrib_location_proj_mtx: 0,
    attrib_location_position: 0,
    attrib_location_uv: 0,
    attrib_location_color: 0,
    vbo_handle: 0,
    elements_handle: 0,
    vao_handle: 0,
    single_context_mode: false,
};


pub fn init(glsl_version: Option<GLSLVersion>, single_context_mode: bool) -> bool {
    let mut io = imgui::get_io().unwrap();
    io.BackendRendererName = str!("imgui::impls::opengl3").as_ptr();
    unsafe {
        g.glsl_version = glsl_version.unwrap_or(GLSL_VERSION_130);
        g.single_context_mode = single_context_mode;
    }
    return true;
}

pub fn new_frame() {
    unsafe {
        if g.font_texture == 0 {
            create_device_objects();
        }
    }
}

pub fn shutdown() {
    unsafe {
        destroy_device_objects();
    }
}

pub fn render_draw_data(draw_data: Option<&mut ImDrawData>) {
    unsafe {
        render_draw_data_unsafe(draw_data);
    }
}

unsafe fn render_draw_data_unsafe(draw_data: Option<&mut ImDrawData>) {
    let draw_data = draw_data.expect("draw data passed to render_draw_data cannot be null.");
    let io = imgui::get_io().unwrap();

    // Avoid rendering when minimized, scale coordinates for retina displays (screen coordinates != framebuffer coordinates)
    let fb_width = (draw_data.DisplaySize.x * io.DisplayFramebufferScale.x) as i32;
    let fb_height = (draw_data.DisplaySize.y * io.DisplayFramebufferScale.y) as i32;
    if fb_width <= 0 || fb_height <= 0 {
        return;
    }
    draw_data.scale_clip_rects(io.DisplayFramebufferScale);

    // Backup GL state
    let mut last_active_texture: GLenum = 0;
    gl::GetIntegerv(gl::ACTIVE_TEXTURE, transmute::<_, *mut GLint>(&mut last_active_texture));
    gl::ActiveTexture(gl::TEXTURE0);
    let mut last_program: GLint = 0;
    gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut last_program);
    let mut last_texture: GLint = 0;
    gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut last_texture);

    let mut last_sampler: GLint = 0 ;
    if GL_SAMPLER_BINDING {
        gl::GetIntegerv(gl::SAMPLER_BINDING, &mut last_sampler);
    }

    let mut last_array_buffer: GLint = 0;
    gl::GetIntegerv(gl::ARRAY_BUFFER_BINDING, &mut last_array_buffer);
    let mut last_vertex_array: GLint = 0;
    gl::GetIntegerv(gl::VERTEX_ARRAY_BINDING, &mut last_vertex_array);

    let mut last_polygon_mode: [GLint; 2] = [0; 2];
    if GL_POLYGON_MODE {
        gl::GetIntegerv(gl::POLYGON_MODE, &mut last_polygon_mode[0]);
    }

    let mut last_viewport: [GLint; 4] = [0; 4];
    gl::GetIntegerv(gl::VIEWPORT, &mut last_viewport[0]);
    let mut last_scissor_box: [GLint; 4] = [0; 4];
    gl::GetIntegerv(gl::SCISSOR_BOX, &mut last_scissor_box[0]);
    let mut last_blend_src_rgb: GLenum = 0;
    gl::GetIntegerv(gl::BLEND_SRC_RGB, transmute::<_, *mut GLint>(&mut last_blend_src_rgb));
    let mut last_blend_dst_rgb: GLenum = 0;
    gl::GetIntegerv(gl::BLEND_DST_RGB, transmute::<_, *mut GLint>(&mut last_blend_dst_rgb));
    let mut last_blend_src_alpha: GLenum = 0;
    gl::GetIntegerv(gl::BLEND_SRC_ALPHA, transmute::<_, *mut GLint>(&mut last_blend_src_alpha));
    let mut last_blend_dst_alpha: GLenum = 0;
    gl::GetIntegerv(gl::BLEND_DST_ALPHA, transmute::<_, *mut GLint>(&mut last_blend_dst_alpha));
    let mut last_blend_equation_rgb: GLenum = 0;
    gl::GetIntegerv(gl::BLEND_EQUATION_RGB, transmute::<_, *mut GLint>(&mut last_blend_equation_rgb));
    let mut last_blend_equation_alpha: GLenum = 0;
    gl::GetIntegerv(gl::BLEND_EQUATION_ALPHA, transmute::<_, *mut GLint>(&mut last_blend_equation_alpha));

    let last_enable_blend: GLboolean = gl::IsEnabled(gl::BLEND);
    let last_enable_cull_face: GLboolean = gl::IsEnabled(gl::CULL_FACE);
    let last_enable_depth_test: GLboolean = gl::IsEnabled(gl::DEPTH_TEST);
    let last_enable_scissor_test: GLboolean = gl::IsEnabled(gl::SCISSOR_TEST);

    let mut clip_origin_lower_left = true;
    if GL_CLIP_ORIGIN {
        let mut last_clip_origin: GLenum = 0;
        gl::GetIntegerv(gl::CLIP_ORIGIN, transmute::<_, *mut GLint>(&mut last_clip_origin));
        if last_clip_origin == gl::UPPER_LEFT {
            clip_origin_lower_left = false;
        }
    }

    // Setup render state: alpha-blending enabled, no face culling, no depth testing, scissor enabled, polygon fill
    gl::Enable(gl::BLEND);
    gl::BlendEquation(gl::FUNC_ADD);
    gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    gl::Disable(gl::CULL_FACE);
    gl::Disable(gl::DEPTH_TEST);
    gl::Enable(gl::SCISSOR_TEST);
    if GL_POLYGON_MODE {
        gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
    }

    // Setup viewport, orthographic projection matrix
    // Our visible imgui space lies from draw_data->DisplayPos (top left) to draw_data->DisplayPos+data_data->DisplaySize (bottom right). DisplayMin is typically (0,0) for single viewport apps.
    gl::Viewport(0, 0, fb_width as GLsizei, fb_height as GLsizei);
    let l = draw_data.DisplayPos.x;
    let r = draw_data.DisplayPos.x + draw_data.DisplaySize.x;
    let t = draw_data.DisplayPos.y;
    let b = draw_data.DisplayPos.y + draw_data.DisplaySize.y;

    let ortho_projection: [[f32; 4]; 4] = [
        [ 2.0/(r-l),    0.0,          0.0,   0.0 ],
        [ 0.0,          2.0/(t-b),    0.0,   0.0 ],
        [ 0.0,          0.0,         -1.0,   0.0 ],
        [ (r+l)/(l-r),  (t+b)/(b-t),  0.0,   1.0 ]
    ];
    gl::UseProgram(g.shader_handle);
    gl::Uniform1i(g.attrib_location_tex, 0);
    gl::UniformMatrix4fv(g.attrib_location_proj_mtx, 1, gl::FALSE, &ortho_projection[0][0]);
    if GL_SAMPLER_BINDING {
        gl::BindSampler(0, 0); // We use combined texture/sampler state. Applications using GL 3.3 may set that otherwise.
    }
    // Recreate the VAO every time
    // (This is to easily allow multiple GL contexts. VAO are not shared among GL contexts, and we don't track creation/deletion of windows so we don't have an obvious key to use to cache them.)
    let mut vao_handle: GLuint = 0;
    if g.single_context_mode {
        vao_handle = g.vao_handle;
    } else {
        gl::GenVertexArrays(1, &mut vao_handle);
    }
    gl::BindVertexArray(vao_handle);
    gl::BindBuffer(gl::ARRAY_BUFFER, g.vbo_handle);
    gl::EnableVertexAttribArray(g.attrib_location_position as _);
    gl::EnableVertexAttribArray(g.attrib_location_uv as _);
    gl::EnableVertexAttribArray(g.attrib_location_color as _);
    gl::VertexAttribPointer(g.attrib_location_position as _, 2, gl::FLOAT, gl::FALSE, size_of!(ImDrawVert) as _, transmute(offset_of!(ImDrawVert, pos)));
    gl::VertexAttribPointer(g.attrib_location_uv as _, 2, gl::FLOAT, gl::FALSE, size_of!(ImDrawVert) as _, transmute(offset_of!(ImDrawVert, uv)));
    gl::VertexAttribPointer(g.attrib_location_color as _, 4, gl::UNSIGNED_BYTE, gl::TRUE, size_of!(ImDrawVert) as _, transmute(offset_of!(ImDrawVert, col)));

    // Draw
    let pos: ImVec2 = draw_data.DisplayPos;
    for n in 0..draw_data.CmdListsCount {
        let cmd_list: *const ImDrawList = *draw_data.CmdLists.offset(n as isize);
        let mut idx_buffer_offset: *const ImDrawIdx = transmute::<_, *const ImDrawIdx>(0usize);

        gl::BindVertexArray(vao_handle); // <-- DO NOT REMOVE (not having this causes a rendering bug that took me days to solve)
        gl::BindBuffer(gl::ARRAY_BUFFER, g.vbo_handle);
        gl::BufferData(gl::ARRAY_BUFFER, (*cmd_list).VtxBuffer.Size as isize * size_of!(ImDrawVert) as isize, transmute((*cmd_list).VtxBuffer.Data), gl::STREAM_DRAW);

        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, g.elements_handle);
        gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, (*cmd_list).IdxBuffer.Size as isize * size_of!(ImDrawIdx) as isize, transmute((*cmd_list).IdxBuffer.Data), gl::STREAM_DRAW);

        for cmd_i in 0..(*cmd_list).CmdBuffer.Size {
            let pcmd: *const ImDrawCmd = (*cmd_list).CmdBuffer.Data.offset(cmd_i as isize);
            if let Some(cb) = (*pcmd).UserCallback {
                cb(cmd_list, pcmd);
            } else {
                let clip_rect = imgui::vec4((*pcmd).ClipRect.x - pos.x, (*pcmd).ClipRect.y - pos.y, (*pcmd).ClipRect.z - pos.x, (*pcmd).ClipRect.w - pos.y);
                if clip_rect.x < fb_width as f32 && clip_rect.y < fb_height as f32 && clip_rect.z >= 0.0 && clip_rect.w >= 0.0 {
                    // Apply scissor/clipping rectangle
                    if clip_origin_lower_left {
                        gl::Scissor(clip_rect.x as GLint, (fb_height as f32 - clip_rect.w) as GLint, (clip_rect.z - clip_rect.x) as GLsizei, (clip_rect.w - clip_rect.y) as GLsizei);
                    } else {
                        gl::Scissor(clip_rect.x as GLint, clip_rect.y as GLint, clip_rect.z as GLsizei, clip_rect.w as GLsizei); // Support for GL 4.5's glClipControl(GL_UPPER_LEFT)
                    }

                    // Bind texture, Draw
                    let _type = if size_of!(ImDrawIdx) == 2  {
                        gl::UNSIGNED_SHORT
                    } else {
                        gl::UNSIGNED_INT
                    };
                    gl::BindTexture(gl::TEXTURE_2D, (*pcmd).TextureId as GLuint);
                    gl::DrawElements(gl::TRIANGLES, (*pcmd).ElemCount as GLint, _type, transmute(idx_buffer_offset));
                }
            }
            idx_buffer_offset = idx_buffer_offset.offset((*pcmd).ElemCount as isize);
        }
    }

    if !g.single_context_mode {
        gl::DeleteVertexArrays(1, &vao_handle);
    }

    // Restore modified GL state
    gl::UseProgram(last_program as GLuint);
    gl::BindTexture(gl::TEXTURE_2D, last_texture as GLuint);
    if GL_SAMPLER_BINDING {
        gl::BindSampler(0, last_sampler as GLuint);
    }
    gl::ActiveTexture(last_active_texture);
    gl::BindVertexArray(last_vertex_array as GLuint);
    gl::BindBuffer(gl::ARRAY_BUFFER, last_array_buffer as GLuint);
    gl::BlendEquationSeparate(last_blend_equation_rgb, last_blend_equation_alpha);
    gl::BlendFuncSeparate(last_blend_src_rgb, last_blend_dst_rgb, last_blend_src_alpha, last_blend_dst_alpha);
    if last_enable_blend != 0 { gl::Enable(gl::BLEND); } else { gl::Disable(gl::BLEND); }
    if last_enable_cull_face != 0 { gl::Enable(gl::CULL_FACE); } { gl::Disable(gl::CULL_FACE); }
    if last_enable_depth_test != 0 { gl::Enable(gl::DEPTH_TEST); } else { gl::Disable(gl::DEPTH_TEST); }
    if last_enable_scissor_test != 0 { gl::Enable(gl::SCISSOR_TEST); } else { gl::Disable(gl::SCISSOR_TEST); }

    if GL_POLYGON_MODE {
        gl::PolygonMode(gl::FRONT_AND_BACK, last_polygon_mode[0] as GLenum);
    }

    gl::Viewport(last_viewport[0], last_viewport[1], last_viewport[2], last_viewport[3]);
    gl::Scissor(last_scissor_box[0], last_scissor_box[1], last_scissor_box[2], last_scissor_box[3]);
}

unsafe fn create_fonts_texture() -> bool {
    // Build texture atlas
    let io = imgui::get_io().unwrap();
    let mut pixels: *mut u8 = ptr::null_mut();
    let mut width: i32 = 0;
    let mut height: i32 = 0;
    (*io.Fonts).get_tex_data_as_rgba32(&mut pixels, &mut width, &mut height, None);

    // Upload texture to graphics system
    let mut last_texture: GLint = 0;
    gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut last_texture);
    gl::GenTextures(1, &mut g.font_texture);
    gl::BindTexture(gl::TEXTURE_2D, g.font_texture);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
    gl::PixelStorei(gl::UNPACK_ROW_LENGTH, 0);
    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as _, width, height, 0, gl::RGBA, gl::UNSIGNED_BYTE, transmute(pixels));

    // Store our identifier
    (*io.Fonts).TexID = transmute(g.font_texture as usize);

    // Restore state
    gl::BindTexture(gl::TEXTURE_2D, last_texture as GLuint);
    return true;
}

unsafe fn destroy_fonts_texture() {
    if g.font_texture != 0 {
        let io = imgui::get_io().unwrap();
        gl::DeleteTextures(1, &mut g.font_texture);
        (*io.Fonts).TexID = transmute(0usize);
        g.font_texture = 0;
    }
}

unsafe fn check_shader(handle: GLuint, desc: &str) -> bool {
    let mut status: GLint = 0;
    let mut log_length: GLint = 0;
    gl::GetShaderiv(handle, gl::COMPILE_STATUS, &mut status);
    gl::GetShaderiv(handle, gl::INFO_LOG_LENGTH, &mut log_length);
    if status as GLboolean == gl::FALSE {
        eprintln!("ERROR imgui::impls::opengl3::create_device_objects: failed to compile {}!", desc);

        if log_length > 0 {
            let mut buf: Vec<u8> = Vec::with_capacity(log_length as usize);
            buf.resize(log_length as usize, 0);
            gl::GetShaderInfoLog(handle, log_length, ptr::null_mut(), transmute(buf.as_mut_ptr()));
            let s = String::from_utf8_unchecked(buf);
            eprintln!("shader error: {}", s);
        }
    }
    return status as GLboolean == gl::TRUE;
}

unsafe fn check_program(handle: GLuint, desc: &str) -> bool {
    let mut status: GLint = 0;
    let mut log_length: GLint = 0;
    gl::GetProgramiv(handle, gl::LINK_STATUS, &mut status);
    gl::GetProgramiv(handle, gl::INFO_LOG_LENGTH, &mut log_length);
    if status as GLboolean == gl::FALSE {
        eprintln!("ERROR imgui::impls::opengl3::create_device_objects: failed to link {}!", desc);
        
        if log_length > 0 {
            let mut buf: Vec<u8> = Vec::with_capacity(log_length as usize);
            buf.resize(log_length as usize, 0);
            gl::GetProgramInfoLog(handle, log_length, ptr::null_mut(), transmute(buf.as_mut_ptr()));
            let s = String::from_utf8_unchecked(buf);
            eprintln!("shader error: {}", s);
        }
    }
    return status as GLboolean == gl::TRUE;
}

pub unsafe fn create_device_objects() -> bool {
    // Backup GL state
    let mut last_texture: GLint = 0;
    let mut last_array_buffer: GLint = 0;
    let mut last_vertex_array: GLint = 0;
    gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut last_texture);
    gl::GetIntegerv(gl::ARRAY_BUFFER_BINDING, &mut last_array_buffer);
    gl::GetIntegerv(gl::VERTEX_ARRAY_BINDING, &mut last_vertex_array);

    // Select shaders matching our GLSL versions
    let vertex_shader_nv: &[u8];
    let fragment_shader_nv: &[u8];
    let version_number: u32;
    match g.glsl_version {
        GLSL_VERSION_120 => {
            vertex_shader_nv = VERTEX_SHADER_GLSL_120;
            fragment_shader_nv = FRAGMENT_SHADER_GLSL_120;
            version_number = 120;
        },
        GLSL_VERSION_130 => {
            vertex_shader_nv = VERTEX_SHADER_GLSL_130;
            fragment_shader_nv = FRAGMENT_SHADER_GLSL_130;
            version_number = 130;
        },
        GLSL_VERSION_300_ES => {
            vertex_shader_nv = VERTEX_SHADER_GLSL_300_ES;
            fragment_shader_nv = FRAGMENT_SHADER_GLSL_300_ES;
            version_number = 300;
        },
        GLSL_VERSION_410_CORE => {
            vertex_shader_nv = VERTEX_SHADER_GLSL_410_CORE;
            fragment_shader_nv = FRAGMENT_SHADER_GLSL_410_CORE;
            version_number = 410;
        },
        GLSLVersion(v) if v < 130 && v > 0 => {
            vertex_shader_nv = VERTEX_SHADER_GLSL_120;
            fragment_shader_nv = FRAGMENT_SHADER_GLSL_120;
            version_number = v;
        },
        _ => {
            vertex_shader_nv = VERTEX_SHADER_GLSL_130;
            fragment_shader_nv = FRAGMENT_SHADER_GLSL_130;
            version_number = 130;
        }
    }

    // Create shaders
    let vertex_shader = format!("#version {}\n{}\0", version_number, std::str::from_utf8_unchecked(vertex_shader_nv)).into_bytes();
    let fragment_shader = format!("#version {}\n{}\0", version_number, std::str::from_utf8_unchecked(fragment_shader_nv)).into_bytes();

    g.vert_handle = gl::CreateShader(gl::VERTEX_SHADER);
    gl::ShaderSource(g.vert_handle, 1, transmute::<*const *const u8, _>(&vertex_shader.as_ptr()), ptr::null_mut());
    gl::CompileShader(g.vert_handle);
    check_shader(g.vert_handle, "vertex shader");

    g.frag_handle = gl::CreateShader(gl::FRAGMENT_SHADER);
    gl::ShaderSource(g.frag_handle, 1, transmute::<*const *const u8, _>(&fragment_shader.as_ptr()), ptr::null_mut());
    gl::CompileShader(g.frag_handle);
    check_shader(g.frag_handle, "fragment shader");

    g.shader_handle = gl::CreateProgram();
    gl::AttachShader(g.shader_handle, g.vert_handle);
    gl::AttachShader(g.shader_handle, g.frag_handle);
    gl::LinkProgram(g.shader_handle);
    check_program(g.shader_handle, "shader program");

    g.attrib_location_tex = gl::GetUniformLocation(g.shader_handle, str!("Texture").as_ptr());
    g.attrib_location_proj_mtx = gl::GetUniformLocation(g.shader_handle, str!("ProjMtx").as_ptr());
    g.attrib_location_position = gl::GetAttribLocation(g.shader_handle, str!("Position").as_ptr());
    g.attrib_location_uv = gl::GetAttribLocation(g.shader_handle, str!("UV").as_ptr());
    g.attrib_location_color = gl::GetAttribLocation(g.shader_handle, str!("Color").as_ptr());

    // Create buffers
    gl::GenBuffers(1, &mut g.vbo_handle);
    gl::GenBuffers(1, &mut g.elements_handle);

    // Create a VAO in single context mode
    if g.single_context_mode {
        gl::GenVertexArrays(1, &mut g.vao_handle);
    }

    // Create font texture
    create_fonts_texture();

    // Restore modified GL state
    gl::BindTexture(gl::TEXTURE_2D, last_texture as GLuint);
    gl::BindBuffer(gl::ARRAY_BUFFER, last_array_buffer as GLuint);
    gl::BindVertexArray(last_vertex_array as GLuint);

    return true;
}

unsafe fn destroy_device_objects() {
    if g.single_context_mode && g.vao_handle != 0 {
        gl::DeleteVertexArrays(1, &mut g.vao_handle);
    }

    if g.vbo_handle != 0 {
        gl::DeleteBuffers(1, &mut g.vbo_handle);
        g.vbo_handle = 0;
    }
    if g.elements_handle != 0 {
        gl::DeleteBuffers(1, &mut g.elements_handle);
        g.elements_handle = 0;
    }

    if g.shader_handle != 0 && g.vert_handle != 0 {
        gl::DetachShader(g.shader_handle, g.vert_handle);
    }
    if g.vert_handle != 0{
        gl::DeleteShader(g.vert_handle);
        g.vert_handle = 0;
    }

    if g.shader_handle != 0 && g.frag_handle != 0 {
        gl::DetachShader(g.shader_handle, g.frag_handle);
    }
    if g.frag_handle != 0 {
        gl::DeleteShader(g.frag_handle);
        g.frag_handle = 0;
    }

    if g.shader_handle != 0{
        gl::DeleteProgram(g.shader_handle);
    }

    destroy_fonts_texture();
}


static VERTEX_SHADER_GLSL_120: &[u8] = b"\
uniform mat4 ProjMtx;
attribute vec2 Position;
attribute vec2 UV;
attribute vec4 Color;
varying vec2 Frag_UV;
varying vec4 Frag_Color;
void main()
{
    Frag_UV = UV;
    Frag_Color = Color;
    gl_Position = ProjMtx * vec4(Position.xy,0,1);
}
";

static VERTEX_SHADER_GLSL_130: &[u8] = b"\
uniform mat4 ProjMtx;
in vec2 Position;
in vec2 UV;
in vec4 Color;
out vec2 Frag_UV;
out vec4 Frag_Color;
void main()
{
    Frag_UV = UV;
    Frag_Color = Color;
    gl_Position = ProjMtx * vec4(Position.xy,0,1);
}
";

static VERTEX_SHADER_GLSL_300_ES: &[u8] = b"\
precision mediump float;
layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 UV;
layout (location = 2) in vec4 Color;
uniform mat4 ProjMtx;
out vec2 Frag_UV;
out vec4 Frag_Color;
void main()
{
    Frag_UV = UV;
    Frag_Color = Color;
    gl_Position = ProjMtx * vec4(Position.xy,0,1);
}
";

static VERTEX_SHADER_GLSL_410_CORE: &[u8] = b"\
layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 UV;
layout (location = 2) in vec4 Color;
uniform mat4 ProjMtx;
out vec2 Frag_UV;
out vec4 Frag_Color;
void main()
{
    Frag_UV = UV;
    Frag_Color = Color;
    gl_Position = ProjMtx * vec4(Position.xy,0,1);
}
";

static FRAGMENT_SHADER_GLSL_120: &[u8] = b"\
#ifdef GL_ES
    precision mediump float;
#endif
uniform sampler2D Texture;
varying vec2 Frag_UV;
varying vec4 Frag_Color;
void main()
{
    gl_FragColor = Frag_Color * texture2D(Texture, Frag_UV.st);
}
";

static FRAGMENT_SHADER_GLSL_130: &[u8] = b"\
uniform sampler2D Texture;
in vec2 Frag_UV;
in vec4 Frag_Color;
out vec4 Out_Color;
void main()
{
    Out_Color = Frag_Color * texture(Texture, Frag_UV.st);
}
";

static FRAGMENT_SHADER_GLSL_300_ES: &[u8] = b"\
precision mediump float;
uniform sampler2D Texture;
in vec2 Frag_UV;
in vec4 Frag_Color;
layout (location = 0) out vec4 Out_Color;
void main()
{
    Out_Color = Frag_Color * texture(Texture, Frag_UV.st);
}
";

static FRAGMENT_SHADER_GLSL_410_CORE: &[u8] = b"\
in vec2 Frag_UV;
in vec4 Frag_Color;
uniform sampler2D Texture;
layout (location = 0) out vec4 Out_Color;
void main()
{
    Out_Color = Frag_Color * texture(Texture, Frag_UV.st);
}
";
