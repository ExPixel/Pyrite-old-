use glutin::{
    EventsLoop,
    WindowedContext,
    PossiblyCurrent,
};

pub struct Window {
    events_loop: Option<EventsLoop>,
    win_context: WindowedContext<PossiblyCurrent>,
    close_request_flag: bool,

    win_size: (f32, f32),
}

impl Window {
    pub fn new(title: &str, width: f64, height: f64) -> Window {
        let el = EventsLoop::new();
        let wb = glutin::WindowBuilder::new()
            .with_title(title)
            .with_dimensions(glutin::dpi::LogicalSize::new(width, height));
        let windowed_context = glutin::ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(wb, &el)
            .expect("Failed to build windowed context.");

        let windowed_context = unsafe {
            let current_context = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| current_context.get_proc_address(symbol) as *const _);
            current_context
        };

        Window {
            events_loop: Some(el),
            win_context: windowed_context,

            close_request_flag: false,
            win_size: (width as _, height as _),
        }
    }

    pub fn close_requested(&self) -> bool {
        self.close_request_flag
    }

    pub fn handle_events(&mut self) {
        if let Some(mut events_loop) = self.events_loop.take() {
            events_loop.poll_events(|event| {
                match event {
                    glutin::Event::WindowEvent { event, .. } => {
                        self.handle_window_event(event);
                    },

                    _ => { /* NOP */ }
                }
            });
            self.events_loop = Some(events_loop);
        } else {
            panic!("Attempted to run tick with no active events loop.");
        }
    }

    fn handle_window_event(&mut self, event: glutin::WindowEvent) {
        match event {
            glutin::WindowEvent::KeyboardInput { input, .. } => {
                if let Some(glutin::VirtualKeyCode::Escape) = input.virtual_keycode {
                    if input.state == glutin::ElementState::Released {
                        self.close_request_flag = true;
                    }
                }
            },
            glutin::WindowEvent::CloseRequested => self.close_request_flag = true,
            glutin::WindowEvent::Resized(logical_size) => {
                let dpi_factor = self.win_context.window().get_hidpi_factor();
                let physical_size = logical_size.to_physical(dpi_factor);
                self.win_context.resize(physical_size);
                self.win_size = (physical_size.width as _, physical_size.height as _);
                unsafe {
                    gl::Viewport(0, 0, physical_size.width as _, physical_size.height as _);
                }
            },
            _ => { /* NOP */ }
        }
    }

    pub fn flip(&self) {
        self.win_context.swap_buffers().unwrap();
    }

    /// Place this window in the center of the screen.
    pub fn set_position_center(&mut self) -> Result<glutin::dpi::LogicalPosition, ()> {
        let window = self.win_context.window();
        let window_dimensions = if let Some(d) = window.get_outer_size() {
            d
        } else {
            return Err(());
        };

        let monitor = window.get_current_monitor();
        let monitor_dimensions = monitor.get_dimensions();
        let monitor_dpi = monitor.get_hidpi_factor();

        let physical_center = glutin::dpi::PhysicalPosition::new(
            monitor_dimensions.width / 2.0 - window_dimensions.width / 2.0,
            monitor_dimensions.height / 2.0 - window_dimensions.height / 2.0,
        );
        let logical_center = physical_center.to_logical(monitor_dpi);
        self.win_context.window().set_position(logical_center);
        return Ok(logical_center);
    }

    pub fn set_title(&mut self, title: &str) {
        self.win_context.window().set_title(title);
    }

    pub fn width(&self) -> f32 {
        self.win_size.0
    }

    pub fn height(&self) -> f32 {
        self.win_size.1
    }

    pub fn center_x(&self) -> f32 {
        self.win_size.0 / 2.0
    }

    pub fn center_y(&self) -> f32 {
        self.win_size.1 / 2.0
    }
}
