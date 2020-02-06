use super::{
    apply_mosaic, AffineBGParams, BGControl, BGOffset, LCDLineBuffer, LCDRegisters, Layer, Pixel,
};
use crate::hardware::VRAM;
use crate::util::memory::{memset, read_u32_unchecked, read_u64_unchecked, write_u64_unchecked};

pub fn render_mode0(registers: &LCDRegisters, vram: &VRAM, pixels: &mut LCDLineBuffer) {
    for priority in (0usize..=3).rev() {
        for bg_index in (0usize..=3).rev() {
            let layer = Layer::from_bg(bg_index as u16);
            if registers.dispcnt.display_layer(layer)
                && registers.bg_cnt[bg_index].priority() == priority as u16
                && (!pixels.windows.enabled || pixels.windows.line_visible(layer))
            {
                let first_target = registers.effects.is_first_target(layer);
                let second_target = registers.effects.is_second_target(layer);
                let textbg = TextBG::new(
                    layer,
                    priority as _,
                    registers.bg_cnt[bg_index],
                    registers.bg_ofs[bg_index],
                    registers.mosaic,
                    first_target,
                    second_target,
                );

                if registers.bg_cnt[bg_index].palette256() {
                    draw_text_bg_8bpp(registers.line as u32, &textbg, vram, pixels);
                } else {
                    draw_text_bg_4bpp(registers.line as u32, &textbg, vram, pixels);
                }
            }
        }
    }
}

pub fn render_mode1(registers: &mut LCDRegisters, vram: &VRAM, pixels: &mut LCDLineBuffer) {
    for priority in (0usize..=3).rev() {
        for bg_index in (0usize..=2).rev() {
            let layer = Layer::from_bg(bg_index as u16);
            if registers.dispcnt.display_layer(layer)
                && registers.bg_cnt[bg_index].priority() == priority as u16
                && (!pixels.windows.enabled || pixels.windows.line_visible(layer))
            {
                let first_target = registers.effects.is_first_target(layer);
                let second_target = registers.effects.is_second_target(layer);

                if bg_index == 2 {
                    let affinebg = AffineBG::new(
                        layer,
                        priority as _,
                        registers.bg_cnt[bg_index],
                        registers.bg2_affine_params.clone(),
                        registers.mosaic,
                        first_target,
                        second_target,
                    );

                    draw_affine_bg(&affinebg, vram, pixels);
                } else {
                    let textbg = TextBG::new(
                        layer,
                        priority as _,
                        registers.bg_cnt[bg_index],
                        registers.bg_ofs[bg_index],
                        registers.mosaic,
                        first_target,
                        second_target,
                    );

                    if registers.bg_cnt[bg_index].palette256() {
                        draw_text_bg_8bpp(registers.line as u32, &textbg, vram, pixels);
                    } else {
                        draw_text_bg_4bpp(registers.line as u32, &textbg, vram, pixels);
                    }
                }
            }
        }
    }

    registers.bg2_affine_params.increment_reference_points();
    registers.bg3_affine_params.increment_reference_points();
}

pub fn render_mode2(registers: &mut LCDRegisters, vram: &VRAM, pixels: &mut LCDLineBuffer) {
    for priority in (0usize..=3).rev() {
        for bg_index in (2usize..=3).rev() {
            let layer = Layer::from_bg(bg_index as u16);
            if registers.dispcnt.display_layer(layer)
                && registers.bg_cnt[bg_index].priority() == priority as u16
                && (!pixels.windows.enabled || pixels.windows.line_visible(layer))
            {
                let first_target = registers.effects.is_first_target(layer);
                let second_target = registers.effects.is_second_target(layer);

                let affinebg = AffineBG::new(
                    layer,
                    priority as _,
                    registers.bg_cnt[bg_index],
                    if bg_index == 2 {
                        registers.bg2_affine_params.clone()
                    } else {
                        registers.bg3_affine_params.clone()
                    },
                    registers.mosaic,
                    first_target,
                    second_target,
                );

                draw_affine_bg(&affinebg, vram, pixels);
            }
        }
    }

    registers.bg2_affine_params.increment_reference_points();
    registers.bg3_affine_params.increment_reference_points();
}

