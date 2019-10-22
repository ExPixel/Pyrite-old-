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
use super::super::memory::ioreg::{ RegBGxCNT, RegBGxHOFS, RegBGxVOFS, RegFixedPoint16, RegFixedPoint28, RegMosaic };
use super::super::memory::palette::Palette;
use super::super::memory::read16_le;
// use super::super::memory::palette::u16_to_pixel;

/// Contains priority and blending information about a pixel.
#[derive(Copy, Clone)]
struct PixelInfo {
    /// This is true if the current pixel at this position is selected as a first target pixel in
    /// the color special effects register.
    is_first_target:        bool,

    /// This is true if the highest second target pixel has already been blended with the first
    /// target pixel in this location.
    second_target_blended:  bool,

    /// The priority assigned to the current pixel in this position.
    priority:               u8,
}

pub fn mode0(line: u32, out: &mut Line, memory: &mut GbaMemory) {
    let backdrop = memory.palette.get_bg256(0) | 0x8000;

    // first we clear the background completely.
    for p in out.iter_mut() { *p = backdrop; }

    let mut pixel_info = [PixelInfo {
        is_first_target: false,
        second_target_blended: false,
        priority: 4,
    }; 240];

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
            let bg = TextBG::new(cnt, xoffset, yoffset, memory.ioregs.mosaic);

            if cnt.pal256() {
                draw_bg_text_mode_8bpp(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                    poke_bg_pixel(off, col, priority as u8, out, &mut pixel_info);
                });
            } else {
                draw_bg_text_mode_4bpp(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                    poke_bg_pixel(off, col, priority as u8, out, &mut pixel_info);
                });
            }
        }
    }

    if memory.ioregs.dispcnt.screen_display_obj() {
        obj::draw_objects(line, memory.ioregs.dispcnt.obj_one_dimensional(), &memory.mem_vram, &memory.mem_oam, &memory.ioregs, &memory.palette, 0x10000, |off, col, priority| {
            poke_obj_pixel(off, col, priority, out, &mut pixel_info);
        });
    }
}

pub fn mode1(line: u32, out: &mut Line, memory: &mut GbaMemory) {
    let backdrop = memory.palette.get_bg256(0) | 0x8000;

    // first we clear the background completely.
    for p in out.iter_mut() { *p = backdrop; }

    let mut pixel_info = [PixelInfo {
        is_first_target: false,
        second_target_blended: false,
        priority: 4,
    }; 240];


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

            if bg_idx == 2 {
                let bg = AffineBG::new(cnt,
                    memory.ioregs.internal_bg2x,
                    memory.ioregs.internal_bg2y,
                    memory.ioregs.bg2pa,
                    memory.ioregs.bg2pc);

                draw_affine_bg(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                    poke_bg_pixel(off, col, priority as u8, out, &mut pixel_info);
                });

                memory.ioregs.internal_bg2x.inner = (memory.ioregs.internal_bg2x.inner.wrapping_add(memory.ioregs.bg2pb.inner as i16 as i32 as u32) << 4) >> 4;
                memory.ioregs.internal_bg2y.inner = (memory.ioregs.internal_bg2y.inner.wrapping_add(memory.ioregs.bg2pd.inner as i16 as i32 as u32) << 4) >> 4;
            } else {
                let xoffset = memory.ioregs.bg_hofs[bg_idx];
                let yoffset = memory.ioregs.bg_vofs[bg_idx];
                let bg = TextBG::new(cnt, xoffset, yoffset, memory.ioregs.mosaic);

                if cnt.pal256() {
                    draw_bg_text_mode_8bpp(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                        poke_bg_pixel(off, col, priority as u8, out, &mut pixel_info);
                    });
                } else {
                    draw_bg_text_mode_4bpp(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                        poke_bg_pixel(off, col, priority as u8, out, &mut pixel_info);
                    });
                }
            }
        }
    }

    obj::draw_objects(line, memory.ioregs.dispcnt.obj_one_dimensional(), &memory.mem_vram, &memory.mem_oam, &memory.ioregs, &memory.palette, 0x10000, |off, col, priority| {
        poke_obj_pixel(off, col, priority, out, &mut pixel_info);
    });
}

