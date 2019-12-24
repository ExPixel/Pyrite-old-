pub mod bitmap;
pub mod obj;
pub mod palette;
pub mod tile;

use self::palette::GbaPalette;
use crate::hardware::{OAM, VRAM};
use crate::util::fixedpoint::{FixedPoint16, FixedPoint32};
use crate::GbaVideoOutput;
use pyrite_common::bits;

pub const OBJ_LAYER: u16 = 4;
pub const BD_LAYER: u16 = 5;

pub const HDRAW_CYCLES: u32 = 960;
pub const HBLANK_CYCLES: u32 = 272;

#[inline(always)]
const fn set_halfword_of_word(word: u32, value: u16, off: u32) -> u32 {
    let shift = (off as u32 & 0x10) << 3;
    (word & !(0xFFFF << shift)) | ((value as u32) << shift)
}

pub struct GbaLCD {
    pub(crate) registers: LCDRegisters,
    pub(crate) pixels: LCDLineBuffer,
    hblank: bool,
    next_state_cycles: u32,
    frame_ready: bool,
}

impl GbaLCD {
    pub fn new() -> GbaLCD {
        GbaLCD {
            registers: LCDRegisters::default(),
            hblank: false,
            next_state_cycles: HDRAW_CYCLES,
            pixels: LCDLineBuffer::new(),
            frame_ready: false,
        }
    }

    /// Returns true if the end of this step was also the end of a video frame.
    #[inline]
    pub fn step(
        &mut self,
        cycles: u32,
        vram: &VRAM,
        oam: &OAM,
        palette: &GbaPalette,
        video: &mut dyn GbaVideoOutput,
    ) -> bool {
        let original_cycles = self.next_state_cycles;
        self.next_state_cycles = self.next_state_cycles.saturating_sub(cycles);
        if self.next_state_cycles == 0 {
            self.hblank = !self.hblank;
            if self.hblank {
                self.next_state_cycles = HDRAW_CYCLES - (cycles - original_cycles);
                return self.hblank(vram, oam, palette, video);
            } else {
                self.next_state_cycles = HBLANK_CYCLES - (cycles - original_cycles);
                self.hdraw();
            }
        }
        return false;
    }

    fn hdraw(&mut self) {
        self.registers.dispstat.set_hblank(false);
        self.registers.line += 1;

        match self.registers.line {
            160 => self.registers.dispstat.set_vblank(true),
            227 => self.registers.dispstat.set_vblank(false),
            228 => self.registers.line = 0,
            _ => { /* NOP */ }
        }

        let vcounter_match = self.registers.dispstat.vcounter_setting() == self.registers.line;
        self.registers.dispstat.set_vcounter(vcounter_match);
    }

    /// Returns true if this is the end of a frame.
    fn hblank(
        &mut self,
        vram: &VRAM,
        oam: &OAM,
        palette: &GbaPalette,
        video: &mut dyn GbaVideoOutput,
    ) -> bool {
        self.registers.dispstat.set_hblank(true);

        if self.registers.line < 160 {
            if self.registers.line == 0 {
                video.pre_frame();
            }
            self.draw_line(vram, oam, palette);
            self.pixels
                .mix(self.registers.effects.effect(), &self.registers);
            video.display_line(self.registers.line as u32, &self.pixels.mixed);
            if self.registers.line == 159 {
                video.post_frame();
                self.frame_ready = true;
            }
        }

        return self.registers.line == 159;
    }

    fn draw_line(&mut self, vram: &VRAM, oam: &OAM, palette: &GbaPalette) {
        // setup obj cycles:
        self.pixels.obj_cycles = if self.registers.dispcnt.hblank_interval_free() {
            954
        } else {
            1210
        };

        self.pixels.clear_flags();
        // setting up the backdrop:
        let backdrop = palette.backdrop();
        for x in 0..240 {
            self.pixels.push_pixel_fast(x, backdrop);
        }

        let mode = self.registers.dispcnt.mode();

        match mode {
            0 => tile::render_mode0(&self.registers, vram, oam, palette, &mut self.pixels),
            1 => tile::render_mode1(&self.registers, vram, oam, palette, &mut self.pixels),
            2 => tile::render_mode2(&self.registers, vram, oam, palette, &mut self.pixels),
            3 => bitmap::render_mode3(&self.registers, vram, oam, palette, &mut self.pixels),
            4 => bitmap::render_mode4(&self.registers, vram, oam, palette, &mut self.pixels),
            5 => bitmap::render_mode5(&self.registers, vram, oam, palette, &mut self.pixels),
            _ => eprintln!("bad mode {}", mode),
        }
    }
}