pub fn draw_affine_bg(bg: &AffineBG, vram: &VRAM, pixels: &mut LCDLineBuffer) {
    let (x_mask, y_mask) = if bg.wraparound {
        ((bg.width - 1) as i32, (bg.height - 1) as i32)
    } else {
        (0xFFFFFFFFu32 as i32, 0xFFFFFFFFu32 as i32)
    };

    let pixel_mask = bg.pixel_mask();

    let mut x = bg.params.internal_x;
    let mut y = bg.params.internal_y;

    for idx in 0..240 {
        x += bg.params.a; // x + dx
        y += bg.params.c; // y + dy

        let ix = apply_mosaic((x.integer() & x_mask) as u32, bg.mosaic_x as u32);
        let iy = apply_mosaic((y.integer() & y_mask) as u32, bg.mosaic_y as u32);

        if (ix < bg.width) & (iy < bg.height) {
            let tx = ix / 8;
            let ty = iy / 8;
            let tile_number = vram[(bg.screen_base + (ty * (bg.width / 8)) + tx) as usize];
            let tile_pixel_data_offset =
                bg.char_base + (64 * tile_number as u32) + (8 * (iy % 8)) + (ix % 8);
            let entry = vram[tile_pixel_data_offset as usize];

            if entry != 0 {
                if !pixels.windows.enabled {
                    pixels.push_pixel(idx, Pixel(pixel_mask | (entry as u16)));
                } else {
                    if let Some(window_effects_mask) = pixels.windows.check_pixel(bg.layer, idx) {
                        pixels.push_pixel(
                            idx,
                            Pixel((pixel_mask & window_effects_mask) | (entry as u16)),
                        );
                    }
                }
            }
        }
    }
}

struct TileLoader<'v> {
    vram: &'v VRAM,
    /// The current block of tiles.
    block: u64,
    /// The current offset being read from.
    offset: usize,
    line_end: usize,
    next_area: usize,
}

