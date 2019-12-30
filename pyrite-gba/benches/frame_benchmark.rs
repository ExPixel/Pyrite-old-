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

fn tonc_m3_demo(c: &mut Criterion) {
    let mut gba = setup_gba("../roms/tonc/m3_demo.gba");
    draw_frames(&mut gba, 256); // used to get into the correct mode

    c.bench_function("tonc/m3_demo", |b| b.iter(|| draw_single_frame(&mut gba)));
    black_box(gba);
}

fn tonc_brin_demo(c: &mut Criterion) {
    let mut gba = setup_gba("../roms/tonc/brin_demo.gba");
    draw_frames(&mut gba, 256);

    c.bench_function("tonc/brin_demo", |b| b.iter(|| draw_single_frame(&mut gba)));
    black_box(gba);
}

fn tonc_cbb_demo(c: &mut Criterion) {
    let mut gba = setup_gba("../roms/tonc/cbb_demo.gba");
    draw_frames(&mut gba, 256);

    c.bench_function("tonc/cbb_demo", |b| b.iter(|| draw_single_frame(&mut gba)));
    black_box(gba);
}

fn tonc_obj_demo(c: &mut Criterion) {
    let mut gba = setup_gba("../roms/tonc/obj_demo.gba");
    draw_frames(&mut gba, 256);

    c.bench_function("tonc/obj_demo", |b| b.iter(|| draw_single_frame(&mut gba)));
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
    tonc_m3_demo,
    tonc_brin_demo,
    tonc_cbb_demo,
    tonc_obj_demo,
);
criterion_main!(benches);
