mod util;
use pyrite_gba::{Gba, NoAudioOutput};
use util::{GbaTestVideo, TestStatus, SCREEN_WIDTH};

#[test]
pub fn test_mode3() {
    let mut gba = Gba::alloc();
    util::load_rom(&mut gba, "../roms/test/mode3.gba");
    gba.reset(true);

    let mut video = GbaTestVideo::new();
    let mut audio = NoAudioOutput;

    util::step_until_status(&mut gba, &mut video, &mut audio, TestStatus::Setup);
    util::step_until_status(&mut gba, &mut video, &mut audio, TestStatus::Ready);

    video.iter().for_each(|(x, y, col)| {
        assert_eq!(
            color_for_coord(x, y),
            col,
            "color for ({}, {}) is wrong",
            x,
            y
        );
    });

    fn color_for_coord(x: u32, y: u32) -> u16 {
        let r = x as u16 & 0x1F;
        let g = y as u16 & 0x1F;
        let b = (x ^ y) & 0x1F;
        rgb5!(r, g, b)
    }
}

#[test]
pub fn test_mode4() {
    let mut gba = Gba::alloc();
    util::load_rom(&mut gba, "../roms/test/mode3.gba");
    gba.reset(true);

    let palette = {
        let mut p = [0u16; 256];
        let (mut r, mut g, mut b) = (0, 0, 0);

        for idx in 0..128 {
            r = (r + 1) & 0x1F;
            g = (g + r) & 0x1F;
            b = (b + g) & 0x1F;
            p[idx] = rgb5!(r, g, b);
        }

        for idx in 128..256 {
            b = (b + 3) & 0x1F;
            g = (g + b) & 0x1F;
            r = (r + g) & 0x1F;
            p[idx] = rgb5!(r, g, b);
        }

        p
    };

    let mut video = GbaTestVideo::new();
    let mut audio = NoAudioOutput;

    util::step_until_status(&mut gba, &mut video, &mut audio, TestStatus::Setup);
    util::step_until_status(&mut gba, &mut video, &mut audio, TestStatus::Ready);

    video.iter().for_each(|(x, y, col)| {
        let expect_entry = (x + (y * SCREEN_WIDTH)) as u8;
        assert_eq!(
            palette[expect_entry as usize], col,
            "color for ({}, {}) is wrong (expected entry = {})",
            x, y, expect_entry
        );
    });

    util::step_until_status(&mut gba, &mut video, &mut audio, TestStatus::Break);

    video.iter().for_each(|(x, y, col)| {
        let expect_entry = 255 - ((x + (y * SCREEN_WIDTH)) as u8);
        assert_eq!(
            palette[expect_entry as usize], col,
            "color for ({}, {}) is wrong (expected entry = {})",
            x, y, expect_entry
        );
    });
}
