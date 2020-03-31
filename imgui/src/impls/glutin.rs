use crate::api as imgui;
use crate::flags::*;
use glutin::event::{DeviceEvent, WindowEvent};
use glutin::event::{ElementState, MouseButton, MouseScrollDelta, VirtualKeyCode};
use glutin::window::{CursorIcon, Window};
use std::time::Instant;

struct GlobalGlutinState {
    time: Option<Instant>,
    mouse_position: imgui::ImVec2,
    mouse_pressed: [bool; 3],
}

#[allow(non_upper_case_globals)]
static mut g: GlobalGlutinState = GlobalGlutinState {
    time: None,
    mouse_position: imgui::ImVec2 { x: 0.0, y: 0.0 },
    mouse_pressed: [false; 3],
};

pub fn frame_start_time() -> Option<std::time::Instant> {
    unsafe { g.time }
}

#[inline]
pub fn process_device_event(_gl_window: &Window, _event: &DeviceEvent) {
    /* NOP */
}
// let mut io = imgui::get_io().unwrap();
// match event {
//     DeviceEvent::ModifiersChanged(modifiers) => {
//         io.KeyShift = modifiers.shift();
//         io.KeyCtrl = modifiers.ctrl();
//         io.KeyAlt = modifiers.alt();
//         io.KeyShift = modifiers.logo();
//     }
//     _ => {}
// }

#[inline]
pub fn process_window_event(_gl_window: &Window, event: &WindowEvent) {
    let mut io = imgui::get_io().unwrap();
    match event {
        WindowEvent::ModifiersChanged(modifiers) => {
            io.KeyShift = modifiers.shift();
            io.KeyCtrl = modifiers.ctrl();
            io.KeyAlt = modifiers.alt();
            io.KeySuper = modifiers.logo();
        }
        WindowEvent::MouseWheel { delta, .. } => {
            let (x, y) = match delta {
                MouseScrollDelta::LineDelta(x, y) => (*x, *y),
                MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
            };
            if x > 0.0 {
                io.MouseWheelH += 1.0;
            }
            if x < 0.0 {
                io.MouseWheelH -= 1.0;
            }
            if y > 0.0 {
                io.MouseWheel += 1.0;
            }
            if y < 0.0 {
                io.MouseWheel -= 1.0;
            }
        }

        WindowEvent::MouseInput { state, button, .. } => match (state, button) {
            (ElementState::Pressed, MouseButton::Left) => unsafe {
                g.mouse_pressed[0] = true;
            },
            (ElementState::Pressed, MouseButton::Right) => unsafe {
                g.mouse_pressed[1] = true;
            },
            (ElementState::Pressed, MouseButton::Middle) => unsafe {
                g.mouse_pressed[2] = true;
            },
            (ElementState::Released, MouseButton::Left) => unsafe {
                g.mouse_pressed[0] = false;
            },
            (ElementState::Released, MouseButton::Right) => unsafe {
                g.mouse_pressed[1] = false;
            },
            (ElementState::Released, MouseButton::Middle) => unsafe {
                g.mouse_pressed[2] = false;
            },
            _ => {}
        },

        WindowEvent::CursorMoved { position, .. } => unsafe {
            g.mouse_position.x = position.x as f32;
            g.mouse_position.y = position.y as f32;
        },

        WindowEvent::ReceivedCharacter(ch) => {
            let io = imgui::get_io().unwrap();
            let mut b5: [u8; 5] = [0; 5];
            let b4: [u8; 4] = unsafe { std::mem::transmute(*ch) };
            let chlen = ch.len_utf8();
            for idx in 0..chlen {
                b5[idx] = b4[idx];
            }
            let imstr =
                unsafe { crate::imstr::ImStr::from_bytes_with_nul_unchecked(&b5[0..(chlen + 1)]) };
            io.add_input_characters_utf8(&imstr);
        }

        WindowEvent::KeyboardInput { input, .. } => {
            let state_b = match input.state {
                ElementState::Pressed => true,
                ElementState::Released => false,
            };

            match input.virtual_keycode {
                Some(kc @ VirtualKeyCode::LShift) | Some(kc @ VirtualKeyCode::RShift) => {
                    io.KeyShift = state_b;
                    io.KeysDown[glutin_vkey_index(kc) as usize] = state_b;
                }

                Some(kc @ VirtualKeyCode::LControl) | Some(kc @ VirtualKeyCode::RControl) => {
                    io.KeyCtrl = state_b;
                    io.KeysDown[glutin_vkey_index(kc) as usize] = state_b;
                }

                Some(kc @ VirtualKeyCode::LAlt) | Some(kc @ VirtualKeyCode::RAlt) => {
                    io.KeyAlt = state_b;
                    io.KeysDown[glutin_vkey_index(kc) as usize] = state_b;
                }

                Some(kc @ VirtualKeyCode::LWin) | Some(kc @ VirtualKeyCode::RWin) => {
                    io.KeySuper = state_b;
                    io.KeysDown[glutin_vkey_index(kc) as usize] = state_b;
                }

                Some(kc) => {
                    io.KeysDown[glutin_vkey_index(kc) as usize] = state_b;
                }

                _ => {}
            }
        }

        WindowEvent::Resized(physical_size) => {
            let logical_size = physical_size.to_logical::<f64>(io.DisplayFramebufferScale.x as f64);
            io.DisplaySize.x = logical_size.width as f32;
            io.DisplaySize.y = logical_size.height as f32;
        }

        glutin::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            io.DisplayFramebufferScale = imgui::vec2(*scale_factor as f32, *scale_factor as f32)
        }

        _ => {}
    }
}