pub struct LCDLineBuffer {
    mixed: [u16; 240],
    unmixed: [u32; 240],
    obj_window: LCDPixelBits,
    obj_semitrans: LCDPixelBits,

    top_layer_first_target: LCDPixelBits,
    top_layer_second_target: LCDPixelBits,
    bot_layer_second_target: LCDPixelBits,

    /// Cycles remaining for drawing objects.
    pub(crate) obj_cycles: u16,
}

impl LCDLineBuffer {
    pub const fn new() -> LCDLineBuffer {
        LCDLineBuffer {
            mixed: [0xFFFF; 240],
            unmixed: [0x0000; 240],
            obj_window: LCDPixelBits::new(),
            obj_semitrans: LCDPixelBits::new(),
            obj_cycles: 1210,

            top_layer_first_target: LCDPixelBits::new(),
            top_layer_second_target: LCDPixelBits::new(),
            bot_layer_second_target: LCDPixelBits::new(),
        }
    }

    #[inline]
    pub fn clear_flags(&mut self) {
        self.obj_window.clear_all();
        self.top_layer_first_target.clear_all();
        self.bot_layer_second_target.clear_all();
        self.bot_layer_second_target.clear_all();
    }

    // @TODO remove this
    #[deprecated]
    #[inline]
    pub fn push_pixel_fast(&mut self, index: usize, color: u16) {
        self.unmixed[index] = (self.unmixed[index] << 16) | (color as u32);
        // self.pixels[index] = color;
    }

    #[inline]
    pub fn push_pixel(
        &mut self,
        index: usize,
        color: u16,
        first_target: bool,
        second_target: bool,
        semi_trans: bool,
    ) {
        if self.top_layer_second_target.get(index) {
            self.unmixed[index] = (self.unmixed[index] << 16) | (color as u32);
            self.bot_layer_second_target.set(index);
        } else {
            // Since this for sure overwrites the bottom pixel anyway with no mixing, there's no
            // point in doing the above.
            self.unmixed[index] = color as u32;
            self.bot_layer_second_target.clear(index);
        }

        self.top_layer_first_target
            .put(index, first_target | semi_trans);
        self.top_layer_second_target.put(index, second_target);
        self.obj_semitrans.put(index, semi_trans);
    }

    pub fn mix(&mut self, effect: SpecialEffect, registers: &LCDRegisters) {
        let eva = bits!(registers.alpha, 0, 4); // EVA * 16
        let evb = bits!(registers.alpha, 8, 12); // EVB * 16

        let no_blending = (effect == SpecialEffect::None && self.obj_semitrans.is_all_zeroes())
            || (effect == SpecialEffect::AlphaBlending
                && (self.bot_layer_second_target.is_all_zeroes() || eva == 16))
            || self.top_layer_first_target.is_all_zeroes();

        if no_blending {
            // if we're not blending be can just copy the top most pixel into the mixed line
            // buffer.
            self.unmixed
                .iter()
                .zip(self.mixed.iter_mut())
                .for_each(|(&s, d)| {
                    *d = s as u16;
                });
            return;
        }

        match effect {
            SpecialEffect::AlphaBlending => {
                for index in 0..240 {
                    // for alpha blending, we only blend if there is both a first and second
                    // target:
                    if !self.top_layer_first_target.get(index)
                        || !self.bot_layer_second_target.get(index)
                    {
                        self.mixed[index] = self.unmixed[index] as u16;
                        continue;
                    }

                    self.mixed[index] = Self::alpha_blend(
                        self.unmixed[index] as u16,
                        (self.unmixed[index] >> 16) as u16,
                        eva,
                        evb,
                    );
                }
            }

            SpecialEffect::BrightnessIncrease => {
                let evy = bits!(registers.brightness, 0, 4); // EVY * 16

                for index in 0..240 {
                    // for brightness blending, we only blend if there is a first target pixel in
                    // the top layer:
                    if !self.top_layer_first_target.get(index) {
                        self.mixed[index] = self.unmixed[index] as u16;
                        continue;
                    }

                    if self.obj_semitrans.get(index) {
                        // implies first target
                        if self.bot_layer_second_target.get(index) {
                            self.mixed[index] = Self::alpha_blend(
                                self.unmixed[index] as u16,
                                (self.unmixed[index] >> 16) as u16,
                                eva,
                                evb,
                            );
                        } else {
                            self.mixed[index] = self.unmixed[index] as u16;
                        }
                        continue;
                    }

                    self.mixed[index] = Self::brightness_increase(self.unmixed[index] as u16, evy);
                }
            }

            SpecialEffect::BrightnessDecrease => {
                let evy = bits!(registers.brightness, 0, 4); // EVY * 16

                for index in 0..240 {
                    // for brightness blending, we only blend if there is a first target pixel in
                    // the top layer:
                    if !self.top_layer_first_target.get(index) {
                        self.mixed[index] = self.unmixed[index] as u16;
                        continue;
                    }

                    if self.obj_semitrans.get(index) {
                        // implies first target
                        if self.bot_layer_second_target.get(index) {
                            self.mixed[index] = Self::alpha_blend(
                                self.unmixed[index] as u16,
                                (self.unmixed[index] >> 16) as u16,
                                eva,
                                evb,
                            );
                        } else {
                            self.mixed[index] = self.unmixed[index] as u16;
                        }
                        continue;
                    }

                    self.mixed[index] = Self::brightness_decrease(self.unmixed[index] as u16, evy);
                }
            }

            // we would only reach here if there are semi-transparent objects.
            SpecialEffect::None => {
                for index in 0..240 {
                    if !self.obj_semitrans.get(index) || !self.bot_layer_second_target.get(index) {
                        self.mixed[index] = self.unmixed[index] as u16;
                    } else {
                        self.mixed[index] = Self::alpha_blend(
                            self.unmixed[index] as u16,
                            (self.unmixed[index] >> 16) as u16,
                            eva,
                            evb,
                        );
                    }
                }
            }
        }
    }

