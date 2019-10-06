//! In BG Modes 3-5 the background is defined in form of a bitmap (unlike as for Tile/Map based BG modes).
//! Bitmaps are implemented as BG2, with Rotation/Scaling support. As bitmap modes are occupying 80KBytes of BG memory,
//! only 16KBytes of VRAM can be used for OBJ tiles.

use super::Line;
use super::super::GbaMemory;
use super::super::memory::read16_le;

/// BG Mode 3 - 240x160 pixels, 32768 colors
/// Two bytes are associated to each pixel, directly defining one of the 32768 colors (without using palette data,
/// and thus not supporting a 'transparent' BG color).
///
///   Bit   Expl.
///   0-4   Red Intensity   (0-31)
///   5-9   Green Intensity (0-31)
///   10-14 Blue Intensity  (0-31)
///   15    Not used in GBA Mode (in NDS Mode: Alpha=0=Transparent, Alpha=1=Normal)
///
/// The first 480 bytes define the topmost line, the next 480 the next line, and so on.
/// The background occupies 75 KBytes (06000000-06012BFF), most of the 80 Kbytes BG area,
/// not allowing to redraw an invisible second frame in background, so this mode is mostly recommended for still images only.
pub fn mode3(line: u32, out: &mut Line, memory: &mut GbaMemory) {
    let pixel_data_start = 480 * line as usize;
    for pixel_offset in 0..240 {
        let pixel = read16_le(&memory.mem_vram, pixel_data_start + (pixel_offset * 2));
        out[pixel_offset as usize] = pixel;
    }
}

pub fn mode4(line: u32, out: &mut Line, memory: &mut GbaMemory) {
    const FRAME1_OFFSET: usize = 0xA000;

    let pixel_data_start = 240*(line as usize) + FRAME1_OFFSET*(memory.ioregs.dispcnt.frame() as usize);
    for pixel_offset in 0..240 {
        let pixel = memory.mem_vram[pixel_data_start + pixel_offset];
        out[pixel_offset as usize] = memory.palette.get_bg256(pixel);
    }
}

pub fn mode5(_line: u32, _out: &mut Line, _memory: &mut GbaMemory) {
    unimplemented!("mode5 rendering");
}
