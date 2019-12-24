mod widgets;
use crate::platform::opengl::GbaTexture;
use pyrite_gba::Gba;

pub struct GbaImGui {
    gba: Box<Gba>,
    main_emulator_gui: widgets::EmulatorGUI,
    gba_texture: GbaTexture,
}

impl GbaImGui {
    pub fn new(gba: Box<Gba>, window: &glutin::Window) -> GbaImGui {
        let mut ret = GbaImGui {
            gba: gba,
            main_emulator_gui: widgets::EmulatorGUI::new(),
            gba_texture: GbaTexture::new(),
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
            gl::Viewport(
                0,
                0,
                (window_width * dpi_factor) as i32,
                (window_height * dpi_factor) as i32,
            );
        }
        imgui::impls::glutin::init(imgui::vec2(window_width, window_height), dpi_factor);
    }

    fn dispose(&mut self) {
        imgui::impls::opengl3::shutdown();
        imgui::impls::glutin::shutdown();
        imgui::destroy_context(None);
    }

    pub fn handle_event(&mut self, window: &glutin::Window, event: &glutin::Event) {
        use glutin::VirtualKeyCode;
        use pyrite_gba::keypad::KeypadInput;

        imgui::impls::glutin::process_window_event(window, event);

        let window_event;
        match event {
            &glutin::Event::WindowEvent { ref event, .. } => {
                window_event = event;
            }
            _ => return,
        }

        match window_event {
            glutin::WindowEvent::KeyboardInput { input, .. } => {
                if self.main_emulator_gui.is_gba_display_focused() {
                    let pressed = match input.state {
                        glutin::ElementState::Pressed => true,
                        glutin::ElementState::Released => false,
                    };

                    match input.virtual_keycode {
                        Some(VirtualKeyCode::Left) => {
                            self.gba.set_key_pressed(KeypadInput::Left, pressed)
                        }
                        Some(VirtualKeyCode::Right) => {
                            self.gba.set_key_pressed(KeypadInput::Right, pressed)
                        }
                        Some(VirtualKeyCode::Up) => {
                            self.gba.set_key_pressed(KeypadInput::Up, pressed)
                        }
                        Some(VirtualKeyCode::Down) => {
                            self.gba.set_key_pressed(KeypadInput::Down, pressed)
                        }

                        Some(VirtualKeyCode::Return) => {
                            self.gba.set_key_pressed(KeypadInput::Start, pressed)
                        }
                        Some(VirtualKeyCode::Back) => {
                            self.gba.set_key_pressed(KeypadInput::Select, pressed)
                        }

                        Some(VirtualKeyCode::Z) => {
                            self.gba.set_key_pressed(KeypadInput::ButtonA, pressed)
                        }
                        Some(VirtualKeyCode::X) => {
                            self.gba.set_key_pressed(KeypadInput::ButtonB, pressed)
                        }

                        Some(VirtualKeyCode::A) => {
                            self.gba.set_key_pressed(KeypadInput::ButtonL, pressed)
                        }
                        Some(VirtualKeyCode::S) => {
                            self.gba.set_key_pressed(KeypadInput::ButtonR, pressed)
                        }

                        _ => { /* NOP */ }
                    }
                }
            }

            glutin::WindowEvent::Resized(logical_size) => {
                let dpi_factor = window.get_hidpi_factor();
                unsafe {
                    gl::Viewport(
                        0,
                        0,
                        (logical_size.width * dpi_factor) as i32,
                        (logical_size.height * dpi_factor) as i32,
                    );
                }
            }

            _ => { /* NOP */ }
        }
    }

    fn render_gba_frame(&mut self) {
        let mut no_audio = pyrite_gba::NoAudioOutput;
        self.gba.video_frame(&mut self.gba_texture, &mut no_audio);
    }

    pub fn render_frame(&mut self, window: &glutin::Window) {
        // @NOTE moved to the top because it takes a really long time on some machines (openGL
        // synchronization?) and I don't want to measure that because it basically takes as long as
        // a full frame to complete.
        // clear the screen
        unsafe {
            gl::ClearColor(
                (0xC4 as f32) / 255.0,
                (0x3D as f32) / 255.0,
                (0x5F as f32) / 255.0,
                1.0,
            );
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        let frame_start_time = std::time::Instant::now();
        self.render_gba_frame();
        let frame_gba_end_time = std::time::Instant::now();

        self.gba_texture.build_texture();

        // initialize imgui frame
        imgui::impls::opengl3::new_frame();
        imgui::impls::glutin::new_frame_with_time(window, frame_start_time);
        imgui::new_frame();

        // Send ImGui commands and build the current frame here:
        self.render_gui();

        // Render ImGui
        imgui::render();

        imgui::impls::opengl3::render_draw_data(imgui::get_draw_data());
        let frame_gui_end_time = std::time::Instant::now();

        let gba_frame_delay = frame_gba_end_time.duration_since(frame_start_time);
        let gui_frame_delay = frame_gui_end_time.duration_since(frame_gba_end_time);

        // these will be used on the next frame:
        self.main_emulator_gui.set_gui_frame_delay(gui_frame_delay);
        self.main_emulator_gui.set_gba_frame_delay(gba_frame_delay);
    }

    fn render_gui(&mut self) {
        self.main_emulator_gui
            .draw(&mut self.gba, &self.gba_texture);
    }
}

impl Drop for GbaImGui {
    fn drop(&mut self) {
        self.dispose();
    }
}
