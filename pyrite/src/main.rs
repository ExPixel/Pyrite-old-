mod gba_imgui;
mod logger;
#[allow(dead_code)]
mod platform;
#[allow(dead_code)]
mod util;

use pyrite_gba::Gba;

fn main() {
    logger::PyriteLogger::init(log::Level::Trace);
    let exit_code = run_emulator();
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn run_emulator() -> i32 {
    let mut window = platform::glutin::window::Window::new("Pyrite", 240.0 * 3.0, 160.0 * 3.0);
    if let Err(_) = window.set_position_center() {
        log::error!("failed to place the window in the center of the screen");
    }

    let mut gba = Gba::alloc();

    const BIOS_FILE: &str = "roms/legal/gba-bios.bin";
    match load_binary(&BIOS_FILE) {
        Ok(bios_binary) => {
            gba.set_bios(bios_binary);
        }

        Err(err) => {
            log::error!("error occurred while loading BIOS ({}): {}", BIOS_FILE, err);
            return 1;
        }
    }

    if let Some(rom_file) = std::env::args().nth(1) {
        match load_binary(&rom_file) {
            Ok(rom_binary) => {
                gba.set_rom(rom_binary);
            }

            Err(err) => {
                log::error!("error occurred while loading ROM ({}): {}", rom_file, err);
                return 1;
            }
        }
        gba.reset(true);
    } else {
        log::error!("error: must pass a GBA ROM as the first argument");
        return 1;
    }

    let mut imgui_ui = gba_imgui::GbaImGui::new(gba, window.glutin_window());
    while !window.close_requested() {
        window.handle_events_with_handler(|window, event| imgui_ui.handle_event(window, event));
        imgui_ui.render_frame(window.glutin_window());
        window.flip();
    }

    return 0;
}

fn load_binary<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Vec<u8>> {
    use std::fs::File;
    use std::io::prelude::*;

    let mut f = File::open(path)?;
    let mut binary = Vec::new();
    f.read_to_end(&mut binary)?;
    return Ok(binary);
}