    fn alpha_blend(first: u16, second: u16, eva: u16, evb: u16) -> u16 {
        // I = MIN ( 31, I1st*EVA + I2nd*EVB )
        // where I is the separate R, G, and B components
        let (r1, g1, b1) = pixel_components(first);
        let (r2, g2, b2) = pixel_components(second);
        let r = std::cmp::min(31, (r1 * eva) / 16 + (r2 * evb) / 16);
        let g = std::cmp::min(31, (g1 * eva) / 16 + (g2 * evb) / 16);
        let b = std::cmp::min(31, (b1 * eva) / 16 + (b2 * evb) / 16);
        rgb16(r, g, b)
    }

    fn brightness_increase(color: u16, evy: u16) -> u16 {
        // I = I1st + (31-I1st)*EVY
        // where I is the separate R, G, and B components
        let (r1, g1, b1) = pixel_components(color);
        let r = std::cmp::min(31, r1 + ((31 - r1) * evy) / 16);
        let g = std::cmp::min(31, g1 + ((31 - g1) * evy) / 16);
        let b = std::cmp::min(31, b1 + ((31 - b1) * evy) / 16);
        rgb16(r, g, b)
    }

    fn brightness_decrease(color: u16, evy: u16) -> u16 {
        // I = I1st - (I1st)*EVY
        // where I is the separate R, G, and B components
        let (r1, g1, b1) = pixel_components(color);
        let r = std::cmp::min(31, r1 - (r1 * evy) / 16);
        let g = std::cmp::min(31, g1 - (g1 * evy) / 16);
        let b = std::cmp::min(31, b1 - (b1 * evy) / 16);
        rgb16(r, g, b)
    }
}

/// A bit vector with a bit associated with each pixel of the LCD.
pub(crate) struct LCDPixelBits {
    bits: [u64; 4],
}

impl LCDPixelBits {
    pub const fn new() -> LCDPixelBits {
        LCDPixelBits { bits: [0, 0, 0, 0] }
    }

    #[inline(always)]
    pub fn set(&mut self, bit: usize) {
        let index = bit / 64;
        let shift = bit as u64 % 64;
        self.bits[index] |= 1 << shift;
    }

    #[inline(always)]
    pub fn clear(&mut self, bit: usize) {
        let index = bit / 64;
        let shift = bit as u64 % 64;
        self.bits[index] &= !(1 << shift);
    }

    #[inline(always)]
    pub fn put(&mut self, bit: usize, value: bool) {
        if value {
            self.set(bit);
        } else {
            self.clear(bit);
        }
    }

    #[inline(always)]
    pub fn get(&self, bit: usize) -> bool {
        let index = bit / 64;
        let shift = bit as u64 % 64;

        ((self.bits[index] >> shift) & 1) != 0
    }

    #[inline]
    pub fn clear_all(&mut self) {
        for b in self.bits.iter_mut() {
            *b = 0
        }
    }

    #[inline]
    pub fn is_all_zeroes(&self) -> bool {
        self.bits.iter().fold(0, |acc, &v| acc | v) == 0
    }
}

#[derive(Default)]
pub struct LCDRegisters {
    /// The current line being rendered by the LCD.
    pub line: u16,

