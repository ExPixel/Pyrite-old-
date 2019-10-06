///! A thin opengl wrapper. You'll probably still have to use raw opengl calls most of the time but
///  this should at least handle the lifetime of the more common objects like buffers and textures.

use gl::types::{
    GLuint,
    GLint,
    GLboolean,
};
use std::{ mem, ptr };
use std::borrow::Cow;
use std::ffi::{ CStr, CString };

#[derive(Copy, Clone)]
pub enum BufferUsage {
    StreamDraw,
    StaticDraw,
    DynamicDraw,

    StreamRead,
    StaticRead,
    DynamicRead,

    StreamCopy,
    StaticCopy,
    DynamicCopy,
}

impl BufferUsage {
    fn as_gl(self) -> GLuint {
        match self {
            BufferUsage::StreamDraw => gl::STREAM_DRAW,
            BufferUsage::StaticDraw => gl::STATIC_DRAW,
            BufferUsage::DynamicDraw => gl::DYNAMIC_DRAW,

            BufferUsage::StreamRead => gl::STREAM_READ,
            BufferUsage::StaticRead => gl::STATIC_READ,
            BufferUsage::DynamicRead => gl::DYNAMIC_READ,

            BufferUsage::StreamCopy => gl::STREAM_COPY,
            BufferUsage::StaticCopy => gl::STATIC_COPY,
            BufferUsage::DynamicCopy => gl::DYNAMIC_COPY,
        }
    }
}

#[derive(Copy, Clone)]
pub enum BufferType {
    ArrayBuffer,
    ElementArrayBuffer,
}

impl BufferType {
    fn as_gl(self) -> GLuint {
        match self {
            BufferType::ArrayBuffer => gl::ARRAY_BUFFER,
            BufferType::ElementArrayBuffer => gl::ELEMENT_ARRAY_BUFFER,
        }
    }
}

pub struct Buffer(GLuint, BufferType);

impl Buffer {
    pub fn new(buffer_type: BufferType) -> Buffer {
        let mut buffer_id: GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut buffer_id);
        }
        Buffer(buffer_id, buffer_type)
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(self.1.as_gl(), self.0);
        }
    }

    #[inline]
    pub fn set_data<DataType: BufferDataType>(&self, data: &[DataType], usage: BufferUsage) {
        unsafe {
            let data_type_size = mem::size_of::<DataType>();
            let buffer_size = data.len() * data_type_size;
            gl::BufferData(self.1.as_gl(), buffer_size as isize, mem::transmute(data.as_ptr()), usage.as_gl());
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.0);
        }
    }
}

pub struct VertexArray(GLuint);

impl VertexArray {
    pub fn new() -> VertexArray {
        let mut vertex_array_id: GLuint = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut vertex_array_id);
        }
        VertexArray(vertex_array_id)
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.0);
        }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.0);
        }
    }
}

#[derive(Copy, Clone)]
pub enum ShaderType {
    Vertex,
    Fragment,
}

impl ShaderType {
    fn as_gl(self) -> GLuint {
        match self {
            ShaderType::Vertex => gl::VERTEX_SHADER,
            ShaderType::Fragment => gl::FRAGMENT_SHADER,
        }
    }
}

pub struct Shader(GLuint, ShaderType);

impl Shader {
    pub fn compile(shader_type: ShaderType, source: &str) -> Result<Shader, String> {
        let zsource = as_zero_str(source);
        unsafe {
            let handle = gl::CreateShader(shader_type.as_gl());
            gl::ShaderSource(handle, 1, &zsource.as_ptr(), ptr::null_mut());
            gl::CompileShader(handle);

            if Self::is_compile_success(handle) {
                Ok(Shader(handle, shader_type))
            } else {
                Err(Self::get_error(handle))
            }
        }
    }

    fn is_compile_success(handle: GLuint) -> bool {
        let mut status: GLint = 0;
        unsafe {
            gl::GetShaderiv(handle, gl::COMPILE_STATUS, &mut status);
        }
        return (status as GLboolean) != gl::FALSE;
    }

    fn get_error(handle: GLuint) -> String {
        let mut log_length: GLint = 0;
        unsafe {
            gl::GetShaderiv(handle, gl::INFO_LOG_LENGTH, &mut log_length);
        }
        if log_length > 0 {
            let mut buf: Vec<u8> = Vec::with_capacity(log_length as usize);
            buf.resize(log_length as usize, 0);
            unsafe {
                gl::GetShaderInfoLog(handle, log_length, ptr::null_mut(), mem::transmute(buf.as_mut_ptr()));
                String::from_utf8_unchecked(buf)
            }
        } else {
            String::new()
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.0);
        }
    }
}