pub fn init(window_size: imgui::ImVec2, dpi_factor: f32) {
    let mut io = imgui::get_io().unwrap();
    io.BackendFlags |= BackendFlags::HasMouseCursors.bits();
    io.BackendFlags |= BackendFlags::HasSetMousePos.bits();
    io.DisplaySize.x = window_size.x;
    io.DisplaySize.y = window_size.y;
    io.DisplayFramebufferScale = imgui::vec2(dpi_factor, dpi_factor);

    // Initialize ImGui's key map:
    io.KeyMap[Key::A.bits() as usize] = glutin_vkey_index(VirtualKeyCode::A) as _;
    io.KeyMap[Key::C.bits() as usize] = glutin_vkey_index(VirtualKeyCode::C) as _;
    io.KeyMap[Key::V.bits() as usize] = glutin_vkey_index(VirtualKeyCode::V) as _;
    io.KeyMap[Key::X.bits() as usize] = glutin_vkey_index(VirtualKeyCode::X) as _;
    io.KeyMap[Key::Y.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Y) as _;
    io.KeyMap[Key::Z.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Z) as _;
    io.KeyMap[Key::End.bits() as usize] = glutin_vkey_index(VirtualKeyCode::End) as _;
    io.KeyMap[Key::Tab.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Tab) as _;
    io.KeyMap[Key::Home.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Home) as _;
    io.KeyMap[Key::Enter.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Return) as _;
    io.KeyMap[Key::Space.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Space) as _;
    io.KeyMap[Key::Delete.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Delete) as _;
    io.KeyMap[Key::Escape.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Escape) as _;
    io.KeyMap[Key::Insert.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Insert) as _;
    io.KeyMap[Key::PageUp.bits() as usize] = glutin_vkey_index(VirtualKeyCode::PageUp) as _;
    io.KeyMap[Key::PageDown.bits() as usize] = glutin_vkey_index(VirtualKeyCode::PageDown) as _;
    io.KeyMap[Key::Backspace.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Back) as _;
    io.KeyMap[Key::UpArrow.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Up) as _;
    io.KeyMap[Key::DownArrow.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Down) as _;
    io.KeyMap[Key::LeftArrow.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Left) as _;
    io.KeyMap[Key::RightArrow.bits() as usize] = glutin_vkey_index(VirtualKeyCode::Right) as _;
}

#[inline]
pub fn is_key_pressed(vkey: VirtualKeyCode) -> bool {
    let index = glutin_vkey_index(vkey) as i32;
    imgui::is_key_pressed(index, true)
}

#[inline]
pub fn glutin_vkey_index(vkey: VirtualKeyCode) -> usize {
    debug_assert!(
        (vkey as i32) < 512,
        "glutin_vkey_index(): VirtualKeyCode index is too high. (>= 512)"
    );
    vkey as i32 as usize
}

pub fn shutdown() {
    /* NOP */
    // This was supposed to do something but I forgot what that was.
    // I'll keep it here for now until I can remember :P
}

pub fn new_frame(gl_window: &Window) {
    new_frame_with_time(gl_window, Instant::now())
}

pub fn new_frame_with_time(gl_window: &Window, now: Instant) {
    let mut io = imgui::get_io().unwrap();
    if let Some(last_time) = unsafe { g.time.take() } {
        let dur = now.duration_since(last_time);
        io.DeltaTime = dur.as_secs_f32();
        unsafe {
            g.time = Some(now);
        }
    } else {
        unsafe {
            g.time = Some(now);
        }
        io.DeltaTime = 1.0f32 / 60.0f32;
    }

    update_mouse_pos_and_buttons(gl_window);
    update_mouse_cursor(gl_window);
}

pub fn update_mouse_pos_and_buttons(gl_window: &Window) {
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
        let _ = gl_window.set_cursor_position(glutin::dpi::LogicalPosition::new(
            io.MousePos.x as f64,
            io.MousePos.y as f64,
        ));
    } else {
        unsafe {
            io.MousePos = g.mouse_position.clone();
        }
    }
}

pub fn update_mouse_cursor(gl_window: &Window) {
    let io = imgui::get_io().unwrap();
    if (io.ConfigFlags & ConfigFlags::NoMouseCursorChange.bits()) != 0 {
        return;
    }
    let imgui_cursor = imgui::get_mouse_cursor();
    if io.MouseDrawCursor || imgui_cursor == MouseCursor::None {
        // Hide OS mouse cursor if imgui is drawing it or if it wants no cursor
        gl_window.set_cursor_visible(false);
    } else {
        // Show OS Cursor
        gl_window.set_cursor_icon(match imgui_cursor {
            MouseCursor::Hand => CursorIcon::Hand,
            MouseCursor::Arrow => CursorIcon::Arrow,
            MouseCursor::ResizeEW => CursorIcon::EwResize,
            MouseCursor::ResizeNS => CursorIcon::NsResize,
            MouseCursor::ResizeNESW => CursorIcon::NeswResize,
            MouseCursor::ResizeNWSE => CursorIcon::NwseResize,
            MouseCursor::ResizeAll => CursorIcon::Move, // #TODO there is not resize all in glutin so I wasn't sure what to use here.
            MouseCursor::TextInput => CursorIcon::Text,
            _ => CursorIcon::Default,
        });
        gl_window.set_cursor_visible(true);
    }
}