    /// LCD control register.
    pub dispcnt: DisplayControl,

    /// LCD status register.
    pub dispstat: DisplayStatus,

    // @TODO implement whatever this is.
    pub greenswap: u16,

    // Background Control Registers:
    pub bg_cnt: [BGControl; 4],
    pub bg_ofs: [BGOffset; 4],

    // LCD BG Rotation / Scaling:
    pub bg2_affine_params: AffineBGParams,
    pub bg3_affine_params: AffineBGParams,

    // LCD Windows:
    pub win0_bounds: WindowBounds,
    pub win1_bounds: WindowBounds,
    pub winin: WindowControl,
    pub winout: WindowControl,

    // Special Effects
    pub mosaic: Mosaic,
    pub effects: EffectsSelection,
    pub alpha: u16,
    pub brightness: u16,
}

impl LCDRegisters {
    #[inline(always)]
    pub fn set_dispstat(&mut self, value: u16) {
        pub const DISPSTAT_WRITEABLE: u16 = 0xFFB4;
        self.dispstat.value =
            (self.dispstat.value & !DISPSTAT_WRITEABLE) | (value & DISPSTAT_WRITEABLE);
    }
}

bitfields! (DisplayStatus: u16 {
    vblank, set_vblank: bool = [0, 0],
    hblank, set_hblank: bool = [1, 1],
    vcounter, set_vcounter: bool = [2, 2],

    vblank_irq_enable, set_vblank_irq_enable: bool = [3, 3],
    hblank_irq_enable, set_hblank_irq_enable: bool = [4, 4],
    vcounter_irq_enable, set_vcounter_irq_enable: bool = [5, 5],

    vcounter_setting, set_vcounter_setting: u16 = [8, 15],
});

bitfields! (DisplayControl: u16 {
    mode, set_mode: u16 = [0, 2],
    frame_select, set_frame_select: u16 = [4, 4],
    hblank_interval_free, set_hblank_interval_free: bool = [5, 5],
    one_dimensional_obj, set_one_dimensional_obj: bool = [6, 6],
    forced_blank, set_forced_blank: bool = [7, 7],

    display_window0, set_display_window0: bool = [13, 13],
    display_window1, set_display_window1: bool = [14, 14],
    display_window_obj, set_display_window_obj: bool = [15, 15],

    windows_enabled, set_windows_enabled: bool = [13, 15],
});

bitfields! (BGControl: u16 {
    priority, set_priority: u16 = [0, 1],
    char_base_block, set_char_base_block: u16 = [2, 3],
    mosaic, set_mosaic: bool = [6, 6],
    palette256, set_palette256: bool = [7, 7],
    screen_base_block, set_screen_base_block: u16 = [8, 12],
    wraparound, set_wraparound: bool = [13, 13],
    screen_size, set_screen_size: u16 = [14, 15],
});

impl DisplayControl {
    /// Layers 0-3 are BG 0-3 respectively. Layer 4 is OBJ
    pub fn display_layer(&self, layer: u16) -> bool {
        assert!(layer <= 4, "display layer index must be in range [0, 4]");
        ((self.value >> (layer + 8)) & 1) != 0
    }
}

#[derive(Default)]
pub struct EffectsSelection {
    inner: u16,
}

impl EffectsSelection {
    #[inline(always)]
    pub fn value(&self) -> u16 {
        self.inner
    }
    #[inline(always)]
    pub fn set_value(&mut self, value: u16) {
        self.inner = value;
    }

    pub fn is_first_target(&self, layer: u16) -> bool {
        assert!(
            layer <= 5,
            "first target layer index must be in range [0, 5]"
        );
        return (self.inner & (1 << layer)) != 0;
    }

    pub fn is_second_target(&self, layer: u16) -> bool {
        assert!(
            layer <= 5,
            "second target layer index must be in range [0, 5]"
        );
        return (self.inner & (1 << (layer + 8))) != 0;
    }

