use crate::platform::opengl::GbaTexture;

pub fn draw_emulator_display_widget(texture: &GbaTexture, open: &mut bool) {
    if imgui::begin(imgui::str!("Emulator Display"), open, imgui::none()) {
        let content_region_size = imgui::get_window_content_region_max();
        let texture_id: imgui::ImTextureID = texture.get_texture_handle() as _;
        imgui::image_with_size(texture_id, imgui::vec2(content_region_size.x, content_region_size.y));
    }
    imgui::end();
}
