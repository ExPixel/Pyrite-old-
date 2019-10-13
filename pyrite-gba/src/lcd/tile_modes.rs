//! # LCD VRAM Overview
//!
//! ### BG Mode 0,1,2 (Tile/Map based Modes)
//! 
//!   06000000-0600FFFF  64 KBytes shared for BG Map and Tiles
//!   06010000-06017FFF  32 KBytes OBJ Tiles
//! 
//! The shared 64K area can be split into BG Map area(s), and BG Tiles area(s),
//! the respective addresses for Map and Tile areas are set up by BG0CNT-BG3CNT registers.
//! The Map address may be specified in units of 2K (steps of 800h), the Tile address in units of 16K (steps of 4000h).
//! 
//! ### BG Mode 0,1 (Tile/Map based Text mode)
//! The tiles may have 4bit or 8bit color depth, minimum map size is 32x32 tiles, maximum is 64x64 tiles, up to 1024 tiles can be used per map.
//! 
//!   Item        Depth     Required Memory
//!   One Tile    4bit      20h bytes
//!   One Tile    8bit      40h bytes
//!   1024 Tiles  4bit      8000h (32K)
//!   1024 Tiles  8bit      10000h (64K) - excluding some bytes for BG map
//!   BG Map      32x32     800h (2K)
//!   BG Map      64x64     2000h (8K)
//! 
//! 
//! ### BG Mode 1,2 (Tile/Map based Rotation/Scaling mode)
//! The tiles may have 8bit color depth only, minimum map size is 16x16 tiles,
//! maximum is 128x128 tiles, up to 256 tiles can be used per map.
//! 
//!   Item        Depth     Required Memory
//!   One Tile    8bit      40h bytes
//!   256  Tiles  8bit      4000h (16K)
//!   BG Map      16x16     100h bytes
//!   BG Map      128x128   4000h (16K)
//!
//! # LCD VRAM Character Data 
//!
//! Each character (tile) consists of 8x8 dots (64 dots in total). The color depth may be either
//! 4bit or 8bit (see BG0CNT-BG3CNT).
//!
//! ### 4bit depth (16 colors, 16 palettes)
//! Each tile occupies 32 bytes of memory, the first 4 bytes for the topmost row of the tile, and so
//! on. Each byte representing two dots, the lower 4 bits define the color for the left (!) dot, the
//! upper 4 bits the color for the right dot.
//!
//! ### 8bit depth (256 colors, 1 palette)
//! Each tile occupies 64 bytes of memory, the first 8 bytes for the topmost row of the tile, and so
//! on. Each byte selects the palette entry for each dot.
//!
//! # LCD VRAM BG Screen Data Format (BG Map) 
//!
//! The display background consists of 8x8 dot tiles,
//! the arrangement of these tiles is specified by the BG Screen Data (BG Map).
//! The separate entries in this map are as follows:
//! 
//! ### Text BG Screen (2 bytes per entry)
//! Specifies the tile number and attributes. Note that BG tile numbers are always specified in steps
//! of 1 (unlike OBJ tile numbers which are using steps of two in 256 color/1 palette mode).
//! 
//!   Bit   Expl.
//!   0-9   Tile Number     (0-1023) (a bit less in 256 color mode, because
//!                            there'd be otherwise no room for the bg map)
//!   10    Horizontal Flip (0=Normal, 1=Mirrored)
//!   11    Vertical Flip   (0=Normal, 1=Mirrored)
//!   12-15 Palette Number  (0-15)    (Not used in 256 color/1 palette mode)
//! 
//! A Text BG Map always consists of 32x32 entries (256x256 pixels), 400h entries = 800h bytes.
//! However, depending on the BG Size, one, two, or four of these Maps may be used together,
//! allowing to create backgrounds of 256x256, 512x256, 256x512, or 512x512 pixels, if so,
//! the first map (SC0) is located at base+0, the next map (SC1) at base+800h, and so on.
//! 
//! ### Rotation/Scaling BG Screen (1 byte per entry)
//! In this mode, only 256 tiles can be used. There are no x/y-flip attributes,
//! the color depth is always 256 colors/1 palette.
//! 
//!   Bit   Expl.
//!   0-7   Tile Number     (0-255)
//! 
//! The dimensions of Rotation/Scaling BG Maps depend on the BG size. For size 0-3 that are:
//! 16x16 tiles (128x128 pixels), 32x32 tiles (256x256 pixels), 64x64 tiles (512x512 pixels), or 128x128 tiles (1024x1024 pixels).
//! 
//! The size and VRAM base address of the separate BG maps for BG0-3 are set up by BG0CNT-BG3CNT registers.