pub fn mode2(line: u32, out: &mut Line, memory: &mut GbaMemory) {
    let backdrop = memory.palette.get_bg256(0) | 0x8000;

    // first we clear the background completely.
    for p in out.iter_mut() { *p = backdrop; }

    let mut pixel_info = [PixelInfo {
        is_first_target: false,
        second_target_blended: false,
        priority: 4,
    }; 240];

    for priority in 0u16..=3u16 {
        for bg_idx in 2usize..=3usize {
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

            if bg_idx == 2 {
                let bg = AffineBG::new(cnt,
                    memory.ioregs.internal_bg2x,
                    memory.ioregs.internal_bg2y,
                    memory.ioregs.bg2pa,
                    memory.ioregs.bg2pc);

                draw_affine_bg(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                    poke_bg_pixel(off, col, priority as u8, out, &mut pixel_info);
                });

                memory.ioregs.internal_bg2x.inner = (memory.ioregs.internal_bg2x.inner.wrapping_add(memory.ioregs.bg2pb.inner as i16 as i32 as u32) << 4) >> 4;
                memory.ioregs.internal_bg2y.inner = (memory.ioregs.internal_bg2y.inner.wrapping_add(memory.ioregs.bg2pd.inner as i16 as i32 as u32) << 4) >> 4;
            } else if bg_idx == 3 {
                let bg = AffineBG::new(cnt,
                    memory.ioregs.internal_bg3x,
                    memory.ioregs.internal_bg3y,
                    memory.ioregs.bg3pa,
                    memory.ioregs.bg3pc);

                draw_affine_bg(line, bg, &memory.mem_vram, &memory.palette, |off, col| {
                    poke_bg_pixel(off, col, priority as u8, out, &mut pixel_info);
                });

                memory.ioregs.internal_bg3x.inner = (memory.ioregs.internal_bg3x.inner.wrapping_add(memory.ioregs.bg3pb.inner as i16 as i32 as u32) << 4) >> 4;
                memory.ioregs.internal_bg3y.inner = (memory.ioregs.internal_bg3y.inner.wrapping_add(memory.ioregs.bg3pd.inner as i16 as i32 as u32) << 4) >> 4;
            }
        }
    }

    obj::draw_objects(line, memory.ioregs.dispcnt.obj_one_dimensional(), &memory.mem_vram, &memory.mem_oam, &memory.ioregs, &memory.palette, 0x10000, |off, col, priority| {
        poke_obj_pixel(off, col, priority, out, &mut pixel_info);
    });
}

/// Internal Screen Size (dots) and size of BG Map (bytes):
///
///   Value  Text Mode
///   0      256x256 (2K)
///   1      512x256 (4K)
///   2      256x512 (4K)
///   3      512x512 (8K)
const TEXT_MODE_SCREEN_SIZE: [(u32, u32); 4] = [
    (256, 256),
    (512, 256),
    (256, 512),
    (512, 512),
];

/// Internal Screen Size (dots) and size of BG Map (bytes):
///
///   Value  Rotation/Scaling Mode
///   0      128x128   (256 bytes)
///   1      256x256   (1K)
///   2      512x512   (4K)
///   3      1024x1024 (16K)
const ROTSCAL_SCREEN_SIZE: [(u32, u32); 4] = [
    (128, 128),
    (256, 256),
    (512, 512),
    (1024, 1024),
];

fn draw_affine_bg<F: FnMut(usize, u16)>(_line: u32, bg: AffineBG, vram: &[u8], palette: &Palette, mut poke: F) {
    let (x_mask, y_mask) = if bg.wraparound {
        ((bg.width - 1) as i32, (bg.height - 1) as i32)
    } else {
        (0xFFFFFFFFu32 as i32, 0xFFFFFFFFu32 as i32)
    };

    for idx in 0..240 {
        let x = (bg.ref_x.wrapping_add(bg.dx as i32 * idx as i32) << 4) >> 4;
        let y = (bg.ref_y.wrapping_add(bg.dy as i32 * idx as i32) << 4) >> 4;

        let real_x = ((x >> 8) & x_mask) as u32;
        let real_y = ((y >> 8) & y_mask) as u32;

        if (real_x < bg.width) & (real_y < bg.height) {
            let tx = real_x / 8;
            let ty = real_y / 8;
            let tile_number = vram[(bg.screen_base + (ty * (bg.width / 8)) + tx) as usize];
            let tile_pixel_data_offset = bg.char_base + (64 * tile_number as u32) + (8 * (real_y % 8)) + (real_x % 8);
            let tile_pixel = palette.get_bg256(vram[tile_pixel_data_offset as usize]);
            poke(idx, tile_pixel);
        }
    }
}

