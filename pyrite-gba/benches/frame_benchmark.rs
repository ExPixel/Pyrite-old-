#[macro_use]
extern crate criterion;

use criterion::black_box;
use criterion::Criterion;

use pyrite_gba::Gba;

fn draw_frames(gba: &mut Gba, frames: u32) {
    let mut no_video = pyrite_gba::NoVideoOutput;
    let mut no_audio = pyrite_gba::NoAudioOutput;

    for _ in 0..frames {
        gba.video_frame(&mut no_video, &mut no_audio);
    }
}

fn draw_single_frame(gba: &mut Gba) {
    let mut no_video = pyrite_gba::NoVideoOutput;
    let mut no_audio = pyrite_gba::NoAudioOutput;

    gba.video_frame(&mut no_video, &mut no_audio);
}

fn setup_gba(rom_file: &str) -> Gba {
    use std::fs::File;
    use std::io::prelude::*;

    let mut gba = Gba::new();

    let mut rom = Vec::new();
    {
        let mut rom_file = File::open(rom_file).expect("failed to open rom file");
        rom_file
            .read_to_end(&mut rom)
            .expect("failed to read rom file");
    }
    gba.set_rom(rom);

    gba.reset(true);
    return gba;
}

fn gba_mode3_benchmark(c: &mut Criterion) {
    let mut gba = setup_gba("../roms/tonc/m3_demo.gba");
    draw_frames(&mut gba, 256); // used to get into the correct mode

    c.bench_function("mode3 frame", |b| b.iter(|| draw_single_frame(&mut gba)));
    black_box(gba);
}

fn gba_mode0_benchmark(c: &mut Criterion) {
    let mut gba = setup_gba("../roms/tonc/brin_demo.gba");
    draw_frames(&mut gba, 256);

    c.bench_function("mode0 frame", |b| b.iter(|| draw_single_frame(&mut gba)));
    black_box(gba);
}

// fn gba_mode0_simple_blending_benchmark(c: &mut Criterion) {
//     let mut gba = setup_gba("../roms/tonc/cbb_demo.gba");
//     draw_frames(&mut gba, 256);

//     c.bench_function("mode0 simple blending frame", |b| {
//         b.iter(|| draw_single_frame(&mut gba))
//     });
//     black_box(gba);
// }

fn gba_obj_benchmark(c: &mut Criterion) {
    let mut gba = setup_gba("../roms/tonc/obj_demo.gba");
    draw_frames(&mut gba, 256);

    c.bench_function("obj frame", |b| b.iter(|| draw_single_frame(&mut gba)));
    black_box(gba);
}

// fn gba_affine_bg_benchmark(c: &mut Criterion) {
//     let mut gba = setup_gba("../roms/tonc/sbb_aff.gba");
//     draw_frames(&mut gba, 256);

//     c.bench_function("mode1 affine bg", |b| {
//         b.iter(|| draw_single_frame(&mut gba))
//     });
//     black_box(gba);
// }

criterion_group!(
    benches,
    gba_mode3_benchmark,
    gba_mode0_benchmark,
    // gba_mode0_simple_blending_benchmark,
    gba_obj_benchmark,
    // gba_affine_bg_benchmark
);
criterion_main!(benches);
