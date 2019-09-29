//! In BG Modes 3-5 the background is defined in form of a bitmap (unlike as for Tile/Map based BG modes).
//! Bitmaps are implemented as BG2, with Rotation/Scaling support. As bitmap modes are occupying 80KBytes of BG memory,
//! only 16KBytes of VRAM can be used for OBJ tiles.

use super::super::GbaMemory;

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
pub fn mode3(line: u32, out: &mut [(u8, u8, u8)], memory: &mut GbaMemory) {
    const BG_DATA_START: u32    = 0x06000000;
    // const BG_DATA_END: u32      = 0x06012BFE;

    let pixel_data_start = BG_DATA_START + (480 * line);
    let mut pixel_offset = 0;
    while pixel_offset < 240 {
        let pixel = memory.read_halfword(pixel_data_start + (pixel_offset * 2));
        out[pixel_offset as usize] = u16_to_pixel(pixel);
        pixel_offset += 1;
    }
}

#[inline(always)]
fn u16_to_pixel(p16: u16) -> (u8, u8, u8) {
    (
        (( p16        & 0x1F) as u8) * 8,
        (((p16 >>  5) & 0x1F) as u8) * 8,
        (((p16 >> 10) & 0x1F) as u8) * 8,
    )
}
