use std::time::Instant;
use glutin::{Event, WindowEvent};
use crate::api as imgui;
use crate::flags::*;

struct GlobalGlutinState {
    time: Option<Instant>,
    mouse_position: imgui::ImVec2,
    mouse_pressed:  [bool; 3],
}

#[allow(non_upper_case_globals)]
static mut g: GlobalGlutinState = GlobalGlutinState {
    time: None,
    mouse_position: imgui::ImVec2 { x: 0.0, y: 0.0 },
    mouse_pressed:  [false; 3],
};

pub fn frame_start_time() -> Option<std::time::Instant> {
    unsafe {
        g.time
    }
}

pub fn process_window_event(gl_window: &glutin::Window, event: &Event) -> bool {
    let mut io = imgui::get_io().unwrap();
    match event {
        Event::WindowEvent {event, ..} => match event {

            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = match delta {
                    glutin::MouseScrollDelta::LineDelta(x, y) => { (*x, *y) },
                    glutin::MouseScrollDelta::PixelDelta(pos) => { (pos.x as f32, pos.y as f32) }
                };
                if x > 0.0 { io.MouseWheelH += 1.0; }
                if x < 0.0 { io.MouseWheelH -= 1.0; }
                if y > 0.0 { io.MouseWheel  += 1.0; }
                if y < 0.0 { io.MouseWheel  -= 1.0; }
            },

            WindowEvent::MouseInput { state, button, .. } => {
                match (state, button) {
                    (glutin::ElementState::Pressed, glutin::MouseButton::Left) => {
                        unsafe { g.mouse_pressed[0] = true; }
                    },
                    (glutin::ElementState::Pressed, glutin::MouseButton::Right) => {
                        unsafe { g.mouse_pressed[1] = true; }
                    },
                    (glutin::ElementState::Pressed, glutin::MouseButton::Middle) => {
                        unsafe { g.mouse_pressed[2] = true; }
                    },
                    (glutin::ElementState::Released, glutin::MouseButton::Left) => {
                        unsafe { g.mouse_pressed[0] = false; }
                    },
                    (glutin::ElementState::Released, glutin::MouseButton::Right) => {
                        unsafe { g.mouse_pressed[1] = false; }
                    },
                    (glutin::ElementState::Released, glutin::MouseButton::Middle) => {
                        unsafe { g.mouse_pressed[2] = false; }
                    },
                    _ => {}
                }
            },

            WindowEvent::CursorMoved { position, .. } => {
                unsafe {
                    g.mouse_position.x = position.x as f32;
                    g.mouse_position.y = position.y as f32;
                }
            },

            WindowEvent::ReceivedCharacter(ch) => {
                let io = imgui::get_io().unwrap();
                let mut b5: [u8; 5] = [0; 5];
                let b4: [u8; 4] = unsafe {
                    std::mem::transmute(*ch)
                };
                let chlen = ch.len_utf8();
                for idx in 0..chlen { b5[idx] = b4[idx]; }
                let imstr = unsafe {
                    crate::imstr::ImStr::from_bytes_with_nul_unchecked(&b5[0..(chlen + 1)])
                };
                io.add_input_characters_utf8(&imstr);
            },

            WindowEvent::KeyboardInput { input, .. } => {
                let state_b = match input.state {
                    glutin::ElementState::Pressed => true,
                    glutin::ElementState::Released => false,
                };

                if let Some(kc) = input.virtual_keycode {
                    io.KeysDown[glutin_vkey_index(kc) as usize] = state_b;
                    io.KeyShift = input.modifiers.shift;
                    io.KeyCtrl = input.modifiers.ctrl;
                    io.KeyAlt = input.modifiers.alt;
                    io.KeySuper = input.modifiers.logo;
                }
            },

            WindowEvent::Resized(logical_size) => {
                let dpi_factor = gl_window.get_hidpi_factor() as f32;
                io.DisplaySize.x = logical_size.width as f32;
                io.DisplaySize.y = logical_size.height as f32;
                io.DisplayFramebufferScale = imgui::vec2(dpi_factor, dpi_factor);
            },

            _ => {},
        },
        _ => {},
    }
    return false;
}