fn draw_bg_text_mode_4bpp<F: FnMut(usize, u16)>(line: u32, bg: TextBG, vram: &[u8], palette: &Palette, mut poke: F) {
    pub const BYTES_PER_TILE: u32 = 32;
    pub const BYTES_PER_LINE: u32 = 4;

    let start_scx = bg.xoffset & (bg.width - 1);
    let scy = if bg.mosaic_y > 0 {
        let original_scy = (bg.yoffset + line) & (bg.height - 1);
        original_scy - (original_scy % bg.mosaic_y as u32)
    } else {
        (bg.yoffset + line) & (bg.height - 1)
    };
    let ty = scy % 8;

    let mosaic_x = bg.mosaic_x as u32;
    let apply_mosaic_x = |x: u32| -> u32 {
        if mosaic_x > 0 {
            x - (x % mosaic_x)
        } else {
            x
        }
    };

    let mut dx = 0;
    while dx < 240 {
        let scx = apply_mosaic_x(start_scx + dx);
        let tile_info_offset = get_tile_info_offset(&bg, scx, scy);
        if tile_info_offset > 0x10000 { dx += 1; continue; }
        let tile_info = read16_le(vram, tile_info_offset as usize);
        let tile_number = (tile_info & 0x3FF) as u32;
        let tile_palette = ((tile_info >> 12) & 0xF) as u8;
        let horizontal_flip = (tile_info & 0x400) != 0;
        let vertical_flip = (tile_info & 0x800) != 0;

        let tx = if horizontal_flip { 7 - (scx % 8) } else { scx % 8 };
        let ty = if vertical_flip { 7 - ty } else { ty };

        let tile_data_start = bg.char_base + (BYTES_PER_TILE * tile_number);
        let mut pixel_offset = tile_data_start + (ty * BYTES_PER_LINE) + tx/2;
        if pixel_offset > 0x10000 { dx += 1; continue }

        // try to do 8 pixels at a time if possible:
        if mosaic_x == 0 && (scx % 8) == 0 && dx <= 231 {
            let pinc = if horizontal_flip { -1i32 as u32 } else { 1u32 };
            for _ in 0..4 {
                let palette_entry = vram[pixel_offset as usize];
                poke(dx as usize, palette.get_bg16(tile_palette, palette_entry & 0xF));
                poke((dx + 1) as usize, palette.get_bg16(tile_palette, palette_entry >> 4));
                dx += 2;
                pixel_offset = pixel_offset.wrapping_add(pinc);
            }
        } else {
            let palette_entry = (vram[pixel_offset as usize] >> ((tx % 2) << 2)) & 0xF;
            poke(dx as usize, palette.get_bg16(tile_palette, palette_entry));
            dx += 1;
        }
    }
}

