mod util;
use pyrite_gba::{Gba, NoAudioOutput};
use util::{GbaTestVideo, TestStatus};

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
