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

fn setup_gba(rom_file: &str) -> Box<Gba> {
    use std::fs::File;
    use std::io::prelude::*;

    let mut gba = Gba::alloc();

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

fn tonc_benchmarks(c: &mut Criterion) {
    let tonc_benchmarks: &[(usize, &'static str, &'static str)] = &[
        (60, "m3_demo", "../roms/tonc/m3_demo.gba"),
        (50, "brin_demo", "../roms/tonc/brin_demo.gba"),
        (50, "cbb_demo", "../roms/tonc/cbb_demo.gba"),
        (50, "obj_demo", "../roms/tonc/obj_demo.gba"),
        (40, "bld_demo", "../roms/tonc/bld_demo.gba"),
        (40, "win_demo", "../roms/tonc/win_demo.gba"),
    ];

    let mut group = c.benchmark_group("tonc");
    for (sample_count, name, filepath) in tonc_benchmarks.iter() {
        if *sample_count == 0 {
            group.sample_size(100);
        } else {
            group.sample_size(*sample_count);
        }

        let mut gba = setup_gba(*filepath);
        draw_frames(&mut gba, 256); // used to get into the correct mode

        group.bench_function(*name, |b| b.iter(|| draw_single_frame(&mut gba)));

        black_box(gba);
    }
    group.finish();
}

// fn gba_affine_bg_benchmark(c: &mut Criterion) {
//     let mut gba = setup_gba("../roms/tonc/sbb_aff.gba");
//     draw_frames(&mut gba, 256);

//     c.bench_function("mode1 affine bg", |b| {
//         b.iter(|| draw_single_frame(&mut gba))
//     });
//     black_box(gba);
// }

criterion_group!(benches, tonc_benchmarks,);
criterion_main!(benches);
