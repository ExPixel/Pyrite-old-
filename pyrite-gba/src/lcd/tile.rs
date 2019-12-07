use crate::hardware::{ VRAM, OAM };
use crate::util::memory::read_u16_unchecked;
use super::obj::{ ObjectPriority, render_objects_no_obj_window };
use super::{ LCDRegisters, LCDLineBuffer, BGControl, BGOffset, apply_mosaic };
use super::palette::GbaPalette;

pub fn render_mode0(registers: &LCDRegisters, vram: &VRAM, oam: &OAM, pal: &GbaPalette, pixels: &mut LCDLineBuffer) {
    let object_priorities = ObjectPriority::sorted(oam);

    for priority in (0usize..=3).rev() {
        for bg_index in (0usize..=3).rev() {
            if !registers.dispcnt.display_layer(bg_index as u16) { continue }
            if registers.bg_cnt[bg_index].priority() == priority as u16 {
                let textbg = TextBG::new(registers.bg_cnt[bg_index], registers.bg_ofs[bg_index], registers.mosaic);

                if registers.bg_cnt[bg_index].palette256() {
                    draw_text_bg_8bpp_no_obj_window(registers.line as u32, 0, 240, &textbg, vram, pal, pixels);
                } else {
                    draw_text_bg_4bpp_no_obj_window(registers.line as u32, 0, 240, &textbg, vram, pal, pixels);
                }
            }
        }

        if registers.dispcnt.display_layer(4) {
            render_objects_no_obj_window(registers, object_priorities.objects_with_priority(priority), vram, oam, pal, pixels);
        }
    }
}

pub fn render_mode1(registers: &LCDRegisters, vram: &VRAM, oam: &OAM, pal: &GbaPalette, pixels: &mut LCDLineBuffer) {
}

pub fn render_mode2(registers: &LCDRegisters, vram: &VRAM, oam: &OAM, pal: &GbaPalette, pixels: &mut LCDLineBuffer) {
}

pub fn draw_text_bg_4bpp_no_obj_window(line: u32, left: u32, right: u32, bg: &TextBG, vram: &VRAM, palette: &GbaPalette, dest: &mut LCDLineBuffer) {
    pub const BYTES_PER_TILE: u32 = 32;
    pub const BYTES_PER_LINE: u32 = 4;

    let start_scx = bg.xoffset & (bg.width - 1);
    let scy = if bg.mosaic_y > 0 {
        let original_scy = (bg.yoffset + line) & (bg.height - 1);
        original_scy - (original_scy % bg.mosaic_y)
    } else {
        (bg.yoffset + line) & (bg.height - 1)
    };
    let ty = scy % 8;

    let mut mdx = left;
    let mut dx = left;

    while dx < right {
        let scx = start_scx + dx - mdx;

        mdx += 1;
        if mdx >= bg.mosaic_x {
            mdx = 0;
        }

        let tile_info_offset = bg.get_tile_info_offset(scx, scy);
        if tile_info_offset > 0x10000 { dx += 1; continue }
        let tile_info = unsafe {
            read_u16_unchecked(vram, tile_info_offset as usize)
        };
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
        if bg.mosaic_x <= 1 && (scx % 8) == 0 && dx <= (right - 8) {
            let pinc = if horizontal_flip { -1i32 as u32 } else { 1u32 };
            for _ in 0..4 {
                let palette_entry = vram[pixel_offset as usize];
                let lo_palette_entry = palette_entry & 0xF;
                let hi_palette_entry = palette_entry >> 4;
                if lo_palette_entry != 0 {
                    dest.push_pixel(dx as usize, palette.bg16(tile_palette as usize, lo_palette_entry as usize), bg.first_target, bg.second_target);
                }
                if hi_palette_entry != 0 {
                    dest.push_pixel(dx as usize + 1, palette.bg16(tile_palette as usize, hi_palette_entry as usize), bg.first_target, bg.second_target);
                }
                dx += 2;
                pixel_offset = pixel_offset.wrapping_add(pinc);
            }
        } else {
            let palette_entry = (vram[pixel_offset as usize] >> ((tx % 2) << 2)) & 0xF;
            if palette_entry != 0 {
                dest.push_pixel(dx as usize, palette.bg16(tile_palette as usize, palette_entry as usize), bg.first_target, bg.second_target);
            }
            dx += 1;
        }
    }
}

pub fn draw_text_bg_8bpp_no_obj_window(line: u32, left: u32, right: u32, bg: &TextBG, vram: &VRAM, palette: &GbaPalette, dest: &mut LCDLineBuffer) {
    unimplemented!("8bpp tiles");
}

pub struct TextBG {
    /// Base address of characters.
    char_base: u32,
    /// Base address for screens.
    screen_base: u32,

    xoffset:    u32,
    yoffset:    u32,
    width:      u32,
    height:     u32,

    mosaic_x:   u32,
    mosaic_y:   u32,

    first_target:   bool,
    second_target:  bool,
}

impl TextBG {
    const SIZES: [(u32, u32); 4] = [
        (256, 256),
        (512, 256),
        (256, 512),
        (512, 512),
    ];

    pub fn new(control: BGControl, offset: BGOffset, reg_mosaic: super::Mosaic) -> TextBG {
        let (width, height) = TextBG::SIZES[control.screen_size() as usize];
        let mosaic = if control.mosaic() {
            reg_mosaic.bg
        } else {
            (0, 0)
        };

        TextBG {
            char_base:      control.char_base_block() as u32 * 16 * 1024,
            screen_base:    control.screen_base_block() as u32 * 2 * 1024,
            xoffset:        offset.x as u32,
            yoffset:        offset.y as u32,
            width:          width,
            height:         height,
            mosaic_x:       mosaic.0 as u32,
            mosaic_y:       mosaic.1 as u32,
            first_target:   false,
            second_target:  false,
        }
    }

    #[inline]
    fn get_tile_info_offset(&self, scx: u32, scy: u32) -> u32 {
        let area_y  = scy % 256;
        let area_ty = area_y / 8;
        let scx = scx & (self.width - 1); // @NOTE: this relies on bg.width being a power of 2
        let area_idx = (scy/256)*(self.width/256) + (scx/256);
        let area_x = scx % 256;
        let area_tx = area_x / 8;
        return self.screen_base + (area_idx * 2048)  + ((area_ty * 32) + area_tx)*2;
    }
}