impl<'v> TileLoader<'v> {
    /// This function expects that X and Y do not exceed the width and height of the screen map.
    fn new(vram: &'v VRAM, base: u32, x: u32, y: u32, width: u32) -> TileLoader<'v> {
        let area = Self::get_area(x, y, width) as usize;
        // Get the x and y coordinates within the current area:
        let (area_x, area_y) = (x % 256, y % 256);
        // Get the x and y TILE coordinates within the current area:
        let (area_tx, area_ty) = (area_x / 8, area_y / 8);

        let mut offset =
            base as usize + (area * 0x800) + (area_ty as usize * 64) + (area_tx as usize * 2);
        let line_end = (offset & !0x3F) + 64;

        let misalignment = offset % 8;
        let block = if misalignment != 0 {
            let v = unsafe { read_u64_unchecked(vram, offset & !0x7) }; // do an aligned load
            if x % 8 != 0 {
                offset += 2;
                v >> (misalignment * 8)
            } else {
                // Because this is aligned, the extra offset increment and shift that is done above
                // will be done by the immediate call to advance.
                v >> ((misalignment - 2) * 8)
            }
        } else {
            // We're block aligned, but since we're not tile aligned, we have to preload the block
            // because a call to advance won't be done before the next pixel offset is read.
            if x % 8 != 0 {
                let v = unsafe { read_u64_unchecked(vram, offset) };
                offset += 2;
                v
            } else {
                0
            }
        };

        TileLoader {
            vram: vram,
            block: block,
            offset: offset,
            line_end: line_end,
            next_area: if width > 256 {
                if area % 2 == 0 {
                    // this is on the left and we want to increment the area
                    0x800
                } else {
                    // this is on the right and we want to decrement the area
                    (-0x800isize) as usize
                }
            } else {
                0
            },
        }
    }

    /// This should be called any time we're going to draw a pixel at a tile aligned boundary.
    /// It will correctly load in the next tile (or the next block/area if necessary).
    fn advance(&mut self) {
        // We load a new block because the offset is 8 byte aligned.
        // This all works because the TileLoader does not bother loading any data to start with if
        // the first pixel being drawn is aligned to the left edge of a tile. So offset % 8 will be
        // 0 and the first call to next will load a block.

        if self.offset % 8 == 0 {
            if self.offset == self.line_end {
                self.offset = (self.offset.wrapping_sub(2) & !0x3F).wrapping_add(self.next_area);
                self.line_end = self.offset + 64;
                self.block = unsafe { read_u64_unchecked(self.vram, self.offset) };
            } else {
                self.block = unsafe { read_u64_unchecked(self.vram, self.offset) };
            }
        } else {
            self.block >>= 16;
        }
        self.offset += 2;
    }

    fn tile_palette(&self) -> u8 {
        ((self.block >> 12) & 0xF) as u8
    }

    fn hflip(&self) -> bool {
        (self.block & 0x400) != 0
    }

    #[inline(always)]
    fn tile_pixel_offset(
        &mut self,
        bytes_per_tile: usize,
        bytes_per_line: usize,
        char_base: usize,
        ty: usize,
    ) -> usize {
        let tile_number = (self.block & 0x3FF) as usize;
        let vflip = (self.block & 0x800) != 0;
        let ty = if vflip { 7 - ty } else { ty };
        let tile_data_start = char_base + (bytes_per_tile * tile_number);
        let pixel_offset = (tile_data_start + (ty * bytes_per_line)) as usize;
        return pixel_offset;
    }

    fn get_area(x: u32, y: u32, width: u32) -> u32 {
        let area_y_inc = if width > 256 { 2 } else { 1 };
        (if x < 256 { 0 } else { 1 }) + (if y < 256 { 0 } else { area_y_inc })
    }
}

macro_rules! write_4bpp {
    ($Dest:expr, $DestOffset:expr, $Palette:expr, $Nibbles:expr) => {
        $Dest[$DestOffset as usize] = ($Palette * 16) + ($Nibbles as u8 & 0xF);
        $Dest[$DestOffset as usize + 1] = ($Palette * 16) + ($Nibbles as u8 >> 4);
    };
}

macro_rules! write_4bpp_rev {
    ($Dest:expr, $DestOffset:expr, $Palette:expr, $Nibbles:expr) => {
        $Dest[$DestOffset as usize] = ($Palette * 16) + ($Nibbles as u8 >> 4);
        $Dest[$DestOffset as usize + 1] = ($Palette * 16) + ($Nibbles as u8 & 0xF);
    };
}

pub fn draw_text_bg_4bpp(line: u32, bg: &TextBG, vram: &VRAM, pixels: &mut LCDLineBuffer) {
    pub const BYTES_PER_TILE: usize = 32;
    pub const BYTES_PER_LINE: usize = 4;

    let start_scx = bg.wrapped_xoffset();
    let scy = bg.wrapped_yoffset_at_line(line);
    let ty = (scy % 8) as usize;

    let mut dx = 0;
    let mut pixel_buffer = [0u8; 240];
    let mut tile_loader = TileLoader::new(vram, bg.screen_base, start_scx, scy, bg.width);

    while dx < 240 {
        let scx = start_scx + dx;

        if scx % 8 == 0 {
            tile_loader.advance();
        }

        // try to do 8 pixels at a time if possible:
        if (scx % 8) == 0 && dx <= 232 {
            let pixel_offset = tile_loader.tile_pixel_offset(
                BYTES_PER_TILE,
                BYTES_PER_LINE,
                bg.char_base as usize,
                ty,
            );
            let tile_palette = tile_loader.tile_palette();

            // we read all 8 nibbles (4 bytes) in one go:
            let pixels8 = unsafe { read_u32_unchecked(vram, pixel_offset) };
            if tile_loader.hflip() {
                write_4bpp_rev!(pixel_buffer, dx + 0, tile_palette, pixels8 >> 24);
                write_4bpp_rev!(pixel_buffer, dx + 2, tile_palette, pixels8 >> 16);
                write_4bpp_rev!(pixel_buffer, dx + 4, tile_palette, pixels8 >> 8);
                write_4bpp_rev!(pixel_buffer, dx + 6, tile_palette, pixels8);
            } else {
                write_4bpp!(pixel_buffer, dx + 0, tile_palette, pixels8);
                write_4bpp!(pixel_buffer, dx + 2, tile_palette, pixels8 >> 8);
                write_4bpp!(pixel_buffer, dx + 4, tile_palette, pixels8 >> 16);
                write_4bpp!(pixel_buffer, dx + 6, tile_palette, pixels8 >> 24);
            }
            dx += 8;
        } else {
            let mut pixel_offset = tile_loader.tile_pixel_offset(
                BYTES_PER_TILE,
                BYTES_PER_LINE,
                bg.char_base as usize,
                ty,
            );

            // get the x offset of the pixel:
            let tx = if tile_loader.hflip() {
                7 - (scx % 8)
            } else {
                scx % 8
            };
            pixel_offset += tx as usize / 2;

            let tile_palette = tile_loader.tile_palette();

            let palette_entry = (vram[pixel_offset as usize] >> ((tx % 2) << 2)) & 0xF;
            pixel_buffer[dx as usize] = (tile_palette * 16) + palette_entry;
            dx += 1;
        }
    }

    if bg.mosaic_x > 1 {
        // Fill each mosaic chunk with the first pixel in the chunk.
        pixel_buffer
            .chunks_mut(bg.mosaic_x as usize)
            .for_each(|chunk| {
                memset(chunk, chunk.first().copied().unwrap_or(0));
            });
    }

    let pixel_mask = bg.pixel_mask();

    if !pixels.windows.enabled {
        macro_rules! draw_entry_simple {
            ($Entry:expr, $DestOffset:expr) => {{
                let entry = $Entry as u8;
                if (entry & 0xF) != 0 {
                    pixels.push_pixel($DestOffset, Pixel(pixel_mask | (entry as u16)));
                }
            }};
        }

        let mut x = 0;
        while x < 240 {
            let entries8 = unsafe { read_u64_unchecked(&pixel_buffer[0..], x) };
            draw_entry_simple!(entries8, x);
            draw_entry_simple!(entries8 >> 8, x + 1);
            draw_entry_simple!(entries8 >> 16, x + 2);
            draw_entry_simple!(entries8 >> 24, x + 3);
            draw_entry_simple!(entries8 >> 32, x + 4);
            draw_entry_simple!(entries8 >> 40, x + 5);
            draw_entry_simple!(entries8 >> 48, x + 6);
            draw_entry_simple!(entries8 >> 56, x + 7);
            x += 8;
        }
    } else {
        macro_rules! draw_entry_windowed {
            ($Entry:expr, $DestOffset:expr) => {{
                let entry = $Entry as u8;
                if (entry & 0xF) != 0 {
                    if let Some(window_effects_mask) =
                        pixels.windows.check_pixel(bg.layer, $DestOffset)
                    {
                        pixels.push_pixel(
                            $DestOffset,
                            Pixel((pixel_mask & window_effects_mask) | (entry as u16)),
                        );
                    }
                }
            }};
        }

        let mut x = 0;
        while x < 240 {
            let entries8 = unsafe { read_u64_unchecked(&pixel_buffer[0..], x) };
            draw_entry_windowed!(entries8, x);
            draw_entry_windowed!(entries8 >> 8, x + 1);
            draw_entry_windowed!(entries8 >> 16, x + 2);
            draw_entry_windowed!(entries8 >> 24, x + 3);
            draw_entry_windowed!(entries8 >> 32, x + 4);
            draw_entry_windowed!(entries8 >> 40, x + 5);
            draw_entry_windowed!(entries8 >> 48, x + 6);
            draw_entry_windowed!(entries8 >> 56, x + 7);
            x += 8;
        }
    }
}

pub fn draw_text_bg_8bpp(line: u32, bg: &TextBG, vram: &VRAM, pixels: &mut LCDLineBuffer) {
    pub const BYTES_PER_TILE: usize = 64;
    pub const BYTES_PER_LINE: usize = 8;

    let start_scx = bg.wrapped_xoffset();
    let scy = bg.wrapped_yoffset_at_line(line);
    let ty = scy as usize % 8;

    let mut dx = 0;
    let mut pixel_buffer = [0u8; 240];
    let mut tile_loader = TileLoader::new(vram, bg.screen_base, start_scx, scy, bg.width);

    while dx < 240 {
        let scx = start_scx + dx;

        if scx % 8 == 0 {
            tile_loader.advance();
        }

        // try to do 8 pixels at a time if possible:
        if (scx % 8) == 0 && dx <= 232 {
            let pixel_offset = tile_loader.tile_pixel_offset(
                BYTES_PER_TILE,
                BYTES_PER_LINE,
                bg.char_base as usize,
                ty,
            );

            if pixel_offset < 0x10000 {
                let mut pixels8 = unsafe { read_u64_unchecked(vram, pixel_offset as usize) };
                if tile_loader.hflip() {
                    pixels8 = pixels8.swap_bytes();
                }
                // The bounds check is already done by dx <= 232
                unsafe { write_u64_unchecked(&mut pixel_buffer, dx as usize, pixels8) };
            }

            dx += 8;
        } else {
            let mut pixel_offset = tile_loader.tile_pixel_offset(
                BYTES_PER_TILE,
                BYTES_PER_LINE,
                bg.char_base as usize,
                ty,
            );

            if pixel_offset < 0x10000 {
                if tile_loader.hflip() {
                    pixel_offset += 7 - (scx as usize % 8);
                } else {
                    pixel_offset += scx as usize % 8;
                }
                let palette_entry = vram[pixel_offset as usize];
                pixel_buffer[dx as usize] = palette_entry;
            }
            dx += 1;
        }
    }

    if bg.mosaic_x > 1 {
        // Fill each mosaic chunk with the first pixel in the chunk.
        pixel_buffer
            .chunks_mut(bg.mosaic_x as usize)
            .for_each(|chunk| {
                memset(chunk, chunk.first().copied().unwrap_or(0));
            });
    }

    let pixel_mask = bg.pixel_mask();

    if !pixels.windows.enabled {
        for x in 0..240 {
            let entry = pixel_buffer[x];
            if entry == 0 {
                continue;
            }
            pixels.push_pixel(x, Pixel(pixel_mask | (entry as u16)));
        }
    } else {
        for x in 0..240 {
            let entry = pixel_buffer[x];
            if entry == 0 {
                continue;
            }
            if let Some(window_effects_mask) = pixels.windows.check_pixel(bg.layer, x) {
                pixels.push_pixel(
                    x,
                    Pixel((pixel_mask & window_effects_mask) | (entry as u16)),
                );
            }
        }
    }
}

pub struct AffineBG {
    layer: Layer,
    priority: u16,

