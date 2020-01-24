pub mod bitmap;
pub mod obj;
pub mod palette;
pub mod tile;

pub const WINOUT: u16 = 0;
pub const WINOBJ: u16 = 1;

use self::palette::GbaPalette;
use crate::dma::GbaDMA;
use crate::hardware::{HardwareEventQueue, OAM, VRAM};
use crate::irq::Interrupt;
use crate::util::fixedpoint::{FixedPoint16, FixedPoint32};
use crate::GbaVideoOutput;
use pyrite_common::bits;

pub const OBJ_LAYER: u16 = 4;
pub const BD_LAYER: u16 = 5;

pub const HDRAW_CYCLES: u32 = 960;
pub const HBLANK_CYCLES: u32 = 272;

pub struct GbaLCD {
    pub(crate) registers: LCDRegisters,
    pub(crate) pixels: LCDLineBuffer,
    hblank: bool,
    next_state_cycles: u32,
}

impl GbaLCD {
    pub fn new() -> GbaLCD {
        GbaLCD {
            registers: LCDRegisters::default(),
            hblank: false,
            next_state_cycles: HDRAW_CYCLES,
            pixels: LCDLineBuffer::new(),
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
        dma: &mut GbaDMA,
        hw_events: &mut HardwareEventQueue,
    ) -> bool {
        let original_cycles = self.next_state_cycles;
        self.next_state_cycles = self.next_state_cycles.saturating_sub(cycles);
        if self.next_state_cycles == 0 {
            self.hblank = !self.hblank;
            if self.hblank {
                self.next_state_cycles = HDRAW_CYCLES - (cycles - original_cycles);
                return self.hblank(vram, oam, palette, video, dma, hw_events);
            } else {
                self.next_state_cycles = HBLANK_CYCLES - (cycles - original_cycles);
                self.hdraw(dma, hw_events);
            }
        }
        return false;
    }

    fn hdraw(&mut self, dma: &mut GbaDMA, hw_events: &mut HardwareEventQueue) {
        self.registers.dispstat.set_hblank(false);
        self.registers.line += 1;

        match self.registers.line {
            160 => {
                dma.start_vblank(hw_events);
                self.registers.dispstat.set_vblank(true);
                self.registers.bg2_affine_params.copy_reference_points();
                self.registers.bg3_affine_params.copy_reference_points();

                if self.registers.dispstat.vblank_irq_enable() {
                    hw_events.push_irq_event(Interrupt::LCDVBlank);
                }
            }
            227 => self.registers.dispstat.set_vblank(false),
            228 => self.registers.line = 0,
            _ => { /* NOP */ }
        }

        let vcounter_match = self.registers.dispstat.vcounter_setting() == self.registers.line;
        self.registers.dispstat.set_vcounter(vcounter_match);

        if vcounter_match && self.registers.dispstat.vcounter_irq_enable() {
            hw_events.push_irq_event(Interrupt::LCDVCounterMatch);
        }
    }

    /// Returns true if this is the end of a frame.
    fn hblank(
        &mut self,
        vram: &VRAM,
        oam: &OAM,
        palette: &GbaPalette,
        video: &mut dyn GbaVideoOutput,
        dma: &mut GbaDMA,
        hw_events: &mut HardwareEventQueue,
    ) -> bool {
        if self.registers.dispstat.hblank_irq_enable() {
            hw_events.push_irq_event(Interrupt::LCDHBlank);
        }
        self.registers.dispstat.set_hblank(true);

        if self.registers.line < 160 {
            dma.start_hblank(hw_events); // NOTE: this does not occure during VBLANK
            if self.registers.line == 0 {
                video.pre_frame();
            }
            self.draw_line(vram, oam);
            self.pixels
                .mix(palette, self.registers.effects.effect(), &self.registers);
            video.display_line(self.registers.line as u32, &self.pixels.mixed);
            if self.registers.line == 159 {
                video.post_frame();
            }
        }

        return self.registers.line == 159;
    }

    fn draw_line(&mut self, vram: &VRAM, oam: &OAM) {
        // setup obj cycles:
        self.pixels.obj_cycles = if self.registers.dispcnt.hblank_interval_free() {
            954
        } else {
            1210
        };

        // setting up the backdrop:
        let backdrop = Pixel(Pixel::layer_mask(Layer::Backdrop) | 0);
        self.pixels.clear(backdrop);

        let mode = self.registers.dispcnt.mode();
        self.pixels.windows = WindowInfo {
            enabled: self.registers.dispcnt.windows_enabled(),
            win0_enabled: self.registers.dispcnt.display_window0(),
            win1_enabled: self.registers.dispcnt.display_window1(),
            win_obj_enabled: self.registers.dispcnt.display_window_obj(),
            win0_bounds: self.registers.win0_bounds,
            win1_bounds: self.registers.win1_bounds,
            winin: self.registers.winin,
            winout: self.registers.winout,

            // @TODO
            obj_window: LCDPixelBits::new(),

            window_effects_masks: [
                Pixel::window_effects_mask(self.registers.winin.effects_enabled(Window::Win0)),
                Pixel::window_effects_mask(self.registers.winin.effects_enabled(Window::Win1)),
                Pixel::window_effects_mask(self.registers.winout.effects_enabled(Window::Outside)),
                Pixel::window_effects_mask(self.registers.winout.effects_enabled(Window::OBJ)),
            ],
        };

        match mode {
            0 => tile::render_mode0(&self.registers, vram, oam, &mut self.pixels),
            1 => tile::render_mode1(&mut self.registers, vram, oam, &mut self.pixels),
            2 => tile::render_mode2(&mut self.registers, vram, oam, &mut self.pixels),
            3 => bitmap::render_mode3(&self.registers, vram, oam, &mut self.pixels),
            4 => bitmap::render_mode4(&self.registers, vram, oam, &mut self.pixels),
            5 => bitmap::render_mode5(&self.registers, vram, oam, &mut self.pixels),
            _ => log::warn!("bad mode {}", mode),
        }
    }
}

pub struct LCDLineBuffer {
    mixed: [u16; 240],
    unmixed: [Pixels2; 240],
    bitmap_palette: [u16; 240],
    windows: WindowInfo,
    /// Cycles remaining for drawing objects.
    pub(crate) obj_cycles: u16,
}

impl LCDLineBuffer {
    pub const fn new() -> LCDLineBuffer {
        LCDLineBuffer {
            mixed: [0xFFFF; 240],
            unmixed: [Pixels2::zero(); 240],
            bitmap_palette: [0; 240],
            obj_cycles: 1210,
            windows: WindowInfo::new(),
        }
    }

