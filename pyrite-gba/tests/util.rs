use pyrite_gba::{Gba, GbaAudioOutput, GbaVideoOutput};
// use pyrite_arm::memory::ArmMemory;
use std::path::Path;

#[macro_export]
macro_rules! rgb5 {
    ($R:expr, $G:expr, $B:expr) => {
        ($R as u16 & 0x1F) | (($G as u16 & 0x1F) << 5) | (($B as u16 & 0x1F) << 10) | (0x8000u16)
    };
}

const BIOS_PATH: &str = "../roms/legal/gba-bios.bin";
const TEST_STATUS_ADDRESS: u32 = 0x02000004;

pub fn load_bios(gba: &mut Gba) {
    println!("loading BIOS from {}", BIOS_PATH);
    let bios = std::fs::read(BIOS_PATH).unwrap_or_else(|err| {
        panic!("failed to load BIOS from path `{}`: {}", BIOS_PATH, err);
    });
    gba.set_bios(bios);
}

pub fn load_rom<P: std::fmt::Display + AsRef<Path>>(gba: &mut Gba, rom_path: P) {
    println!("loading rom from {}", rom_path);
    let rom = std::fs::read(rom_path.as_ref()).unwrap_or_else(|err| {
        panic!("failed to load rom from path `{}`: {}", rom_path, err);
    });
    gba.set_rom(rom);
}

/// Steps the GBA until the predicate `pred` returns true.
pub fn step_until<F: FnMut(&mut Gba) -> bool>(
    gba: &mut Gba,
    video: &mut dyn GbaVideoOutput,
    audio: &mut dyn GbaAudioOutput,
    mut pred: F,
) {
    loop {
        gba.step(video, audio);
        if pred(gba) {
            break;
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TestStatus {
    Setup = 0xDEADBEEF,
    Ready = 0xABCDEF01,
    Break = 0xACFEBDBB,
}

pub fn step_until_status(
    gba: &mut Gba,
    video: &mut dyn GbaVideoOutput,
    audio: &mut dyn GbaAudioOutput,
    status: TestStatus,
) {
    println!("waiting for test status `{:?}`", status);
    step_until(gba, video, audio, |gba| {
        gba.hardware.view32(TEST_STATUS_ADDRESS) == status as u32
    });
}

/// Test GBA video implementation that just contains a frame buffer
/// that cna be queried.
pub struct GbaTestVideo {
    pub pixels: [[u16; 240]; 160],
}

impl GbaTestVideo {
    pub fn new() -> GbaTestVideo {
        GbaTestVideo {
            pixels: [[0u16; 240]; 160],
        }
    }

    /// Returns the color of a pixel at the given location.
    #[inline(always)]
    pub fn at<X: Into<u32>, Y: Into<u32>>(&self, x: X, y: Y) -> u16 {
        self.pixels[y.into() as usize][x.into() as usize]
    }

    pub fn iter<'s>(&'s self) -> impl 's + Iterator<Item = (u32, u32, u16)> {
        GbaVideoOutputIter {
            x: 0,
            y: 0,
            p: self,
        }
    }
}

impl GbaVideoOutput for GbaTestVideo {
    fn pre_frame(&mut self) {
        /* NOP */
    }

    fn post_frame(&mut self) {
        /* NOP */
    }

    fn display_line(&mut self, line: u32, pixels: &[u16; 240]) {
        self.pixels[line as usize].copy_from_slice(pixels);
    }
}

pub struct GbaVideoOutputIter<'a> {
    x: u32,
    y: u32,
    p: &'a GbaTestVideo,
}

impl<'a> Iterator for GbaVideoOutputIter<'a> {
    type Item = (u32, u32, u16);

    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= 240 {
            self.x = 0;
            self.y += 1;
        }

        if self.y >= 160 {
            return None;
        }

        let col = self.p.at(self.x, self.y);
        let ret = (self.x, self.y, col);

        self.x += 1;

        return Some(ret);
    }
}
