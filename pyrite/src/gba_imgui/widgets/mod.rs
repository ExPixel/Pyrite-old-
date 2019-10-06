use crate::platform::opengl::GbaTexture;

pub struct EmulatorDisplayWidget {
    pub open: bool,
    focused: bool,
    pub scale: f32,
}

impl EmulatorDisplayWidget {
    pub fn new() -> EmulatorDisplayWidget {
        EmulatorDisplayWidget {
            open: true,
            focused: false,
            scale: 2.0f32,
        }
    }

    pub fn draw(&mut self, texture: &GbaTexture) {
        let content_size = imgui::vec2(240.0 * self.scale, 160.0 * self.scale);
        imgui::set_next_window_content_size(content_size);
        imgui::push_style_var_vec2(imgui::StyleVar::WindowPadding, imgui::vec2(0.0, 0.0));
        imgui::push_style_var_float(imgui::StyleVar::WindowRounding, 0.0);

        if imgui::begin(imgui::str!("Emulator Display"), &mut self.open, imgui::WindowFlags::AlwaysAutoResize | imgui::WindowFlags::NoScrollbar) {
            let texture_id: imgui::ImTextureID = texture.get_texture_handle() as _;
            imgui::image(texture_id, content_size);
            self.focused = imgui::is_window_focused(imgui::FocusedFlags::ChildWindows);
        }

        imgui::pop_style_var(2);
        imgui::end();
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }
}
