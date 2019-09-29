#[allow(dead_code)] mod platform;

use pyrite_gba::Gba;

fn main() {
    let exit_code = run_emulator();
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn run_emulator() -> i32 {
    let mut window = platform::glutin::window::Window::new("Pyrite Emulator", 480.0, 320.0);
    if let Err(_) = window.set_position_center() {
        log::error!("failed to place the window in the center of the screen");
    }
    let mut video = platform::opengl::PyriteGL::new();
    let mut no_audio = pyrite_gba::NoAudioOutput;

    let mut gba = Gba::new();

    const ROM_FILE: &str = "roms/tonc/m3_demo.gba";
    match load_binary(ROM_FILE) {
        Ok(rom_binary) => {
            gba.set_rom(rom_binary);
        },

        Err(err) => {
            eprintln!("error occurred while loading ROM ({}): {}", ROM_FILE, err);
            return 1;
        }
    }
    gba.reset(true);

    while !window.close_requested() {
        window.handle_events();

        loop {
            gba.step(&mut video, &mut no_audio);
            if gba.is_frame_ready() { break }
        }

        video.render();
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