    pub fn clear(&mut self, clear_pixel: Pixel) {
        self.unmixed.iter_mut().for_each(|p| p.set(clear_pixel));
    }

    #[inline]
    pub fn push_bitmap_pixel(&mut self, index: usize, pixel_metadata: Pixel, color: u16) {
        self.unmixed[index].push(Pixel(pixel_metadata.0 | (index as u8 as u16)));
        self.bitmap_palette[index] = color;
    }

    // @TODO rename this to push_pixel when ready:
    #[inline]
    pub fn push_pixel(&mut self, index: usize, pix: Pixel) {
        self.unmixed[index].push(pix);
    }

    #[inline(always)]
    pub fn lookup_color(&self, palette: &GbaPalette, bitmap16: bool, pixel: Pixel) -> u16 {
        if bitmap16 && pixel.layer() == Layer::BG2 {
            self.bitmap_palette[pixel.pal_index()]
        } else if pixel.layer() != Layer::OBJ {
            palette.bg256(pixel.pal_index())
        } else {
            palette.obj256(pixel.pal_index())
        }
    }

    #[inline(always)]
    pub fn mix(&mut self, palette: &GbaPalette, effect: SpecialEffect, registers: &LCDRegisters) {
        let bm_color = registers.dispcnt.mode() == 3 || registers.dispcnt.mode() == 5;
        if bm_color {
            self.internal_mix_bitmap(palette, effect, registers);
        } else {
            self.internal_mix_tile(palette, effect, registers);
        }
    }

    fn internal_mix_bitmap(
        &mut self,
        palette: &GbaPalette,
        effect: SpecialEffect,
        registers: &LCDRegisters,
    ) {
        self.internal_mix(palette, effect, registers, true)
    }

    fn internal_mix_tile(
        &mut self,
        palette: &GbaPalette,
        effect: SpecialEffect,
        registers: &LCDRegisters,
    ) {
        self.internal_mix(palette, effect, registers, false)
    }

