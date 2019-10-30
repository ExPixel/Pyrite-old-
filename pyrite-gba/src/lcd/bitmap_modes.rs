//! In BG Modes 3-5 the background is defined in form of a bitmap (unlike as for Tile/Map based BG modes).
//! Bitmaps are implemented as BG2, with Rotation/Scaling support. As bitmap modes are occupying 80KBytes of BG memory,
//! only 16KBytes of VRAM can be used for OBJ tiles.

use super::{ obj, RawLine };
use super::blending::{ apply_mosaic, poke_obj_pixel, poke_bg_pixel, get_compositing_info };
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
pub fn mode3(line: u32, raw_pixels: &mut RawLine, memory: &mut GbaMemory) {
    let (special_effects, windows) = get_compositing_info(&memory.ioregs);

    // Bitmap Modes use BG2
    if memory.ioregs.dispcnt.screen_display_obj() {
        obj::draw_objects(line, &memory.mem_vram, &memory.mem_oam, &memory.ioregs, &memory.palette, 0x14000, |off, col, priority, mode| {
            poke_obj_pixel(off, col, priority, mode, raw_pixels, special_effects, windows);
        });
    }

    if memory.ioregs.dispcnt.screen_display_bg2() {
        let mosaic_x = if memory.ioregs.bg_cnt[2].mosaic() { memory.ioregs.mosaic.bg_h_size() as u32 + 1 } else { 0 };
        let mosaic_y = if memory.ioregs.bg_cnt[2].mosaic() { memory.ioregs.mosaic.bg_v_size() as u32 + 1 } else { 0 };
        let priority = memory.ioregs.bg_cnt[2].priority() as u8;

        let y = apply_mosaic(line, mosaic_y);
        let pixel_data_start = 480 * y as usize;
        for screen_x in 0..240 {
            let pixel_offset = apply_mosaic(screen_x, mosaic_x) as usize;
            let pixel = read16_le(&memory.mem_vram, pixel_data_start + (pixel_offset * 2)) | 0x8000;
            poke_bg_pixel(2, screen_x as usize, pixel, priority, raw_pixels, special_effects, windows);
        }
    }
}

pub fn mode4(line: u32, raw_pixels: &mut RawLine, memory: &mut GbaMemory) {
    const FRAME1_OFFSET: usize = 0xA000;

    let (special_effects, windows) = get_compositing_info(&memory.ioregs);

    if memory.ioregs.dispcnt.screen_display_obj() {
        obj::draw_objects(line, &memory.mem_vram, &memory.mem_oam, &memory.ioregs, &memory.palette, 0x14000, |off, col, priority, mode| {
            poke_obj_pixel(off, col, priority, mode, raw_pixels, special_effects, windows);
        });
    }

    if memory.ioregs.dispcnt.screen_display_bg2() {
        let mosaic_x = if memory.ioregs.bg_cnt[2].mosaic() { memory.ioregs.mosaic.bg_h_size() as u32 + 1 } else { 0 };
        let mosaic_y = if memory.ioregs.bg_cnt[2].mosaic() { memory.ioregs.mosaic.bg_v_size() as u32 + 1 } else { 0 };
        let priority = memory.ioregs.bg_cnt[2].priority() as u8;

        let y = apply_mosaic(line, mosaic_y) as usize;
        let pixel_data_start = 240*y + FRAME1_OFFSET*(memory.ioregs.dispcnt.frame() as usize);
        for screen_x in 0..240 {
            let pixel_offset = apply_mosaic(screen_x, mosaic_x) as usize;
            let pixel = memory.mem_vram[pixel_data_start + pixel_offset];
            poke_bg_pixel(2, screen_x as usize, memory.palette.get_bg256(pixel), priority, raw_pixels, special_effects, windows);
        }
    }
}

pub fn mode5(line: u32, raw_pixels: &mut RawLine, memory: &mut GbaMemory) {
    const FRAME1_OFFSET: usize = 0xA000;

    let (special_effects, windows) = get_compositing_info(&memory.ioregs);

    if memory.ioregs.dispcnt.screen_display_obj() {
        obj::draw_objects(line, &memory.mem_vram, &memory.mem_oam, &memory.ioregs, &memory.palette, 0x14000, |off, col, priority, mode| {
            poke_obj_pixel(off, col, priority, mode, raw_pixels, special_effects, windows);
        });
    }

    if memory.ioregs.dispcnt.screen_display_bg2() && line < 128 {
        let mosaic_x = if memory.ioregs.bg_cnt[2].mosaic() { memory.ioregs.mosaic.bg_h_size() as u32 + 1 } else { 0 };
        let mosaic_y = if memory.ioregs.bg_cnt[2].mosaic() { memory.ioregs.mosaic.bg_v_size() as u32 + 1 } else { 0 };
        let priority = memory.ioregs.bg_cnt[2].priority() as u8;

        let y = apply_mosaic(line, mosaic_y) as usize;
        let pixel_data_start = 320*y + FRAME1_OFFSET*(memory.ioregs.dispcnt.frame() as usize);
        for screen_x in 0..160 {
            let pixel_offset = apply_mosaic(screen_x, mosaic_x) as usize;
            let pixel = read16_le(&memory.mem_vram, pixel_data_start + (pixel_offset * 2)) | 0x8000;
            poke_bg_pixel(2, screen_x as usize, pixel, priority, raw_pixels, special_effects, windows);
        }
    }
}