use super::{ Line, obj };
use super::super::GbaMemory;
use super::super::memory::ioreg::{ RegBGxCNT, RegBGxHOFS, RegBGxVOFS };
use super::super::memory::palette::Palette;
use super::super::memory::read16_le;
// use super::super::memory::palette::u16_to_pixel;

pub fn mode0(line: u32, out: &mut Line, memory: &mut GbaMemory) {
    let backdrop = memory.palette.get_bg256(0) | 0x8000;

    // first we clear the background completely.
    for p in out.iter_mut() { *p = backdrop; }

    let mut pixel_priority_mask = [4u8; 240];

    for priority in 0u16..=3u16 {
        for bg_idx in 0usize..=3usize {
            let cnt = memory.ioregs.bg_cnt[bg_idx];
            if cnt.priority() != priority { continue; }
            let enabled = match bg_idx {
                0 => memory.ioregs.dispcnt.screen_display_bg0(),
                1 => memory.ioregs.dispcnt.screen_display_bg1(),
                2 => memory.ioregs.dispcnt.screen_display_bg2(),
                3 => memory.ioregs.dispcnt.screen_display_bg3(),
                _ => unreachable!(),
            };
            if !enabled { continue; }

            let xoffset = memory.ioregs.bg_hofs[bg_idx];
            let yoffset = memory.ioregs.bg_vofs[bg_idx];
            let bg = TextBG::new(cnt, xoffset, yoffset);

            if cnt.pal256() {
                draw_bg_text_mode_8bpp(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                    if (col & 0x8000) != 0 && pixel_priority_mask[off] > priority as u8 {
                        pixel_priority_mask[off] = priority as u8;
                        out[off] = col;
                    }
                });
            } else {
                draw_bg_text_mode_4bpp(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                    if (col & 0x8000) != 0 && pixel_priority_mask[off] > priority as u8 {
                        pixel_priority_mask[off] = priority as u8;
                        out[off] = col;
                    }
                });
            }
        }
    }

    obj::draw_objects(line, memory.ioregs.dispcnt.obj_one_dimensional(), &memory.mem_vram, &memory.mem_oam, &memory.palette, 0x10000, |off, col, priority| {
        if (col & 0x8000) != 0 && pixel_priority_mask[off] >= priority {
            // offset should never be out of bounds here
            unsafe {
                *pixel_priority_mask.get_unchecked_mut(off) = priority;
                *out.get_unchecked_mut(off) = col;
            }
        }
    });
}

pub fn mode1(line: u32, out: &mut Line, memory: &mut GbaMemory) {
    let backdrop = memory.palette.get_bg256(0) | 0x8000;

    // first we clear the background completely.
    for p in out.iter_mut() { *p = backdrop; }

    let mut pixel_priority_mask = [4u8; 240];

    for priority in 0u16..=3u16 {
        for bg_idx in 0usize..=2usize {
            let cnt = memory.ioregs.bg_cnt[bg_idx];
            if cnt.priority() != priority { continue; }
            let enabled = match bg_idx {
                0 => memory.ioregs.dispcnt.screen_display_bg0(),
                1 => memory.ioregs.dispcnt.screen_display_bg1(),
                2 => memory.ioregs.dispcnt.screen_display_bg2(),
                3 => memory.ioregs.dispcnt.screen_display_bg3(),
                _ => unreachable!(),
            };
            if !enabled { continue; }

            let xoffset = memory.ioregs.bg_hofs[bg_idx];
            let yoffset = memory.ioregs.bg_vofs[bg_idx];
            let bg = TextBG::new(cnt, xoffset, yoffset);

            if bg_idx == 2 {
            } else {
                if cnt.pal256() {
                    draw_bg_text_mode_8bpp(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                        if (col & 0x8000) != 0 && pixel_priority_mask[off] > priority as u8 {
                            pixel_priority_mask[off] = priority as u8;
                            out[off] = col;
                        }
                    });
                } else {
                    draw_bg_text_mode_4bpp(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                        if (col & 0x8000) != 0 && pixel_priority_mask[off] > priority as u8 {
                            pixel_priority_mask[off] = priority as u8;
                            out[off] = col;
                        }
                    });
                }
            }
        }
    }

    obj::draw_objects(line, memory.ioregs.dispcnt.obj_one_dimensional(), &memory.mem_vram, &memory.mem_oam, &memory.palette, 0x10000, |off, col, priority| {
        if (col & 0x8000) != 0 && pixel_priority_mask[off] >= priority {
            // offset should never be out of bounds here
            unsafe {
                *pixel_priority_mask.get_unchecked_mut(off) = priority;
                *out.get_unchecked_mut(off) = col;
            }
        }
    });
}

