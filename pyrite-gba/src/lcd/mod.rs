pub mod palette;
pub mod obj;
pub mod bitmap;

use self::palette::GbaPalette;
use crate::util::fixedpoint::{ FixedPoint16, FixedPoint32 };
use crate::hardware::{ OAM, VRAM };
use crate::GbaVideoOutput;
use pyrite_common::bits;

pub const OBJ_LAYER: u16 = 4;
pub const  BD_LAYER: u16 = 5;

pub const HDRAW_CYCLES: u32 = 960;
pub const HBLANK_CYCLES: u32 = 272;


#[inline(always)]
const fn set_halfword_of_word(word: u32, value: u16, off: u32) -> u32 {
    let shift = (off as u32 & 0x10) << 3;
    (word & !(0xFFFF << shift)) | ((value as u32) << shift)
}

pub struct GbaLCD {
    pub(crate) registers:   LCDRegisters,
    hblank:                 bool,
    next_state_cycles:      u32,
    pixels:                 [u16; 240],
    frame_ready:            bool,
}

impl GbaLCD {
    pub fn new() -> GbaLCD {
        GbaLCD {
            registers:          LCDRegisters::default(),
            hblank:             false,
            next_state_cycles:  HDRAW_CYCLES,
            pixels:             [0xFFFF; 240],
            frame_ready:        false,
        }
    }

    #[inline]
    pub fn step(&mut self, cycles: u32, vram: &VRAM, oam: &OAM, palette: &GbaPalette, video: &mut dyn GbaVideoOutput) {
        let original_cycles = self.next_state_cycles;
        self.next_state_cycles = self.next_state_cycles.saturating_sub(cycles);
        if self.next_state_cycles == 0 {
            self.hblank = !self.hblank;
            if self.hblank {
                self.next_state_cycles = HDRAW_CYCLES - (cycles - original_cycles);
                self.hblank(vram, oam, palette, video);
            } else {
                self.next_state_cycles = HBLANK_CYCLES - (cycles - original_cycles);
                self.hdraw();
            }
        }
    }

    fn hdraw(&mut self) {
        self.registers.dispstat.set_hblank(true);
        self.registers.line += 1;

        match self.registers.line {
            160 => self.registers.dispstat.set_vblank(true),
            227 => self.registers.dispstat.set_vblank(false),
            228 => self.registers.line = 0,
            _ => { /* NOP */ },
        }
    }

    fn hblank(&mut self, vram: &VRAM, oam: &OAM, palette: &GbaPalette, video: &mut dyn GbaVideoOutput) {
        self.registers.dispstat.set_hblank(false);

        if self.registers.line < 160 {
            if self.registers.line ==   0 {  video.pre_frame(); }
            self.draw_line(vram, oam, palette);
            video.display_line(self.registers.line as u32, &self.pixels);
            if self.registers.line == 159 { video.post_frame(); self.frame_ready = true; }
        }
    }

    fn draw_line(&mut self, vram: &VRAM, oam: &OAM, palette: &GbaPalette) {
        let mode = self.registers.dispcnt.mode();
        
        match mode {
            0 => eprintln!("unhandled BG mode 0"),
            1 => eprintln!("unhandled BG mode 0"),
            2 => eprintln!("unhandled BG mode 0"),
            3 => bitmap::render_mode3(&self.registers,vram, oam, palette, &mut self.pixels),
            4 => eprintln!("unhandled BG mode 0"),
            5 => eprintln!("unhandled BG mode 0"),
            _ => eprintln!("bad mode {}", mode),
        }
    }
}

pub struct LCDLineBuffer {
    pixels: [u32; 240],
}

impl LCDLineBuffer {
    /// Pushes a pixel onto the LCD.
    pub fn push(pixel: u16, offset: usize) {
    }
}

#[derive(Default)]
pub struct LCDRegisters {
    /// The current line being rendered by the LCD.
    pub line: u16,

    /// LCD control register.
    pub dispcnt:    DisplayControl,

    /// LCD status register.
    pub dispstat:   DisplayStatus,

    // @TODO implement whatever this is.
    pub greenswap:  u16,

    // Background Control Registers:
    pub bg_cnt:     [BGControl; 4],
    pub bg_ofs:     [BGOffset; 4],


    // LCD BG Rotation / Scaling:
    pub bg2_affine_params:  AffineBGParams,
    pub bg3_affine_params:  AffineBGParams,

