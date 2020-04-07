mod widgets;
use crate::debugger::{GbaDebugger, GbaStepSize};
use crate::platform::audio::PlatformAudio;
use crate::platform::opengl::GbaTexture;
use pyrite_gba::Gba;

// The frame rate of the GBA.
// Right now 60FPS.
pub const GBA_FRAMERATE_LIMIT: std::time::Duration = std::time::Duration::from_micros(16600);

pub struct GbaImGui {
    gba: Box<Gba>,
    audio: PlatformAudio,
    main_emulator_gui: widgets::EmulatorGUI,
    gba_debugger: GbaDebugger,
    time_of_last_gba_frame: std::time::Instant,
    close_requested: bool,
}

impl GbaImGui {
    pub fn new(gba: Box<Gba>) -> GbaImGui {
        GbaImGui {
            gba: gba,
            audio: PlatformAudio::new(),
            main_emulator_gui: widgets::EmulatorGUI::new(),
            gba_debugger: GbaDebugger::new(),
            time_of_last_gba_frame: std::time::Instant::now() - GBA_FRAMERATE_LIMIT,
            close_requested: false,
        }
    }

    pub fn run(mut self) {
        let event_loop = glutin::event_loop::EventLoop::<()>::new();
        let wb = glutin::window::WindowBuilder::new()
            .with_title("Pyrite")
            .with_inner_size(glutin::dpi::LogicalSize::new(240.0 * 3.0, 160.0 * 3.0));
        let windowed_context = glutin::ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(wb, &event_loop)
            .unwrap();
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };
        gl::load_with(|symbol| windowed_context.get_proc_address(symbol) as *const _);

        log::debug!(
            "windowed context pixel format: {:?}",
            windowed_context.get_pixel_format()
        );
        windowed_context.swap_buffers().unwrap();

        let mut gba_texture = GbaTexture::new();
        self.init(windowed_context.window());
        let mut pyrite_wait_for_swap = false;