pub fn mode2(_line: u32, _out: &mut Line, _memory: &mut GbaMemory) {
}

/// Internal Screen Size (dots) and size of BG Map (bytes):
///
///   Value  Text Mode      Rotation/Scaling Mode
///   0      256x256 (2K)   128x128   (256 bytes)
///   1      512x256 (4K)   256x256   (1K)
///   2      256x512 (4K)   512x512   (4K)
///   3      512x512 (8K)   1024x1024 (16K)
const TEXT_MODE_SCREEN_SIZE: [(u32, u32); 4] = [
    (256, 256),
    (512, 256),
    (256, 512),
    (512, 512),
];

fn draw_affine_bg<F: FnMut(usize, u16)>(line: u32, bg: AffineBG, vram: &[u8], palette: Palette, mut poke: F) {
}

fn draw_bg_text_mode_4bpp<F: FnMut(usize, u16)>(line: u32, bg: TextBG, vram: &[u8], palette: &Palette, mut poke: F) {
    let scx = bg.xoffset & (bg.width - 1);
    let scy = (bg.yoffset + line) % bg.height;

    let ty = scy % 8;
    let align_start = if scx % 8 != 0 { 8 - (scx % 8) } else { 0 }; // start at the next whole tile on screen
    let align_end = if align_start != 0 { 8 - align_start } else { 0 };

    let mut dx = align_start;
    while dx <= (240 - 8 - align_end) {
        let tile_info_offset = get_tile_info_offset(&bg, scx + dx, scy);
        if tile_info_offset >= 0x10000 { // this is in object VRAM
            dx += 8;
            continue;
        }

        let tile_info = read16_le(vram, tile_info_offset as usize);
        let tile_number = (tile_info & 0x3FF) as u32;
        let tile_palette = ((tile_info >> 12) & 0xF) as u8;
        let horizontal_flip = (tile_info & 0x400) != 0;
        let vertical_flip = (tile_info & 0x800) != 0;
        let tile_data_start = bg.char_base + (32 * tile_number) + (if vertical_flip { 28 - (4 * ty) } else { 4 * ty });
        if tile_data_start >= 0x10000 { // this is in object VRAM
            dx += 8;
            continue;
        }

        if horizontal_flip {
            for otx in 0..4 {
                let tx = 3 - otx;
                let tpixel = vram[(tile_data_start + tx) as usize];
                let left = palette.get_bg16(tile_palette, tpixel & 0xF);
                let right = palette.get_bg16(tile_palette, (tpixel >> 4) & 0xF);
                poke((dx + otx*2) as usize, right);
                poke((dx + otx*2 + 1) as usize, left);
            }
        } else {
            for tx in 0..4 {
                let tpixel = vram[(tile_data_start + tx) as usize];
                let left = palette.get_bg16(tile_palette, tpixel & 0xF);
                let right = palette.get_bg16(tile_palette, (tpixel >> 4) & 0xF);
                poke((dx + tx*2) as usize, left);
                poke((dx + tx*2 + 1) as usize, right);
            }
        }

        dx += 8;
    }

    if align_start != 0 {
        // @NOTE I couldn't think of a better way to break out of a block wihout a whole bunch of nested
        // if statements...
        'left_edge: loop {
            let tile_info_offset = get_tile_info_offset(&bg, scx, scy);
            if tile_info_offset >= 0x10000 { // this is in object VRAM
                break 'left_edge;
            }

            let tile_info = read16_le(vram, tile_info_offset as usize);
            let tile_number = (tile_info & 0x3FF) as u32;
            let tile_palette = ((tile_info >> 12) & 0xF) as u8;
            let horizontal_flip = (tile_info & 0x400) != 0;
            let vertical_flip = (tile_info & 0x800) != 0;
            let tile_data_start = bg.char_base + (32 * tile_number) + (if vertical_flip { 28 - (4 * ty) } else { 4 * ty });
            if tile_data_start >= 0x10000 { // this is in object VRAM
                break 'left_edge;
            }

            let unalign_start = scx % 8;

            for otx in unalign_start..8 {
                let tx = if horizontal_flip { 7 - otx } else { otx };
                let tpixel = vram[(tile_data_start + (tx / 2)) as usize];
                if tx % 2 == 0 {
                    poke((otx - unalign_start) as usize, palette.get_bg16(tile_palette, tpixel & 0xF));
                } else {
                    poke((otx - unalign_start) as usize, palette.get_bg16(tile_palette, (tpixel >> 4) & 0xF));
                }
            }

            break 'left_edge;
        }

        'right_edge: loop {
            let tile_info_offset = get_tile_info_offset(&bg, scx + 240 - align_end, scy);
            if tile_info_offset >= 0x10000 { // this is in object VRAM
                break 'right_edge;
            }

            let tile_info = read16_le(vram, tile_info_offset as usize);
            let tile_number = (tile_info & 0x3FF) as u32;
            let tile_palette = ((tile_info >> 12) & 0xF) as u8;
            let horizontal_flip = (tile_info & 0x400) != 0;
            let vertical_flip = (tile_info & 0x800) != 0;
            let tile_data_start = bg.char_base + (32 * tile_number) + (if vertical_flip { 28 - (4 * ty) } else { 4 * ty });
            if tile_data_start >= 0x10000 { // this is in object VRAM
                break 'right_edge;
            }

            let unalign_start = 240 - align_end;
            for otx in unalign_start..240 {
                let tx = if horizontal_flip { 7 - (otx - unalign_start) } else { otx - unalign_start };
                let tpixel = vram[(tile_data_start + (tx / 2)) as usize];
                if tx % 2 == 0 {
                    poke(otx as usize, palette.get_bg16(tile_palette, tpixel & 0xF));
                } else {
                    poke(otx as usize, palette.get_bg16(tile_palette, (tpixel >> 4) & 0xF));
                }
            }

            break 'right_edge;
        }
    }
}