    // This is inlined into `internal_mix_bitmap` and `internal_mix_tile` in order to simplify the
    // lookup color calls where possible.
    #[inline(always)]
    fn internal_mix(
        &mut self,
        palette: &GbaPalette,
        effect: SpecialEffect,
        registers: &LCDRegisters,
        bm_color: bool,
    ) {
        let eva = bits!(registers.alpha, 0, 4); // EVA * 16
        let evb = bits!(registers.alpha, 8, 12); // EVB * 16

        match effect {
            SpecialEffect::AlphaBlending => {
                for x in 0..240 {
                    let Pixels2 { top, bot } = self.unmixed[x];
                    if !top.first_target() || !bot.second_target() {
                        self.mixed[x] = self.lookup_color(palette, bm_color, top);
                    } else {
                        self.mixed[x] = Self::alpha_blend(
                            self.lookup_color(palette, bm_color, top),
                            self.lookup_color(palette, bm_color, bot),
                            eva,
                            evb,
                        );
                    }
                }
            }

            SpecialEffect::BrightnessIncrease => {
                let evy = bits!(registers.brightness, 0, 4); // EVY * 16

                for x in 0..240 {
                    let Pixels2 { top, bot } = self.unmixed[x];

                    // If the top pixel is not a first target pixel, we don't bother trying to do
                    // any blending.
                    if !top.first_target() {
                        self.mixed[x] = self.lookup_color(palette, bm_color, top);
                        continue;
                    }

                    // Semi-transparent object pixels can only use alpha transparency:
                    if top.semi_transparent() {
                        if bot.second_target() {
                            self.mixed[x] = Self::alpha_blend(
                                self.lookup_color(palette, bm_color, top),
                                self.lookup_color(palette, bm_color, bot),
                                eva,
                                evb,
                            );
                        } else {
                            self.mixed[x] = self.lookup_color(palette, bm_color, top);
                        }
                        continue;
                    }

                    self.mixed[x] =
                        Self::brightness_increase(self.lookup_color(palette, bm_color, top), evy);
                }
            }

            SpecialEffect::BrightnessDecrease => {
                let evy = bits!(registers.brightness, 0, 4); // EVY * 16

                for x in 0..240 {
                    let Pixels2 { top, bot } = self.unmixed[x];

                    // If the top pixel is not a first target pixel, we don't bother trying to do
                    // any blending.
                    if !top.first_target() {
                        self.mixed[x] = self.lookup_color(palette, bm_color, top);
                        continue;
                    }

                    // Semi-transparent object pixels can only use alpha transparency:
                    if top.semi_transparent() {
                        if bot.second_target() {
                            self.mixed[x] = Self::alpha_blend(
                                self.lookup_color(palette, bm_color, top),
                                self.lookup_color(palette, bm_color, bot),
                                eva,
                                evb,
                            );
                        } else {
                            self.mixed[x] = self.lookup_color(palette, bm_color, top);
                        }
                        continue;
                    }

                    self.mixed[x] =
                        Self::brightness_decrease(self.lookup_color(palette, bm_color, top), evy);
                }
            }

            // handled at the top
            SpecialEffect::None => {
                // if we're not blending be can just copy the top most pixel into the mixed line
                // buffer.
                for x in 0..240 {
                    let Pixels2 { top, bot: _bot } = self.unmixed[x];
                    self.mixed[x] = self.lookup_color(palette, bm_color, top);
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

#[derive(Clone, Copy)]
pub struct Pixels2 {
    top: Pixel,
    bot: Pixel,
}

impl Pixels2 {
    pub const fn zero() -> Pixels2 {
        Pixels2 {
            top: Pixel(0),
            bot: Pixel(0),
        }
    }

    #[inline(always)]
    pub fn push(&mut self, new_top: Pixel) {
        self.bot = self.top;
        self.top = new_top;
    }

    #[inline(always)]
    pub fn set(&mut self, p: Pixel) {
        self.top = p;
        self.bot = Pixel(0);
    }
}

impl Default for Pixels2 {
    fn default() -> Pixels2 {
        Self::zero()
    }
}

/// Contains a pixel's color and some metadata:
///
/// BITS:
/// * 0 -  7: Color palette entry. If this is a pixel from a bitmap mode it will be an index into the
///           bitmap palette.
/// * 8 - 10: The layer of this pixel.
/// *     11: If this is 1, this is a first target pixel.
/// *     12: If this is 1, this is a second target pixel.
/// *     13: If this is 1, this is a semi-transparent OBJ pixel.
#[derive(Clone, Copy)]
pub struct Pixel(pub u16);

impl Pixel {
    pub const FIRST_TARGET: u16 = 0x0800;
    pub const SECOND_TARGET: u16 = 0x1000;
    pub const SEMI_TRANSPARENT: u16 = 0x2000;

    /// When this mask is ANDed with a pixel. It will either leave the FIRST_TARGET and
    /// SECOND_TARGET bits alone or remove them completely if special effects are disabled by a
    /// window.
    pub fn window_effects_mask(enabled: bool) -> u16 {
        if enabled {
            0xFFFF
        } else {
            !(Self::FIRST_TARGET | Self::SECOND_TARGET)
        }
    }

    #[inline(always)]
    pub const fn layer_mask(layer: Layer) -> u16 {
        ((layer as u16) & 0b0111) << 8
    }

    pub const fn pal_index(self) -> usize {
        self.0 as u8 as usize
    }

    #[inline(always)]
    pub const fn first_target(self) -> bool {
        (self.0 & Self::FIRST_TARGET) != 0
    }

    #[inline(always)]
    pub const fn second_target(self) -> bool {
        (self.0 & Self::SECOND_TARGET) != 0
    }

    #[inline(always)]
    pub const fn semi_transparent(self) -> bool {
        (self.0 & Self::SEMI_TRANSPARENT) != 0
    }

    // @TODO mark this as const when `Layer::from_index` becomes const.
    #[inline(always)]
    pub fn layer(self) -> Layer {
        unsafe { Layer::from_index_unsafe((self.0 >> 8) & 0b0111) }
    }
}

/// A bit vector with a bit associated with each pixel of the LCD.
pub struct LCDPixelBits {
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
    pub fn set_all(&mut self) {
        for b in self.bits.iter_mut() {
            *b = !0;
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
        pub const DISPSTAT_WRITEABLE: u16 = 0xFFB8;
        self.dispstat.value =
            (self.dispstat.value & !DISPSTAT_WRITEABLE) | (value & DISPSTAT_WRITEABLE);
    }
}

#[repr(u16)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    BG0 = 0,
    BG1 = 1,
    BG2 = 2,
    BG3 = 3,
    OBJ = 4,
    Backdrop = 5,
}

impl Layer {
    #[inline(always)]
    pub const fn index(self) -> u16 {
        self as u16
    }

    // @TODO this can be made const when `match` is enabled in const fns in stable Rust.
    pub fn from_bg(bg_index: u16) -> Layer {
        match bg_index {
            0 => Layer::BG0,
            1 => Layer::BG1,
            2 => Layer::BG2,
            3 => Layer::BG3,
            _ => panic!("Invalid BG layer."),
        }
    }

    // @TODO this can be made const when `match` is enabled in const fns in stable Rust.
    pub fn from_index(index: u16) -> Layer {
        match index {
            0 => Layer::BG0,
            1 => Layer::BG1,
            2 => Layer::BG2,
            3 => Layer::BG3,
            4 => Layer::OBJ,
            5 => Layer::Backdrop,
            _ => panic!("Invalid layer."),
        }
    }

    pub unsafe fn from_index_unsafe(index: u16) -> Layer {
        match index {
            0 => Layer::BG0,
            1 => Layer::BG1,
            2 => Layer::BG2,
            3 => Layer::BG3,
            4 => Layer::OBJ,
            5 => Layer::Backdrop,
            _ => std::hint::unreachable_unchecked(),
        }
    }
}

#[repr(u16)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Win0 = 0,
    Win1 = 1,
    Outside = 2,
    OBJ = 3,
}

impl Window {
    // @TODO this can be made const when `match` is enabled in const fns in stable Rust.
    pub fn from_index(index: u16) -> Window {
        match index {
            0 => Window::Win0,
            1 => Window::Win1,
            2 => Window::Outside,
            3 => Window::OBJ,
            _ => panic!("Invalid window."),
        }
    }

    // @TODO this can be made const when `match` is enabled in const fns in stable Rust.
    pub unsafe fn from_index_unsafe(index: u16) -> Window {
        match index {
            0 => Window::Win0,
            1 => Window::Win1,
            2 => Window::Outside,
            3 => Window::OBJ,
            _ => std::hint::unreachable_unchecked(),
        }
    }

    #[inline(always)]
    pub const fn index(self) -> usize {
        self as u16 as usize
    }

    #[inline(always)]
    pub const fn reg_index(self) -> u16 {
        (self as u16) & 1
    }
}

pub struct WindowInfo {
    /// This flag is set to true of any of the three windows are enabled.
    pub enabled: bool,
    pub win0_enabled: bool,
    pub win1_enabled: bool,
    pub win_obj_enabled: bool,
    pub win0_bounds: WindowBounds,
    pub win1_bounds: WindowBounds,
    pub winin: WindowControl,
    pub winout: WindowControl,
    pub obj_window: LCDPixelBits,

    pub window_effects_masks: [u16; 4],
}

impl WindowInfo {
    pub const fn new() -> WindowInfo {
        WindowInfo {
            enabled: false,
            win0_enabled: false,
            win1_enabled: false,
            win_obj_enabled: false,
            win0_bounds: WindowBounds::zero(),
            win1_bounds: WindowBounds::zero(),
            winin: WindowControl { inner: 0 },
            winout: WindowControl { inner: 0 },
            obj_window: LCDPixelBits::new(),
            window_effects_masks: [0; 4],
        }
    }

    /// Returns Some(window) if a pixel is contained inside of a given window.
    pub(crate) fn check_pixel(&self, layer: Layer, x: u16, y: u16) -> Option<u16> {
        if self.win0_enabled && self.win0_bounds.contains(x, y) {
            if self.winin.layer_enabled(Window::Win0, layer) {
                return Some(self.window_effects_masks[Window::Win0.index()]);
            } else {
                return None;
            }
        }

        if self.win1_enabled && self.win1_bounds.contains(x, y) {
            if self.winin.layer_enabled(Window::Win1, layer) {
                return Some(self.window_effects_masks[Window::Win1.index()]);
            } else {
                return None;
            }
        }

        if self.win_obj_enabled && self.obj_window.get(x as usize) {
            if self.winout.layer_enabled(Window::OBJ, layer) {
                return Some(self.window_effects_masks[Window::OBJ.index()]);
            } else {
                return None;
            }
        }

        if self.winout.layer_enabled(Window::Outside, layer) {
            return Some(self.window_effects_masks[Window::Outside.index()]);
        }

        return None;
    }
}

// pub(crate) fn get_window_bits(
//     layer: u16,
//     line: u16,
//     window_info: &WindowInfo,
//     obj_window: &LCDPixelBits,
// ) -> LCDPixelBits {
//     let mut bits = LCDPixelBits::new();

//     if !window_info.enabled {
//         bits.set_all();
//         return bits;
//     }

//     // TODO: maybe create a seprate loop for `check_pixel` when obj_window is not enabled?
//     for x in 0..240 {
//         if window_info.check_pixel_obj_window(layer, x, line, obj_window) {
//             bits.set(x);
//         }
//     }

//     return bits;
// }

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
    pub fn display_layer(&self, layer: Layer) -> bool {
        ((self.value >> (layer.index() + 8)) & 1) != 0
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

    pub fn is_first_target(&self, layer: Layer) -> bool {
        return (self.inner & (1 << layer.index())) != 0;
    }

    pub fn is_second_target(&self, layer: Layer) -> bool {
        return (self.inner & (1 << (layer.index() + 8))) != 0;
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

#[derive(Default, Clone)]
pub struct AffineBGParams {
    pub internal_x: FixedPoint32,
    pub internal_y: FixedPoint32,

    pub a: FixedPoint32,
    pub b: FixedPoint32,
    pub c: FixedPoint32,
    pub d: FixedPoint32,

    pub x: FixedPoint32,
    pub y: FixedPoint32,
}

impl AffineBGParams {
    /// Copies the reference point registers into the internal reference point registers.
    pub fn copy_reference_points(&mut self) {
        self.internal_x = self.x;
        self.internal_y = self.y;
    }

    pub fn increment_reference_points(&mut self) {
        self.internal_x += self.b; // increment by dmx
        self.internal_y += self.d; // increment by dmy
    }

    pub fn set_x(&mut self, value: u32) {
        self.x = FixedPoint32::wrap(((value as i32) << 4) >> 4);
    }

    pub fn set_y(&mut self, value: u32) {
        self.y = FixedPoint32::wrap(((value as i32) << 4) >> 4);
    }

    pub fn set_x_lo(&mut self, value: u16) {
        let raw_x = (self.x.to_inner() & 0xFFFF0000u32 as i32) | (value as i32);
        self.x = FixedPoint32::wrap(raw_x);
    }

    pub fn set_x_hi(&mut self, value: u16) {
        // this will sign extend the final 4 bits
        self.set_x((self.x.to_inner() as u32 & 0x0000FFFF) | ((value as u32) << 16));
    }

    pub fn set_y_lo(&mut self, value: u16) {
        let raw_y = (self.y.to_inner() & 0xFFFF0000u32 as i32) | (value as i32);
        self.y = FixedPoint32::wrap(raw_y);
    }

    pub fn set_y_hi(&mut self, value: u16) {
        // this will sign extend the final 4 bits
        self.set_y((self.y.to_inner() as u32 & 0x0000FFFF) | ((value as u32) << 16));
    }

    pub fn set_a(&mut self, value: u16) {
        self.a = FixedPoint32::from(FixedPoint16::wrap(value as i16));
    }

    pub fn set_b(&mut self, value: u16) {
        self.b = FixedPoint32::from(FixedPoint16::wrap(value as i16));
    }

    pub fn set_c(&mut self, value: u16) {
        self.c = FixedPoint32::from(FixedPoint16::wrap(value as i16));
    }

    pub fn set_d(&mut self, value: u16) {
        self.d = FixedPoint32::from(FixedPoint16::wrap(value as i16));
    }
}

#[derive(Default, Clone, Copy)]
pub struct WindowBounds {
    /// Left-most coordinate of the window.
    pub left: u16,
    /// Top-most coordinate of the window.
    pub top: u16,
    /// Right-most coordinate of the window, plus 1.
    pub right: u16,
    /// Bottom-most coordinate of the window, plus 1.
    pub bottom: u16,
}

impl WindowBounds {
    pub const fn zero() -> WindowBounds {
        WindowBounds {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        let h1 = (self.left <= self.right) & ((x >= self.left) & (x < self.right));
        let h2 = (self.left > self.right) & ((x >= self.left) | (x < self.right));
        if !(h1 | h2) {
            return false;
        }

        let v1 = (self.top <= self.bottom) & ((y >= self.top) & (y < self.bottom));
        let v2 = (self.top > self.bottom) & ((y >= self.top) | (y < self.bottom));
        return v1 | v2;
    }

    pub(crate) fn set_h(&mut self, h: u16) {
        self.left = std::cmp::min(h >> 8, 240);
        self.right = h & 0xFF;

        // Garbage values of R>240 or L>R are interpreted as R=240.
        if self.right > 240 {
            self.right = 240;
        }
    }

    pub(crate) fn set_v(&mut self, v: u16) {
        self.top = std::cmp::min(v >> 8, 160);
        self.bottom = v & 0xFF;

        // Garbage values of B>160 or T>B are interpreted as B=160.
        if self.bottom > 160 {
            self.bottom = 160;
        }
    }
}

#[derive(Default, Clone, Copy)]
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

    /// Returns true if the given layer (layer #4) is enabled in the given window
    /// (0 or 1). #NOTE That if this window control is for WINOUT, window 0 is the outside window
    /// and window 1 is the OBJ window.
    #[inline(always)]
    pub fn layer_enabled(&self, window: Window, layer: Layer) -> bool {
        ((self.inner >> (layer.index() + (window.reg_index() * 8))) & 1) != 0
    }

    /// Returns true if color special effects is enabled for a given window.
    /// #NOTE That if this window control is for WINOUT, window 0 is the outside window
    /// and window 1 is the OBJ window.
    pub fn effects_enabled(&self, window: Window) -> bool {
        ((self.inner >> (5 + (window.reg_index() * 8))) & 1) != 0
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
