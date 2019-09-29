#[macro_use]
extern crate criterion;

use criterion::Criterion;
use criterion::black_box;

use pyrite_gba::Gba;

fn draw_frame(gba: &mut Gba, frames: u32) {
    for _ in 0..frames {
        let mut no_video = pyrite_gba::NoVideoOutput;
        let mut no_audio = pyrite_gba::NoAudioOutput;

        gba.step(&mut no_video, &mut no_audio);

        if gba.is_frame_ready() {
            return
        }
    }
}

fn single_step(gba: &mut Gba) {
    let mut no_video = pyrite_gba::NoVideoOutput;
    let mut no_audio = pyrite_gba::NoAudioOutput;
    gba.step(&mut no_video, &mut no_audio);
}

fn setup_gba() -> Gba {
    use std::fs::File;
    use std::io::prelude::*;

    pub const ROM_FILE: &str = "../roms/tonc/m3_demo.gba";

    let mut gba = Gba::new();

    let mut rom = Vec::new();
    {
        let mut rom_file = File::open(ROM_FILE).expect("failed to open rom file");
        rom_file.read_to_end(&mut rom).expect("failed to read rom file");
    }
    gba.set_rom(rom);

    gba.reset(true);
    return gba;
}

fn gba_step_benchmark(c: &mut Criterion) {
    let mut gba = setup_gba();
    c.bench_function("gba step", |b| b.iter(|| single_step(&mut gba)));
    black_box(gba);
}

fn gba_frame_benchmark(c: &mut Criterion) {
    let mut gba = setup_gba();
    c.bench_function("gba 60 frames", |b| b.iter(|| draw_frame(&mut gba, 60)));
    black_box(gba);
}

criterion_group!(benches, gba_frame_benchmark, gba_step_benchmark);
criterion_main!(benches);