pub struct Program(GLuint);

impl Program {
    pub fn link(shaders: &[&Shader]) -> Result<Program, String> {
        unsafe {
            let handle = gl::CreateProgram();
            for shader in shaders.iter() {
                gl::AttachShader(handle, shader.0);
            }
            gl::LinkProgram(handle);

            if Self::is_link_success(handle) {
                Ok(Program(handle))
            } else {
                Err(Self::get_error(handle))
            }
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::UseProgram(self.0);
        }
    }

    pub fn attrib_location(&self, attrib: &str) -> GLint {
        let zattrib = as_zero_str(attrib);
        unsafe {
            gl::GetAttribLocation(self.0, zattrib.as_ptr())
        }
    }

    pub fn uniform_location(&self, uniform: &str) -> GLint {
        let zuniform = as_zero_str(uniform);
        unsafe {
            gl::GetUniformLocation(self.0, zuniform.as_ptr())
        }
    }

    fn is_link_success(handle: GLuint) -> bool {
        let mut status: GLint = 0;
        unsafe {
            gl::GetProgramiv(handle, gl::LINK_STATUS, &mut status);
        }
        return (status as GLboolean) != gl::FALSE;
    }

    fn get_error(handle: GLuint) -> String {
        let mut log_length: GLint = 0;
        unsafe {
            gl::GetProgramiv(handle, gl::INFO_LOG_LENGTH, &mut log_length);
        }
        if log_length > 0 {
            let mut buf: Vec<u8> = Vec::with_capacity(log_length as usize);
            buf.resize(log_length as usize, 0);
            unsafe {
                gl::GetProgramInfoLog(handle, log_length, ptr::null_mut(), mem::transmute(buf.as_mut_ptr()));
                String::from_utf8_unchecked(buf)
            }
        } else {
            String::new()
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.0)
        }
    }
}

#[derive(Copy, Clone)]
pub enum InternalPixelFormat {
    Red,
    RG,
    RGB,
    RGBA,
}

impl InternalPixelFormat {
    fn as_gl(self) -> GLuint {
        match self {
            InternalPixelFormat::Red => gl::RED,
            InternalPixelFormat::RG => gl::RG,
            InternalPixelFormat::RGB => gl::RGB,
            InternalPixelFormat::RGBA => gl::RGBA,
        }
    }
}

#[derive(Copy, Clone)]
pub enum PixelDataFormat {
    Red,
    RG,
    RGB,
    BGR,
    RGBA,
    BGRA
}

impl PixelDataFormat {
    fn as_gl(self) -> GLuint {
        match self {
            PixelDataFormat::Red => gl::RED,
            PixelDataFormat::RG => gl::RG,
            PixelDataFormat::RGB => gl::RGB,
            PixelDataFormat::BGR => gl::BGR,
            PixelDataFormat::RGBA => gl::RGBA,
            PixelDataFormat::BGRA => gl::BGRA,
        }
    }
}

#[derive(Copy, Clone)]
pub enum PixelDataType {
    UnsignedByte,
    Byte,
    UnsignedShort,
    Short,
    UnsignedInt,
    Int,
    Float,
    #[allow(non_camel_case_types)]
    UnsignedByte_3_3_2,
    #[allow(non_camel_case_types)]
    UnsignedByte_2_3_3_Rev,
    #[allow(non_camel_case_types)]
    UnsignedShort_5_6_5,
    #[allow(non_camel_case_types)]
    UnsignedShort_5_6_5_Rev,
    #[allow(non_camel_case_types)]
    UnsignedShort_4_4_4_4,
    #[allow(non_camel_case_types)]
    UnsignedShort_4_4_4_4_Rev,
    #[allow(non_camel_case_types)]
    UnsignedShort_5_5_5_1,
    #[allow(non_camel_case_types)]
    UnsignedShort_1_5_5_5_Rev,
    #[allow(non_camel_case_types)]
    UnsignedInt_8_8_8_8,
    #[allow(non_camel_case_types)]
    UnsignedInt_8_8_8_8_Rev,
    #[allow(non_camel_case_types)]
    UnsignedInt_10_10_10_2,
    #[allow(non_camel_case_types)]
    UnsignedInt_2_10_10_10_Rev,
}

