use crate::platform::audio::PlatformAudio;
use crate::platform::opengl::PyriteGL;
use pyrite_gba::Gba;

// The frame rate of the GBA.
// Right now 60FPS.
pub const GBA_FRAMERATE_LIMIT: std::time::Duration = std::time::Duration::from_micros(16600);

pub struct PyriteGUI {
    gba: Box<Gba>,
    audio: PlatformAudio,
    close_requested: bool,
    modifier_shift: bool,
    modifier_ctrl: bool,

    title_buffer: String,
    gba_frame_counter: FrameCounter,
    gba_frame_timer: Timer,
    title_update_timer: Timer,
}

impl PyriteGUI {
    pub fn new(gba: Box<Gba>) -> PyriteGUI {
        PyriteGUI {
            gba: gba,
            audio: PlatformAudio::new(),
            close_requested: false,

            modifier_shift: false,
            modifier_ctrl: false,

            title_buffer: String::new(),
            gba_frame_counter: FrameCounter::new(),
            gba_frame_timer: Timer::new(GBA_FRAMERATE_LIMIT),
            title_update_timer: Timer::new(std::time::Duration::from_secs(1)),
        }
    }

    pub fn run(mut self) {
        let event_loop = glutin::event_loop::EventLoop::<()>::new();
        let wb = glutin::window::WindowBuilder::new()
            .with_visible(true)
            .with_title("Pyrite")
            .with_inner_size(glutin::dpi::LogicalSize::new(240.0 * 2.0, 160.0 * 2.0));
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

        // TODO
        let mut pyrite_gl = PyriteGL::new();
        self.init(windowed_context.window());
        let mut wait_for_redraw = false;

        self.audio.init();
        event_loop.run(move |event, _, control_flow| {
            *control_flow = glutin::event_loop::ControlFlow::Poll;

            self.update_timers();

            match event {
                glutin::event::Event::WindowEvent { ref event, .. } => {
                    self.handle_window_event(event);
                    if let &glutin::event::WindowEvent::CloseRequested = event {
                        self.close_requested = true;
                    }
                }

                // @TODO render here if possible somehow:
                glutin::event::Event::RedrawRequested(_) => {
                    self.render_frame(windowed_context.window(), &mut pyrite_gl);
                    windowed_context.swap_buffers().unwrap();
                    wait_for_redraw = false;
                }
                _ => {}
            }

            if self.close_requested {
                *control_flow = glutin::event_loop::ControlFlow::Exit;
            } else {
                self.update_title(windowed_context.window());
                self.build_gba_frame(&mut pyrite_gl);
                if !wait_for_redraw {
                    windowed_context.window().request_redraw();
                }
            }
        });
    }

    fn handle_window_event(&mut self, window_event: &glutin::event::WindowEvent) {
        use glutin::event::VirtualKeyCode;
        use pyrite_gba::keypad::KeypadInput;

        match window_event {
            glutin::event::WindowEvent::ModifiersChanged(modifiers) => {
                self.modifier_ctrl = modifiers.ctrl();
                self.modifier_shift = modifiers.shift();
            }

            glutin::event::WindowEvent::KeyboardInput { input, .. } => {
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
                    Some(VirtualKeyCode::Up) => self.gba.set_key_pressed(KeypadInput::Up, pressed),
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

                    Some(VirtualKeyCode::Q) => {
                        if self.modifier_shift && self.modifier_ctrl {
                            self.close_requested = true;
                        }
                    }

                    _ => { /* NOP */ }
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

    pub fn update_timers(&mut self) {
        let now = std::time::Instant::now();
        self.gba_frame_timer.update(now);
        self.title_update_timer.update(now);
    }

    pub fn update_title(&mut self, window: &glutin::window::Window) {
        use std::fmt::Write;
        const FPS_MUL: f64 = 100.0 / 60.0;
        if !self.title_update_timer.pop_fire() {
            return;
        }
        self.title_buffer.clear();
        let fps = self.gba_frame_counter.fps();
        write!(
            &mut self.title_buffer,
            "Pyrite ({:.02} FPS | {:.02} %)",
            fps,
            fps * FPS_MUL
        )
        .unwrap();
        window.set_title(&self.title_buffer);
        self.gba_frame_counter.clear();
    }

    pub fn build_gba_frame(&mut self, pyrite_gl: &mut PyriteGL) {
        if !self.gba_frame_timer.pop_fire() {
            return;
        }
        let mut no_audio = pyrite_gba::NoAudioOutput;
        let frame_start = std::time::Instant::now();
        self.gba.video_frame(pyrite_gl, &mut no_audio);
        self.gba_frame_counter.add_frame(frame_start.elapsed());
        pyrite_gl.build_frame();
    }

    pub fn render_frame(&mut self, _window: &glutin::window::Window, pyrite_gl: &mut PyriteGL) {
        unsafe {
            gl::ClearColor(
                (0xC4 as f32) / 255.0,
                (0x3D as f32) / 255.0,
                (0x5F as f32) / 255.0,
                1.0,
            );
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        pyrite_gl.render();
    }

    fn init(&mut self, window: &glutin::window::Window) {
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
    }
}

pub struct FrameCounter {
    duration: std::time::Duration,
    count: u64,
}

impl FrameCounter {
    pub fn new() -> FrameCounter {
        FrameCounter {
            duration: std::time::Duration::from_nanos(0),
            count: 0,
        }
    }

    fn add_frame(&mut self, elapsed: std::time::Duration) {
        self.duration += elapsed;
        self.count += 1;
    }

    fn fps(&self) -> f64 {
        self.count as f64 / self.duration.as_secs_f64()
    }

    fn clear(&mut self) {
        self.duration = std::time::Duration::from_nanos(0);
        self.count = 0;
    }
}

pub struct Timer {
    last_time: std::time::Instant,
    required: std::time::Duration,
    fire: bool,
}

impl Timer {
    pub fn new(required: std::time::Duration) -> Timer {
        Timer {
            last_time: std::time::Instant::now(),
            required: required,
            fire: false,
        }
    }

    pub fn update(&mut self, now: std::time::Instant) {
        if now.duration_since(self.last_time) >= self.required {
            self.fire = true;
            self.last_time = now;
        }
    }

    pub fn pop_fire(&mut self) -> bool {
        std::mem::replace(&mut self.fire, false)
    }
}
