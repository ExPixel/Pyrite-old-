pub fn draw_text_bg_no_obj_window(bg: &TextBG, vram: &[u8], dest: &[u16]) {
}

pub fn draw_text_bg_with_obj_window(bg: &TextBG, vram: &[u8], dest: &[u16]) {
}

pub struct TextBG {
    /// Base address of characters.
    char_base: u32,
    /// Base address for screens.
    screen_base: u32,
    /// The first pixel to start rendering (starting from the left of the screen).
    left: u32,
    /// The last pixel to render.
    right: u32,
    /// The width of the background.
    width: u32,
    /// The height of the background.
    height: u32,
}