impl PixelDataType {
    fn as_gl(self) -> GLuint {
        match self {
            PixelDataType::UnsignedByte => gl::UNSIGNED_BYTE,
            PixelDataType::Byte => gl::BYTE,
            PixelDataType::UnsignedShort => gl::UNSIGNED_SHORT,
            PixelDataType::Short => gl::SHORT,
            PixelDataType::UnsignedInt => gl::UNSIGNED_INT,
            PixelDataType::Int => gl::INT,
            PixelDataType::Float => gl::FLOAT,
            PixelDataType::UnsignedByte_3_3_2 => gl::UNSIGNED_BYTE_3_3_2,
            PixelDataType::UnsignedByte_2_3_3_Rev => gl::UNSIGNED_BYTE_2_3_3_REV,
            PixelDataType::UnsignedShort_5_6_5 => gl::UNSIGNED_SHORT_5_6_5,
            PixelDataType::UnsignedShort_5_6_5_Rev => gl::UNSIGNED_SHORT_5_6_5_REV,
            PixelDataType::UnsignedShort_4_4_4_4 => gl::UNSIGNED_SHORT_4_4_4_4,
            PixelDataType::UnsignedShort_4_4_4_4_Rev => gl::UNSIGNED_SHORT_4_4_4_4_REV,
            PixelDataType::UnsignedShort_5_5_5_1 => gl::UNSIGNED_SHORT_5_5_5_1,
            PixelDataType::UnsignedShort_1_5_5_5_Rev => gl::UNSIGNED_SHORT_1_5_5_5_REV,
            PixelDataType::UnsignedInt_8_8_8_8 => gl::UNSIGNED_INT_8_8_8_8,
            PixelDataType::UnsignedInt_8_8_8_8_Rev => gl::UNSIGNED_INT_8_8_8_8_REV,
            PixelDataType::UnsignedInt_10_10_10_2 => gl::UNSIGNED_INT_10_10_10_2,
            PixelDataType::UnsignedInt_2_10_10_10_Rev => gl::UNSIGNED_INT_2_10_10_10_REV,
        }
    }
}

pub struct Texture {
    handle: GLuint,
    width:  u32,
    height: u32,
}

impl Texture {
    pub fn new<PData: PixelData>(width: u32, height: u32, internal_format: InternalPixelFormat,  pixel_data_format: PixelDataFormat, pixel_data_type: PixelDataType, pixel_data: Option<PData>) -> Texture {
        let mut handle: GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut handle);
            gl::BindTexture(gl::TEXTURE_2D, handle);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);

            if let Some(data) = pixel_data {
                let pixel_data_ptr = data.get_data_ptr();
                gl::TexImage2D(gl::TEXTURE_2D, 0, internal_format.as_gl() as _, width as _, height as _, 0, pixel_data_format.as_gl(), pixel_data_type.as_gl(), mem::transmute(pixel_data_ptr));
            } else {
                gl::TexImage2D(gl::TEXTURE_2D, 0, internal_format.as_gl() as _, width as _, height as _, 0, pixel_data_format.as_gl(), pixel_data_type.as_gl(), ptr::null());
            }
        }

        Texture {
            handle,
            width,
            height
        }
    }

    pub fn get_handle(&self) -> GLuint {
        self.handle
    }

    pub fn get_width(&mut self) -> u32 {
        self.width
    }

    pub fn get_height(&mut self) -> u32 {
        self.height
    }

    /// This calls `glTexImage2D` which will recreate all of the internal data structures which can
    /// be quite slow. If you just want to update the pixel data, use `set_pixels` instead which
    /// will use `glTexSubImage2D`.
    pub fn set_data<PData: PixelData>(&mut self, width: u32, height: u32, internal_format: InternalPixelFormat, pixel_data_format: PixelDataFormat, pixel_data_type: PixelDataType, pixel_data: PData) {
        self.width = width;
        self.height = height;

        let pixel_data_ptr = pixel_data.get_data_ptr();
        unsafe {
            gl::TexImage2D(gl::TEXTURE_2D, 0, internal_format.as_gl() as _, width as _, height as _, 0, pixel_data_format.as_gl(), pixel_data_type.as_gl(), mem::transmute(pixel_data_ptr));
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.handle as _);
        }
    }

    pub fn set_pixels<PData: PixelData>(&self, xoffset: u32, yoffset: u32, width: u32, height: u32, pixel_data_format: PixelDataFormat, pixel_data_type: PixelDataType, pixel_data: PData) {
        debug_assert!(xoffset + width <= self.width, "out of bounds 'xoffset + width'");
        debug_assert!(yoffset + height <= self.height, "out of bounds 'yoffset + height'");

        let pixel_data_ptr= pixel_data.get_data_ptr();
        unsafe {
            gl::TexSubImage2D(gl::TEXTURE_2D, 0, xoffset as _, yoffset as _, width as _, height as _, pixel_data_format.as_gl(), pixel_data_type.as_gl(), mem::transmute(pixel_data_ptr));
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &mut self.handle);
        }
    }
}