fn draw_bg_text_mode_8bpp<F: FnMut(usize, u16)>(line: u32, bg: TextBG, vram: &[u8], palette: &Palette, mut poke: F) {
    pub const BYTES_PER_TILE: u32 = 64;
    pub const BYTES_PER_LINE: u32 = 8;

    let start_scx = bg.xoffset & (bg.width - 1);
    let scy = if bg.mosaic_y > 0 {
        let original_scy = (bg.yoffset + line) & (bg.height - 1);
        original_scy - (original_scy % bg.mosaic_y as u32)
    } else {
        (bg.yoffset + line) & (bg.height - 1)
    };
    let ty = scy % 8;

    let mosaic_x = bg.mosaic_x as u32;
    let apply_mosaic_x = |x: u32| -> u32 {
        if mosaic_x > 0 {
            x - (x % mosaic_x)
        } else {
            x
        }
    };

    let mut dx = 0;
    while dx < 240 {
        let scx = apply_mosaic_x(start_scx + dx);
        let tile_info_offset = get_tile_info_offset(&bg, scx, scy);
        if tile_info_offset > 0x10000 { dx += 1; continue; }
        let tile_info = read16_le(vram, tile_info_offset as usize);
        let tile_number = (tile_info & 0x3FF) as u32;
        let horizontal_flip = (tile_info & 0x400) != 0;
        let vertical_flip = (tile_info & 0x800) != 0;

        let tx = if horizontal_flip { 7 - (scx % 8) } else { scx % 8 };
        let ty = if vertical_flip { 7 - ty } else { ty };

        let tile_data_start = bg.char_base + (BYTES_PER_TILE * tile_number);
        let mut pixel_offset = tile_data_start + (ty * BYTES_PER_LINE) + tx;
        if pixel_offset > 0x10000 { dx += 1; continue }

        // try to do 8 pixels at a time if possible:
        if mosaic_x == 0 && (scx % 8) == 0 && dx <= 231 {
            let pinc = if horizontal_flip { -1i32 as u32 } else { 1u32 };
            for _ in 0..8 {
                let palette_entry = vram[pixel_offset as usize];
                poke(dx as usize, palette.get_bg256(palette_entry));
                dx += 1;
                pixel_offset = pixel_offset.wrapping_add(pinc);
            }
        } else {
            let palette_entry = vram[pixel_offset as usize];
            poke(dx as usize, palette.get_bg256(palette_entry));
            dx += 1;
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

    mosaic_x:   u16,
    mosaic_y:   u16,
}

impl TextBG {
    #[inline]
    pub fn new(bg_cnt: RegBGxCNT, bg_hofs: RegBGxHOFS, bg_vofs: RegBGxVOFS, mosaic: RegMosaic) -> TextBG {
        TextBG {
            char_base:      bg_cnt.char_base_block() as u32 * (1024 * 16),
            screen_base:    bg_cnt.screen_base_block() as u32 *  (1024 * 2),

            width:      TEXT_MODE_SCREEN_SIZE[bg_cnt.screen_size() as usize].0,
            height:     TEXT_MODE_SCREEN_SIZE[bg_cnt.screen_size() as usize].1,
            xoffset:    bg_hofs.offset() as u32,
            yoffset:    bg_vofs.offset() as u32,

            mosaic_x:   if bg_cnt.mosaic() { mosaic.bg_h_size() + 1 } else { 0 },
            mosaic_y:   if bg_cnt.mosaic() { mosaic.bg_v_size() + 1 } else { 0 },
        }
    }
}

struct AffineBG {
    char_base:      u32,
    screen_base:    u32,
    wraparound:     bool,
    width:  u32,
    height: u32,

    ref_x:  i32,
    ref_y:  i32,

    dx:     i16,
    dy:     i16,
}

impl AffineBG {
    #[inline]
    pub fn new(bg_cnt: RegBGxCNT, ref_x: RegFixedPoint28, ref_y: RegFixedPoint28, dx: RegFixedPoint16, dy: RegFixedPoint16) -> AffineBG {
        AffineBG {
            char_base:      bg_cnt.char_base_block() as u32 * (1024 * 16),
            screen_base:    bg_cnt.screen_base_block() as u32 *  (1024 * 2),
            wraparound:     bg_cnt.display_area_overflow_wrap(),
            width:  ROTSCAL_SCREEN_SIZE[bg_cnt.screen_size() as usize].0,
            height: ROTSCAL_SCREEN_SIZE[bg_cnt.screen_size() as usize].1,

            // copies bit 27 to 28-31
            ref_x:  ((ref_x.used_portion() as i32) << 4) >> 4,
            ref_y:  ((ref_y.used_portion() as i32) << 4) >> 4,

            dx:     dx.inner as i16,
            dy:     dy.inner as i16,
        }
    }
}

#[inline]
fn poke_bg_pixel(offset: usize, color: u16, bg_priority: u8, out: &mut Line, pixel_info: &mut [PixelInfo; 240]) {
    if (color & 0x8000) != 0 && pixel_info[offset].priority > bg_priority {
        pixel_info[offset].priority = bg_priority;
        out[offset] = color;
    }
}

#[inline]
fn poke_obj_pixel(offset: usize, color: u16, obj_priority: u8, out: &mut Line, pixel_info: &mut [PixelInfo; 240]) {
    if (color & 0x8000) != 0 && pixel_info[offset].priority >= obj_priority {
        // offset should never be out of bounds here
        unsafe {
            (*pixel_info.get_unchecked_mut(offset)).priority = obj_priority;
            *out.get_unchecked_mut(offset) = color;
        }
    }
}