    /// Base address of characters.
    char_base: u32,

    /// Base address for screens.
    screen_base: u32,

    width: u32,
    height: u32,

    params: AffineBGParams,
    wraparound: bool,

    mosaic_x: u32,
    mosaic_y: u32,

    first_target: bool,
    second_target: bool,
}

impl AffineBG {
    /// Internal Screen Size (dots) and size of BG Map (bytes):
    ///
    ///   Value  Rotation/Scaling Mode
    ///   0      128x128   (256 bytes)
    ///   1      256x256   (1K)
    ///   2      512x512   (4K)
    ///   3      1024x1024 (16K)
    const SIZES: [(u32, u32); 4] = [(128, 128), (256, 256), (512, 512), (1024, 1024)];

    pub fn new(
        layer: Layer,
        priority: u16,
        control: BGControl,
        params: AffineBGParams,
        reg_mosaic: super::Mosaic,
        first_target: bool,
        second_target: bool,
    ) -> AffineBG {
        let (width, height) = AffineBG::SIZES[control.screen_size() as usize];
        let mosaic = if control.mosaic() {
            reg_mosaic.bg
        } else {
            (0, 0)
        };

        AffineBG {
            layer: layer,
            priority: priority,
            char_base: control.char_base_block() as u32 * 16 * 1024,
            screen_base: control.screen_base_block() as u32 * 2 * 1024,
            width: width,
            height: height,
            params: params,
            wraparound: control.wraparound(),
            mosaic_x: mosaic.0 as u32,
            mosaic_y: mosaic.1 as u32,
            first_target: first_target,
            second_target: second_target,
        }
    }

