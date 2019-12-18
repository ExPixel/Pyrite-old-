use super::globj::{InternalPixelFormat, PixelDataFormat, PixelDataType, Texture};
use pyrite_gba::GbaVideoOutput;

pub struct GbaTexture {
    texture: Texture,
    data: Box<[u16; 240 * 160]>,
}

impl GbaTexture {
    pub fn new() -> GbaTexture {
        let mut x = GbaTexture {
            texture: Texture::new::<&[u8]>(
                240,
                160,
                InternalPixelFormat::RGBA,
                PixelDataFormat::BGRA,
                PixelDataType::UnsignedShort_1_5_5_5_Rev,
                None,
            ),
            data: Box::new([0xFFFF; 240 * 160]),
        };
        x.build_texture();
        return x;
    }

    pub fn get_texture_handle(&self) -> gl::types::GLuint {
        self.texture.get_handle()
    }

    pub fn build_texture(&mut self) {
        self.texture.bind();
        unsafe {
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 2);
            self.texture.set_pixels::<&[u8]>(
                0,
                0,
                240,
                160,
                PixelDataFormat::RGBA,
                PixelDataType::UnsignedShort_1_5_5_5_Rev,
                std::mem::transmute(&self.data[0..]),
            );
        }
    }
}

impl GbaVideoOutput for GbaTexture {
    fn display_line(&mut self, line: u32, pixels: &[u16]) {
        let offset = (line as usize) * 240;
        (&mut self.data[offset..(offset + 240)]).copy_from_slice(&pixels);
    }

    fn pre_frame(&mut self) {
        /* NOP */
    }

    fn post_frame(&mut self) {
        /* NOP */
    }
}
