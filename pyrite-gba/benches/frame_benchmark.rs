#[macro_use]
extern crate criterion;

use criterion::Criterion;
use criterion::black_box;

use pyrite_gba::Gba;

fn draw_frames(gba: &mut Gba, frames: u32) {
    let mut no_video = pyrite_gba::NoVideoOutput;
    let mut no_audio = pyrite_gba::NoAudioOutput;

    for _ in 0..frames {

        loop {
            gba.step(&mut no_video, &mut no_audio);
            if gba.is_frame_ready() {
                break
            }
        }
    }

    black_box(no_video);
    black_box(no_audio);
}

fn draw_single_frame(gba: &mut Gba) {
    let mut no_video = pyrite_gba::NoVideoOutput;
    let mut no_audio = pyrite_gba::NoAudioOutput;

    loop {
        gba.step(&mut no_video, &mut no_audio);
        if gba.is_frame_ready() {
            break
        }
    }

    black_box(no_video);
    black_box(no_audio);
}

fn setup_gba(rom_file: &str) -> Gba {
    use std::fs::File;
    use std::io::prelude::*;

    let mut gba = Gba::new();

    let mut rom = Vec::new();
    {
        let mut rom_file = File::open(rom_file).expect("failed to open rom file");
        rom_file.read_to_end(&mut rom).expect("failed to read rom file");
    }
    gba.set_rom(rom);

    gba.reset(true);
    return gba;
}

fn gba_mode3_benchmark(c: &mut Criterion) {
    let mut gba = setup_gba("../roms/tonc/m3_demo.gba");
    draw_frames(&mut gba, 256);

    c.bench_function("mode3 frame", |b| b.iter(|| draw_single_frame(&mut gba)));
    black_box(gba);
}

fn gba_mode0_benchmark(c: &mut Criterion) {
    let mut gba = setup_gba("../roms/tonc/brin_demo.gba");
    draw_frames(&mut gba, 256);

    c.bench_function("mode0 frame", |b| b.iter(|| draw_single_frame(&mut gba)));
    black_box(gba);
}

fn gba_mode0_simple_blending_benchmark(c: &mut Criterion) {
    let mut gba = setup_gba("../roms/tonc/cbb_demo.gba");
    draw_frames(&mut gba, 256);

    c.bench_function("mode0 simple blending frame", |b| b.iter(|| draw_single_frame(&mut gba)));
    black_box(gba);
}

criterion_group!(benches, gba_mode3_benchmark, gba_mode0_benchmark, gba_mode0_simple_blending_benchmark);
criterion_main!(benches);