    // LCD Windows:
    pub win0_bounds:    WindowBounds,
    pub win1_bounds:    WindowBounds,
    pub winin:          WindowControl,
    pub winout:         WindowControl,

    // Special Effects
    pub mosaic:     Mosaic,
    pub effects:    EffectsSelection,
    pub alpha:      u16,
    pub brightness: u16,
}

impl LCDRegisters {
    #[inline(always)]
    pub fn set_dispstat(&mut self, value: u16) {
        pub const DISPSTAT_WRITEABLE: u16 = 0xFFB4;
        self.dispstat.value = (self.dispstat.value & !DISPSTAT_WRITEABLE) | (value & DISPSTAT_WRITEABLE);
    }
}

bitfields! (DisplayStatus: u16 {
    vblank, set_vblank: bool = [0, 0],
    hblank, set_hblank: bool = [1, 1],
    vcounter, set_vcounter: bool = [2, 2],

    vblank_irq_enable, set_vblank_irq_enable: bool = [3, 3],
    hblank_irq_enable, set_hblank_irq_enable: bool = [4, 4],
    vcounter_irq_enable, set_vcounter_irq_enable: bool = [5, 5],
});

bitfields! (DisplayControl: u16 {
    mode, set_mode: u16 = [0, 2],
    frame_select, set_frame_select: u16 = [4, 4],
    hblank_internal_free, set_hblank_interval_free: bool = [5, 5],
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
    pub fn display_layer(layer: u16) -> bool {
        assert!(layer <= 4,"display layer index must be in range [0, 4]");
        ((layer >> (layer + 8)) & 1) != 0
    }
}

#[derive(Default)]
pub struct EffectsSelection {
    inner: u16,
}

impl EffectsSelection {
    #[inline(always)] pub fn value(&self) -> u16 { self.inner }
    #[inline(always)] pub fn set_value(&mut self, value: u16) { self.inner = value; }

    pub fn is_first_target(&self, layer: u16) -> bool {
        assert!(layer <= 5, "first target layer index must be in range [0, 5]");
        return (self.inner & (1 << layer)) != 0;
    }

    pub fn is_second_target(&self, layer: u16) -> bool {
        assert!(layer <= 5, "second target layer index must be in range [0, 5]");
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

#[derive(Clone, Copy)]
pub enum SpecialEffect {
    None,
    AlphaBlending,
    BrightnessIncrease,
    BrightnessDecrease,
}


#[derive(Default)]
pub struct Mosaic {
    pub  bg: (/* horizontal */ u8, /* vertical */ u8),
    pub obj: (/* horizontal */ u8, /* vertical */ u8),
}

impl Mosaic {
    pub fn set_value(&mut self, value: u16) {
         self.bg.0 = (bits!(value,  0,  3) + 1) as u8;
         self.bg.1 = (bits!(value,  4,  7) + 1) as u8;
        self.obj.0 = (bits!(value,  8, 11) + 1) as u8;
        self.obj.1 = (bits!(value, 12, 15) + 1) as u8;
    }
}

#[derive(Default)]
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

    pub a:  FixedPoint32,
    pub b:  FixedPoint32,
    pub c:  FixedPoint32,
    pub d:  FixedPoint32,

    pub x:  u32,
    pub y:  u32,
}

impl AffineBGParams {
    /// Copies the reference point registers into the internal reference point registers.
    #[inline]
    pub fn copy_reference_points(&mut self) {
        self.internal_x = FixedPoint32::wrap(((self.x as i32) << 4) >> 4);
        self.internal_y = FixedPoint32::wrap(((self.y as i32) << 4) >> 4);
    }

    #[inline]
    pub fn set_a(&mut self, value: u16) { self.a = FixedPoint32::from(FixedPoint16::wrap(value as i16)); }
    #[inline]
    pub fn set_b(&mut self, value: u16) { self.b = FixedPoint32::from(FixedPoint16::wrap(value as i16)); }
    #[inline]
    pub fn set_c(&mut self, value: u16) { self.c = FixedPoint32::from(FixedPoint16::wrap(value as i16)); }
    #[inline]
    pub fn set_d(&mut self, value: u16) { self.d = FixedPoint32::from(FixedPoint16::wrap(value as i16)); }
}

#[derive(Default)]
pub struct WindowBounds {
    pub left:   u16,
    pub top:    u16,
    pub right:  u16,
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
    pub fn value(&self) -> u16 { self.inner }

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