    fn pixel_mask(&self) -> u16 {
        let first_target_mask = if self.first_target {
            Pixel::FIRST_TARGET
        } else {
            0
        };

        let second_target_mask = if self.second_target {
            Pixel::SECOND_TARGET
        } else {
            0
        };

        Pixel::layer_mask(self.layer)
            | Pixel::priority_mask(self.priority)
            | first_target_mask
            | second_target_mask
    }
}

pub struct TextBG {
    layer: Layer,
    priority: u16,

    /// Base address of characters.
    char_base: u32,
    /// Base address for screens.
    screen_base: u32,

    xoffset: u32,
    yoffset: u32,
    width: u32,
    height: u32,

    mosaic_x: u32,
    mosaic_y: u32,

    first_target: bool,
    second_target: bool,
}

impl TextBG {
    /// Internal Screen Size (dots) and size of BG Map (bytes):
    ///
    ///   Value  Text Mode
    ///   0      256x256 (2K)
    ///   1      512x256 (4K)
    ///   2      256x512 (4K)
    ///   3      512x512 (8K)
    const SIZES: [(u32, u32); 4] = [(256, 256), (512, 256), (256, 512), (512, 512)];

    pub fn new(
        layer: Layer,
        priority: u16,
        control: BGControl,
        offset: BGOffset,
        reg_mosaic: super::Mosaic,
        first_target: bool,
        second_target: bool,
    ) -> TextBG {
        let (width, height) = TextBG::SIZES[control.screen_size() as usize];
        let mosaic = if control.mosaic() {
            reg_mosaic.bg
        } else {
            (0, 0)
        };

        TextBG {
            layer: layer,
            priority: priority,
            char_base: control.char_base_block() as u32 * 16 * 1024,
            screen_base: control.screen_base_block() as u32 * 2 * 1024,
            xoffset: offset.x as u32,
            yoffset: offset.y as u32,
            width: width,
            height: height,
            mosaic_x: mosaic.0 as u32,
            mosaic_y: mosaic.1 as u32,
            first_target: first_target,
            second_target: second_target,
        }
    }