pub fn init(window_size: imgui::ImVec2, dpi_factor: f32) {
    let mut io = imgui::get_io().unwrap();
    io.BackendFlags |= BackendFlags::HasMouseCursors.bits();
    io.BackendFlags |= BackendFlags::HasSetMousePos.bits();
    io.DisplaySize.x = window_size.x;
    io.DisplaySize.y = window_size.y;
    io.DisplayFramebufferScale = imgui::vec2(dpi_factor, dpi_factor);

    // Initialize ImGui's key map:
    io.KeyMap[Key::A.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::A) as _;
    io.KeyMap[Key::C.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::C) as _;
    io.KeyMap[Key::V.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::V) as _;
    io.KeyMap[Key::X.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::X) as _;
    io.KeyMap[Key::Y.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Y) as _;
    io.KeyMap[Key::Z.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Z) as _;
    io.KeyMap[Key::End.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::End) as _;
    io.KeyMap[Key::Tab.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Tab) as _;
    io.KeyMap[Key::Home.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Home) as _;
    io.KeyMap[Key::Enter.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Return) as _;
    io.KeyMap[Key::Space.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Space) as _;
    io.KeyMap[Key::Delete.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Delete) as _;
    io.KeyMap[Key::Escape.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Escape) as _;
    io.KeyMap[Key::Insert.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Insert) as _;
    io.KeyMap[Key::PageUp.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::PageUp) as _;
    io.KeyMap[Key::PageDown.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::PageDown) as _;
    io.KeyMap[Key::Backspace.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Back) as _;
    io.KeyMap[Key::UpArrow.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Up) as _;
    io.KeyMap[Key::DownArrow.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Down) as _;
    io.KeyMap[Key::LeftArrow.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Left) as _;
    io.KeyMap[Key::RightArrow.bits() as usize] = glutin_vkey_index(glutin::VirtualKeyCode::Right) as _;
}

#[inline(always)]
fn glutin_vkey_index(vkey: glutin::VirtualKeyCode) -> usize {
    debug_assert!((vkey as i32) < 512, "glutin_vkey_index(): VirtualKeyCode index is too high. (>= 512)");
    vkey as i32 as usize
}

pub fn shutdown() {
    /* NOP */
    // This was supposed to do something but I forgot what that was.
    // I'll keep it here for now until I can remember :P
}

pub fn new_frame(gl_window: &glutin::Window) {
    let mut io = imgui::get_io().unwrap();
    if let Some(last_time) = unsafe { g.time.take() } {
        let now = Instant::now();
        let dur = now.duration_since(last_time);
        let seconds = (dur.as_secs() as f64) + (dur.subsec_micros() as f64 / 1000000.0f64);
        io.DeltaTime = seconds as f32;
        unsafe {
            g.time = Some(now);
        }
    } else {
        unsafe {
            g.time = Some(Instant::now());
        }
        io.DeltaTime = 1.0f32 / 60.0f32;
    }

    update_mouse_pos_and_buttons(gl_window);
    update_mouse_cursor(gl_window);
}

pub fn update_mouse_pos_and_buttons(gl_window: &glutin::Window) {
    let mut io = imgui::get_io().unwrap();
    unsafe {
        io.MouseDown[0] = g.mouse_pressed[0];
        io.MouseDown[1] = g.mouse_pressed[1];
        io.MouseDown[2] = g.mouse_pressed[2];
        // #TODO we might miss mouse events this way.
        // g.mouse_pressed[0] = false;
        // g.mouse_pressed[1] = false;
        // g.mouse_pressed[2] = false;
    }


    if io.WantSetMousePos {
        // #TODO maybe I should be removing the HasSetMouseCursor flag from the backend support flags
        //       after this fails once (or maybe even after a certain number of times)
        let _ = gl_window.set_cursor_position(glutin::dpi::LogicalPosition::new(io.MousePos.x as f64, io.MousePos.y as f64));
    } else {
        unsafe {
            io.MousePos = g.mouse_position.clone();
        }
    }
}

pub fn update_mouse_cursor(gl_window: &glutin::Window) {
    let io = imgui::get_io().unwrap();
    if (io.ConfigFlags & ConfigFlags::NoMouseCursorChange.bits()) != 0 {
        return
    }
    let imgui_cursor = imgui::get_mouse_cursor();
    if io.MouseDrawCursor || imgui_cursor == MouseCursor::None {
        // Hide OS mouse cursor if imgui is drawing it or if it wants no cursor
        gl_window.hide_cursor(true);
    } else {
        // Show OS Cursor
        gl_window.set_cursor(match imgui_cursor {
            MouseCursor::Hand => glutin::MouseCursor::Hand,
            MouseCursor::Arrow => glutin::MouseCursor::Arrow,
            MouseCursor::ResizeEW => glutin::MouseCursor::EwResize,
            MouseCursor::ResizeNS => glutin::MouseCursor::NsResize,
            MouseCursor::ResizeNESW => glutin::MouseCursor::NeswResize,
            MouseCursor::ResizeNWSE => glutin::MouseCursor::NwseResize,
            MouseCursor::ResizeAll => glutin::MouseCursor::Move, // #TODO there is not resize all in glutin so I wasn't sure what to use here.
            MouseCursor::TextInput => glutin::MouseCursor::Text,
            _ => glutin::MouseCursor::Default,
        });
        gl_window.hide_cursor(false);
    }
}