pub trait PixelData {
    fn get_data_ptr(&self) -> *mut u8;
}

impl PixelData for &[u32] {
    fn get_data_ptr(&self) -> *mut u8 {
        unsafe {
            mem::transmute(self.as_ptr())
        }
    }
}

impl PixelData for &[u16] {
    fn get_data_ptr(&self) -> *mut u8 {
        unsafe {
            mem::transmute(self.as_ptr())
        }
    }
}

impl PixelData for &[u8] {
    fn get_data_ptr(&self) -> *mut u8 {
        unsafe {
            mem::transmute(self.as_ptr())
        }
    }
}

pub const FULLSCREEN_BUFFER_DATA: [f32; 24] = [
    /* Position */  /* UV */
    -1.0, -1.0,      0.0,  1.0, // bottom left
    -1.0,  1.0,      0.0,  0.0, // top left
     1.0, -1.0,      1.0,  1.0, // bottom right

     1.0, -1.0,      1.0,  1.0, // bottom left
     1.0,  1.0,      1.0,  0.0, // top right
    -1.0,  1.0,      0.0,  0.0, // top left
];

pub const SIMPLE_VERTEX_SHADER: &str   = "\
#version 130

in  vec2 Position;
in  vec2 UV;
out vec2 FragUV;

void main() {
    FragUV = UV;
    gl_Position = vec4(Position.xy, 0, 1);
}\0";

pub const SIMPLE_FRAGMENT_SHADER: &str = "\
#version 130

uniform sampler2D Texture;
in  vec2 FragUV;
out vec4 OutColor;

void main() {
    OutColor = texture(Texture, FragUV.st);
}\0";

/// A marker trait for pieces of data that can be used for a buffer.
pub trait BufferDataType {}
impl BufferDataType for i8 {}
impl BufferDataType for u8 {}
impl BufferDataType for i16 {}
impl BufferDataType for u16 {}
impl BufferDataType for i32 {}
impl BufferDataType for u32 {}
impl BufferDataType for i64 {}
impl BufferDataType for u64 {}
impl BufferDataType for f32 {}
impl BufferDataType for f64 {}

fn as_zero_str<'s>(string: &'s str) -> Cow<'s, CStr> {
    if string.ends_with('\0') {
        // if it's zero terminated there's nothing to do.
        Cow::from(
            unsafe {
                CStr::from_bytes_with_nul_unchecked(string.as_bytes())
            }
        )
    } else {
        Cow::from(
            CString::new(string).expect("Failed to create zero-terminated string in CString::new.")
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub enum GLErrorType {
    NoError,
    InvalidEnum,
    InvalidValue,
    InvalidOperation,
    InvalidFrameBufferOperation,
    OutOfMemory,
    Unknown,
}

impl GLErrorType {
    fn from_gl(e: gl::types::GLenum) -> GLErrorType {
        match e {
            gl::NO_ERROR => GLErrorType::NoError,
            gl::INVALID_ENUM => GLErrorType::InvalidEnum,
            gl::INVALID_VALUE => GLErrorType::InvalidValue,
            gl::INVALID_OPERATION => GLErrorType::InvalidOperation,
            gl::INVALID_FRAMEBUFFER_OPERATION => GLErrorType::InvalidFrameBufferOperation,
            gl::OUT_OF_MEMORY => GLErrorType::OutOfMemory,
            _ => GLErrorType::Unknown,
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            GLErrorType::NoError => "NO_ERROR",
            GLErrorType::InvalidEnum => "INVALID_ENUM",
            GLErrorType::InvalidValue => "INVALID_VALUE",
            GLErrorType::InvalidOperation => "INVALID_OPERATION",
            GLErrorType::InvalidFrameBufferOperation => "INVALID_FRAMEBUFFER_OPERATION",
            GLErrorType::OutOfMemory => "OUT_OF_MEMORY",
            GLErrorType::Unknown => "UNKNOWN",
        }
    }
}

impl std::fmt::Display for GLErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

/// Returns true if an error occurred.
pub fn check_gl_errors<F: Fn(GLErrorType)>(on_error: F) -> bool {
    let mut error_occurred = false;
    let mut err = unsafe { gl::GetError() };
    while err != gl::NO_ERROR {
        on_error(GLErrorType::from_gl(err));
        error_occurred = true;
        err = unsafe { gl::GetError() };
    }
    error_occurred
}