    pub fn effect(&self) -> SpecialEffect {
        match bits!(self.inner, 6, 7) {
            0 => SpecialEffect::None,
            1 => SpecialEffect::AlphaBlending,
            2 => SpecialEffect::BrightnessIncrease,
            3 => SpecialEffect::BrightnessDecrease,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum SpecialEffect {
    None,
    AlphaBlending,
    BrightnessIncrease,
    BrightnessDecrease,
}

#[derive(Default, Clone, Copy)]
pub struct Mosaic {
    pub bg: (/* horizontal */ u8, /* vertical */ u8),
    pub obj: (/* horizontal */ u8, /* vertical */ u8),
}

impl Mosaic {
    pub fn set_value(&mut self, value: u16) {
        self.bg.0 = (bits!(value, 0, 3) + 1) as u8;
        self.bg.1 = (bits!(value, 4, 7) + 1) as u8;
        self.obj.0 = (bits!(value, 8, 11) + 1) as u8;
        self.obj.1 = (bits!(value, 12, 15) + 1) as u8;
    }
}

#[derive(Default, Copy, Clone)]
pub struct BGOffset {
    pub x: u16,
    pub y: u16,
}

impl BGOffset {
    #[inline(always)]
    pub fn set_x(&mut self, x: u16) {
        self.x = x & 0x1FF;
    }

    #[inline(always)]
    pub fn set_y(&mut self, y: u16) {
        self.y = y & 0x1FF;
    }
}

#[derive(Default)]
pub struct AffineBGParams {
    pub internal_x: FixedPoint32,
    pub internal_y: FixedPoint32,

    pub a: FixedPoint32,
    pub b: FixedPoint32,
    pub c: FixedPoint32,
    pub d: FixedPoint32,

    pub x: u32,
    pub y: u32,
}

impl AffineBGParams {
    /// Copies the reference point registers into the internal reference point registers.
    #[inline]
    pub fn copy_reference_points(&mut self) {
        self.internal_x = FixedPoint32::wrap(((self.x as i32) << 4) >> 4);
        self.internal_y = FixedPoint32::wrap(((self.y as i32) << 4) >> 4);
    }

    #[inline]
    pub fn set_a(&mut self, value: u16) {
        self.a = FixedPoint32::from(FixedPoint16::wrap(value as i16));
    }
    #[inline]
    pub fn set_b(&mut self, value: u16) {
        self.b = FixedPoint32::from(FixedPoint16::wrap(value as i16));
    }
    #[inline]
    pub fn set_c(&mut self, value: u16) {
        self.c = FixedPoint32::from(FixedPoint16::wrap(value as i16));
    }
    #[inline]
    pub fn set_d(&mut self, value: u16) {
        self.d = FixedPoint32::from(FixedPoint16::wrap(value as i16));
    }
}

#[derive(Default)]
pub struct WindowBounds {
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
}

impl WindowBounds {
    #[inline(always)]
    pub fn set_left(&mut self, left: u16) {
        self.left = std::cmp::min(left, 240);
    }

    #[inline(always)]
    pub fn set_right(&mut self, right: u16) {
        self.right = std::cmp::min(right, 240);
    }

    #[inline(always)]
    pub fn set_top(&mut self, top: u16) {
        self.top = std::cmp::min(top, 160);
    }

    #[inline(always)]
    pub fn set_bottom(&mut self, bottom: u16) {
        self.bottom = std::cmp::min(bottom, 160);
    }
}

#[derive(Default)]
pub struct WindowControl {
    inner: u16,
}

impl WindowControl {
    pub fn set_value(&mut self, value: u16) {
        self.inner = value;
    }

    #[inline(always)]
    pub fn value(&self) -> u16 {
        self.inner
    }

    /// Returns true if the given background or OBJ layer (layer #4) is enabled in the given window
    /// (0 or 1). #NOTE That if this window control is for WINOUT, window 0 is the outside window
    /// and window 1 is the OBJ window.
    #[inline(always)]
    pub fn enabled(&self, window: u16, background: u16) -> bool {
        (self.inner & (self.inner << (background + (window * 8)))) != 0
    }

    /// Returns true if color special effects is enabled for a given window.
    pub fn effects_enabled(&self, window: u16) -> bool {
        (self.inner & (self.inner << (5 + (window * 8)))) != 0
    }
}

#[inline]
pub fn apply_mosaic(value: u32, mosaic: u32) -> u32 {
    if mosaic > 1 {
        return value - (value % mosaic);
    } else {
        return value;
    }
}

#[inline]
pub fn apply_mosaic_cond(mosaic_cond: bool, value: u16, mosaic: u16) -> u16 {
    if mosaic_cond && mosaic > 0 {
        return value - (value % mosaic);
    } else {
        return value;
    }
}

#[inline(always)]
pub fn pixel_components(pixel: u16) -> (u16, u16, u16) {
    (pixel & 0x1F, (pixel >> 5) & 0x1F, (pixel >> 10) & 0x1F)
}

#[inline(always)]
pub fn rgb16(r: u16, g: u16, b: u16) -> u16 {
    (r & 0x1F) | ((g & 0x1F) << 5) | ((b & 0x1F) << 10) | 0x8000
}