fn draw_bg_text_mode_8bpp<F: FnMut(usize, u16)>(line: u32, bg: TextBG, vram: &[u8], palette: &Palette, mut poke: F) {
    let scx = bg.xoffset & (bg.width - 1);
    let scy = (bg.yoffset + line) % bg.height;

    let ty = scy % 8;
    let align_start = if scx % 8 != 0 { 8 - (scx % 8) } else { 0 }; // start at the next whole tile on screen
    let align_end = if align_start != 0 { 8 - align_start } else { 0 };

    let mut dx = align_start;
    while dx <= (240 - 8 - align_end) {
        let tile_info_offset = get_tile_info_offset(&bg, scx + dx, scy);
        if tile_info_offset >= 0x10000 { // this is in object VRAM
            dx += 8;
            continue;
        }

        let tile_info = read16_le(vram, tile_info_offset as usize);
        let tile_number = (tile_info & 0x3FF) as u32;
        let horizontal_flip = (tile_info & 0x400) != 0;
        let vertical_flip = (tile_info & 0x800) != 0;
        let tile_data_start = bg.char_base + (64 * tile_number) + (if vertical_flip { 56 - (8 * ty) } else { 8 * ty });
        if tile_data_start >= 0x10000 { // this is in object VRAM
            dx += 8;
            continue;
        }

        if horizontal_flip {
            for otx in 0..8 {
                let tx = 7 - otx;
                let tpixel = vram[(tile_data_start + tx) as usize];
                poke((dx + otx) as usize, palette.get_bg256(tpixel));
            }
        } else {
            for tx in 0..8 {
                let tpixel = vram[(tile_data_start + tx) as usize];
                poke((dx + tx) as usize, palette.get_bg256(tpixel));
            }
        }

        dx += 8;
    }

    if align_start != 0 {
        // Left Edge
        'left_edge: loop {
            let tile_info_offset = get_tile_info_offset(&bg, scx, scy);
            if tile_info_offset >= 0x10000 { // this is in object VRAM
                break 'left_edge;
            }

            let tile_info = read16_le(vram, tile_info_offset as usize);
            let tile_number = (tile_info & 0x3FF) as u32;
            let horizontal_flip = (tile_info & 0x400) != 0;
            let vertical_flip = (tile_info & 0x800) != 0;
            let tile_data_start = bg.char_base + (64 * tile_number) + (if vertical_flip { 56 - (8 * ty) } else { 8 * ty });
            if tile_data_start >= 0x10000 { // this is in object VRAM
                break 'left_edge;
            }

            let unalign_start = scx % 8;

            for otx in unalign_start..8 {
                let tx = if horizontal_flip { 7 - otx } else { otx };
                let tpixel = vram[(tile_data_start + (tx / 2)) as usize];
                poke((otx - unalign_start) as usize, palette.get_bg256(tpixel));
            }

            break 'left_edge;
        }

        // Right Edge
        'right_edge: loop {
            let tile_info_offset = get_tile_info_offset(&bg, scx + 240 - align_end, scy);
            if tile_info_offset >= 0x10000 { // this is in object VRAM
                break 'right_edge;
            }

            let tile_info = read16_le(vram, tile_info_offset as usize);
            let tile_number = (tile_info & 0x3FF) as u32;
            let horizontal_flip = (tile_info & 0x400) != 0;
            let vertical_flip = (tile_info & 0x800) != 0;
            let tile_data_start = bg.char_base + (64 * tile_number) + (if vertical_flip { 56 - (8 * ty) } else { 8 * ty });
            if tile_data_start >= 0x10000 { // this is in object VRAM
                break 'right_edge;
            }

            let unalign_start = 240 - align_end;
            for otx in unalign_start..240 {
                let tx = if horizontal_flip { 7 - (otx - unalign_start) } else { otx - unalign_start };
                let tpixel = vram[(tile_data_start + (tx / 2)) as usize];
                poke(otx as usize, palette.get_bg256(tpixel));
            }

            break 'right_edge;
        }
    }
}

