use pyrite_gba::Gba;

pub struct GbaImGui {
    video:  crate::platform::opengl::PyriteGL,
    gba:    Gba,
}

impl GbaImGui {
    pub fn new(gba: Gba, window: &glutin::Window) -> GbaImGui {
        let mut ret = GbaImGui {
            video:  crate::platform::opengl::PyriteGL::new(),
            gba:    gba,
        };
        ret.init(window);
        ret
    }

    fn init(&mut self, window: &glutin::Window) {
        imgui::create_context(None);
        imgui::style_colors_dark(None);
        imgui::impls::opengl3::init(
            Some(imgui::impls::opengl3::GLSL_VERSION_120),
            true, // single context mode
        );

        let (window_width, window_height) = if let Some(size) = window.get_inner_size() {
            (size.width as f32, size.height as f32)
        } else {
            (0.0f32, 0.0f32)
        };
        let dpi_factor = window.get_hidpi_factor() as f32;
        unsafe {
            gl::Viewport(0, 0, (window_width * dpi_factor) as i32, (window_height * dpi_factor) as i32);
        }
        imgui::impls::glutin::init(
            imgui::vec2(window_width, window_height),
            dpi_factor
        );
    }

    fn dispose(&mut self) {
        imgui::impls::opengl3::shutdown();
        imgui::impls::glutin::shutdown();
        imgui::destroy_context(None);
    }

    pub fn handle_event(&mut self, window: &glutin::Window, event: &glutin::Event) {
        use glutin::VirtualKeyCode;
        use pyrite_gba::KeypadInput;

        imgui::impls::glutin::process_window_event(window, event);

        let window_event;
        match event {
            &glutin::Event::WindowEvent { ref event, .. } => {
                window_event = event;
            }, 
            _ => { return },
        }

        match window_event {
            glutin::WindowEvent::KeyboardInput { input, .. } => {
                let pressed = match input.state {
                    glutin::ElementState::Pressed => true,
                    glutin::ElementState::Released => false,
                };

                match input.virtual_keycode {
                    Some(VirtualKeyCode::Left) => self.gba.set_key_pressed(KeypadInput::Left, pressed),
                    Some(VirtualKeyCode::Right) => self.gba.set_key_pressed(KeypadInput::Right, pressed),
                    Some(VirtualKeyCode::Up) => self.gba.set_key_pressed(KeypadInput::Up, pressed),
                    Some(VirtualKeyCode::Down) => self.gba.set_key_pressed(KeypadInput::Down, pressed),

                    Some(VirtualKeyCode::Return) => self.gba.set_key_pressed(KeypadInput::Start, pressed),
                    Some(VirtualKeyCode::Back) => self.gba.set_key_pressed(KeypadInput::Select, pressed),

                    Some(VirtualKeyCode::Z) => self.gba.set_key_pressed(KeypadInput::ButtonA, pressed),
                    Some(VirtualKeyCode::X) => self.gba.set_key_pressed(KeypadInput::ButtonB, pressed),

                    Some(VirtualKeyCode::A) => self.gba.set_key_pressed(KeypadInput::ButtonL, pressed),
                    Some(VirtualKeyCode::S) => self.gba.set_key_pressed(KeypadInput::ButtonR, pressed),
                    _ => { /* NOP */ },
                }
            },

            glutin::WindowEvent::Resized(logical_size) => {
                let dpi_factor = window.get_hidpi_factor();
                unsafe {
                    gl::Viewport(0, 0, (logical_size.width * dpi_factor) as i32, (logical_size.height * dpi_factor) as i32);
                }
            },

            _ => { /* NOP */ },
        }
    }

    fn render_gba_frame(&mut self) {
        let mut no_audio = pyrite_gba::NoAudioOutput;
        loop {
            self.gba.step(&mut self.video, &mut no_audio);
            if self.gba.is_frame_ready() { break }
        }
        self.video.render();
    }

    pub fn render_frame(&mut self, window: &glutin::Window) {
        // clear the screen
        unsafe {
            gl::ClearColor((0xC4 as f32)/255.0, (0x3D as f32)/255.0, (0x5F as f32)/255.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        self.render_gba_frame();

        // initialize imgui frame
        imgui::impls::opengl3::new_frame();
        imgui::impls::glutin::new_frame(window);
        imgui::new_frame();

        // Send ImGui commands and build the current frame here:
        // self.ui.render(&mut self.gba);
        if imgui::begin(imgui::str!("Test"), &mut true, imgui::none()) {
            imgui::end();
        }

        // Render ImGui
        imgui::render();

        imgui::impls::opengl3::render_draw_data(imgui::get_draw_data());
    }
}

impl Drop for GbaImGui {
    fn drop(&mut self) {
        self.dispose();
    }
}