    // #[inline]
    // fn get_tile_info_offset(&self, scx: u32, scy: u32) -> u32 {
    //     let area_y = scy % 256;
    //     let area_ty = area_y / 8;
    //     let scx = scx & (self.width - 1); // @NOTE: this relies on bg.width being a power of 2
    //     let area_idx = (scy / 256) * (self.width / 256) + (scx / 256);
    //     let area_x = scx % 256;
    //     let area_tx = area_x / 8;
    //     return self.screen_base + (area_idx * 2048) + ((area_ty * 32) + area_tx) * 2;
    // }

    /// Returns the real X offset of this text mode background taking into account wrapping and
    /// the mosaic registers.
    fn wrapped_xoffset(&self) -> u32 {
        if self.mosaic_x > 0 {
            let original_scx = self.xoffset & (self.width - 1);
            original_scx - (original_scx % self.mosaic_x)
        } else {
            self.xoffset & (self.width - 1)
        }
    }

    /// Returns the real Y offset of this text mode background at a given line taking into
    /// account wrapping and the mosaic registers.
    fn wrapped_yoffset_at_line(&self, line: u32) -> u32 {
        if self.mosaic_y > 0 {
            let original_scy = (self.yoffset + line) & (self.height - 1);
            original_scy - (original_scy % self.mosaic_y)
        } else {
            (self.yoffset + line) & (self.height - 1)
        }
    }

    fn pixel_mask(&self) -> u16 {
        let first_target_mask = if self.first_target {
            Pixel::FIRST_TARGET
        } else {
            0
        };

        let second_target_mask = if self.second_target {
            Pixel::SECOND_TARGET
        } else {
            0
        };

        Pixel::layer_mask(self.layer)
            | Pixel::priority_mask(self.priority)
            | first_target_mask
            | second_target_mask
    }
}