#[inline(always)]
fn get_tile_info_offset(bg: &TextBG, scx: u32, scy: u32) -> u32 {
    let area_y  = scy % 256;
    let area_ty = area_y / 8;
    let scx = scx & (bg.width - 1); // @NOTE: this relies on bg.width being a power of 2
    let area_idx = (scy/256)*(bg.width/256) + (scx/256);
    let area_x = scx % 256;
    let area_tx = area_x / 8;
    return bg.screen_base + (area_idx * 2048)  + ((area_ty * 32) + area_tx)*2;
}

struct TextBG {
    char_base:      u32,
    screen_base:    u32,

    width:      u32,
    height:     u32,
    xoffset:    u32,
    yoffset:    u32,
}

impl TextBG {
    #[inline]
    pub fn new(bg_cnt: RegBGxCNT, bg_hofs: RegBGxHOFS, bg_vofs: RegBGxVOFS) -> TextBG {
        TextBG {
            char_base:      bg_cnt.char_base_block() as u32 * (1024 * 16),
            screen_base:    bg_cnt.screen_base_block() as u32 *  (1024 * 2),

            width:      TEXT_MODE_SCREEN_SIZE[bg_cnt.screen_size() as usize].0,
            height:     TEXT_MODE_SCREEN_SIZE[bg_cnt.screen_size() as usize].1,
            xoffset:    bg_hofs.offset() as u32,
            yoffset:    bg_vofs.offset() as u32,
        }
    }
}

struct AffineBG {
}