        self.audio.init();
        event_loop.run(move |event, _, control_flow| {
            *control_flow = glutin::event_loop::ControlFlow::Poll;

            match event {
                glutin::event::Event::DeviceEvent { ref event, .. } => {
                    imgui::impls::glutin::process_device_event(windowed_context.window(), event);
                }
                glutin::event::Event::WindowEvent { ref event, .. } => {
                    imgui::impls::glutin::process_window_event(windowed_context.window(), event);
                    self.handle_window_event(event);

                    if let &glutin::event::WindowEvent::CloseRequested = event {
                        self.close_requested = true;
                    }
                }

                // @TODO render here if possible somehow:
                glutin::event::Event::RedrawRequested(_) => {
                    windowed_context.swap_buffers().unwrap();
                    pyrite_wait_for_swap = false;
                }
                _ => {}
            }

            if self.close_requested {
                *control_flow = glutin::event_loop::ControlFlow::Exit;
            } else if !pyrite_wait_for_swap {
                pyrite_wait_for_swap = true;
                self.update_frame(windowed_context.window());
                self.render_frame(windowed_context.window(), &mut gba_texture);
                windowed_context.window().request_redraw();
            }
        });
    }

    fn init(&mut self, window: &glutin::window::Window) {
        imgui::create_context(None);
        imgui::style_colors_dark(None);
        imgui::impls::opengl3::init(
            Some(imgui::impls::opengl3::GLSL_VERSION_120),
            true, // single context mode
        );

        let window_size = window.inner_size();
        let window_scale_factor = window.scale_factor();
        unsafe {
            gl::Viewport(
                0,
                0,
                (window_size.width as f64 * window_scale_factor) as i32,
                (window_size.height as f64 * window_scale_factor) as i32,
            );
        }
        imgui::impls::glutin::init(
            imgui::vec2(window_size.width as f32, window_size.height as f32),
            window_scale_factor as f32,
        );
    }

    fn dispose(&mut self) {
        imgui::impls::opengl3::shutdown();
        imgui::impls::glutin::shutdown();
        imgui::destroy_context(None);
    }

    fn handle_window_event(&mut self, window_event: &glutin::event::WindowEvent) {
        use glutin::event::VirtualKeyCode;
        use pyrite_gba::keypad::KeypadInput;

        match window_event {
            glutin::event::WindowEvent::KeyboardInput { input, .. } => {
                if self.main_emulator_gui.is_gba_display_focused() {
                    let pressed = match input.state {
                        glutin::event::ElementState::Pressed => true,
                        glutin::event::ElementState::Released => false,
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

            glutin::event::WindowEvent::Resized(physical_size) => unsafe {
                gl::Viewport(
                    0,
                    0,
                    physical_size.width as i32,
                    physical_size.height as i32,
                );
            },

            glutin::event::WindowEvent::ScaleFactorChanged { new_inner_size, .. } => unsafe {
                gl::Viewport(
                    0,
                    0,
                    new_inner_size.width as i32,
                    new_inner_size.height as i32,
                );
            },

            _ => { /* NOP */ }
        }
    }

    fn render_gba_frame(&mut self, gba_texture: &mut GbaTexture) {
        let mut no_audio = pyrite_gba::NoAudioOutput;

        if self.gba_debugger.debugging {
            match self.gba_debugger.pop_step_size() {
                Some(GbaStepSize::Instruction) => {
                    self.gba.step(gba_texture, &mut no_audio);
                }

                Some(GbaStepSize::VideoFrame) => {
                    self.gba.video_frame(gba_texture, &mut no_audio);
                }

                Some(GbaStepSize::VideoLine) => {
                    todo!();
                }

                None => {
                    if !self.gba_debugger.paused {
                        self.gba_debugger.step_gba_video_frame(
                            &mut self.gba,
                            gba_texture,
                            &mut no_audio,
                        );
                    }
                }
            }
        } else {
            self.gba.video_frame(gba_texture, &mut no_audio);
        }
    }

    pub fn update_frame(&mut self, _window: &glutin::window::Window) {
        use glutin::event::VirtualKeyCode;
        use imgui::impls::glutin::is_key_pressed;

        let io = imgui::get_io().unwrap();

        if is_key_pressed(VirtualKeyCode::Q) && io.KeyCtrl {
            self.close_requested = true;
        }
    }

    pub fn render_frame(&mut self, window: &glutin::window::Window, gba_texture: &mut GbaTexture) {
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

        let dur_since_last_gba_frame = frame_start_time.duration_since(self.time_of_last_gba_frame);
        let render_gba_frame = dur_since_last_gba_frame > GBA_FRAMERATE_LIMIT; // 16.6 ms
        if render_gba_frame {
            self.render_gba_frame(gba_texture);
            self.time_of_last_gba_frame = frame_start_time;
        }
        let frame_gba_end_time = std::time::Instant::now();

        gba_texture.build_texture();

        // initialize imgui frame
        imgui::impls::opengl3::new_frame();
        imgui::impls::glutin::new_frame_with_time(window, frame_start_time);
        imgui::new_frame();

        // Send ImGui commands and build the current frame here:
        self.render_gui(gba_texture);

        // Render ImGui
        imgui::render();

        imgui::impls::opengl3::render_draw_data(imgui::get_draw_data());
        let frame_gui_end_time = std::time::Instant::now();

        let gba_frame_delay = frame_gba_end_time.duration_since(frame_start_time);
        let gui_frame_delay = frame_gui_end_time.duration_since(frame_gba_end_time);

        // these will be used on the next frame:
        self.main_emulator_gui.set_gui_frame_delay(gui_frame_delay);
        if render_gba_frame {
            self.main_emulator_gui.set_gba_frame_delay(gba_frame_delay);
        }
    }

    fn render_gui(&mut self, gba_texture: &mut GbaTexture) {
        self.main_emulator_gui
            .draw(&mut self.gba, gba_texture, &mut self.gba_debugger);
    }
}

impl Drop for GbaImGui {
    fn drop(&mut self) {
        self.dispose();
    }
}
