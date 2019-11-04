use crate::memory::ioreg::{
    IORegisters,
    RegEffectsSelect,
    RegAlpha,
    RegBrightness,
    ColorSpecialEffect,
    RegDISPCNT,
    RegWINH,
    RegWINV,
    RegWININ,
    RegWINOUT,
};
use super::obj::ObjMode;
use super::{ Line, RawLine };

#[inline]
pub fn apply_mosaic(value: u32, mosaic: u32) -> u32 {
    if mosaic > 0 {
        return value - (value % mosaic)
    } else {
        return value;
    }
}

#[inline]
pub fn apply_mosaic_cond(mosaic_cond: bool, value: u32, mosaic: u32) -> u32 {
    if mosaic_cond && mosaic > 0 {
        return value - (value % mosaic)
    } else {
        return value;
    }
}

#[derive(Clone, Copy)]
pub struct RawPixel {
    pub top:    RawPixelLayer,
    pub bottom: RawPixelLayer,
    pub obj_window: bool,
}

impl RawPixel {
    pub const fn empty() -> RawPixel {
        RawPixel {
            top:    RawPixelLayer::empty(),
            bottom: RawPixelLayer::empty(),
            obj_window: false,
        }
    }

    pub fn backdrop(bldcnt: RegEffectsSelect, backdrop_color: u16) -> RawPixel {
        RawPixel {
            top:    RawPixelLayer::backdrop(bldcnt, backdrop_color),
            bottom: RawPixelLayer::empty(),
            obj_window: false,
        }
    }
}

#[derive(Clone, Copy)]
pub struct RawPixelLayer {
    /// The color of this raw pixel.
    pub color: u16,

    /// True if this pixel is from a semi transparent object.
    pub semi_transparent_obj: bool,

    /// True if this pixel is selected as a first target pixel.
    pub first_target: bool,

    /// True if this pixel is selected as a second target pixel.
    pub second_target: bool,

    /// The priority of this pixel layer. Lower number means higher priority.
    pub priority: u8,
}

impl RawPixelLayer {
    pub const fn empty() -> RawPixelLayer {
        RawPixelLayer {
            color: 0,
            semi_transparent_obj: false,
            first_target: false,
            second_target: false,
            priority: 4,
        }
    }

    pub fn backdrop(bldcnt: RegEffectsSelect, backdrop_color: u16) -> RawPixelLayer {
        RawPixelLayer {
            color: backdrop_color,
            semi_transparent_obj: false,
            first_target: bldcnt.is_first_target(5),
            second_target: bldcnt.is_second_target(5),
            priority: 4,
        }
    }
}

#[derive(Clone, Copy)]
pub struct SpecialEffects {
    select:     RegEffectsSelect,
    alpha:      RegAlpha,
    brightness: RegBrightness,
}

impl SpecialEffects {
    pub fn new(select: RegEffectsSelect, alpha: RegAlpha, brightness: RegBrightness) -> SpecialEffects {
        SpecialEffects { select, alpha, brightness }
    }

    pub fn blend(&self, first_target: u16, second_target: u16) -> u16 {
        match self.select.special_effect() {
            ColorSpecialEffect::AlphaBlending => {
                self.alpha_blend(first_target, second_target)
            },

            ColorSpecialEffect::BrightnessIncrease => {
                self.brightness_increase(first_target)
            },

            ColorSpecialEffect::BrightnessDecrease => {
                self.brightness_decrease(first_target)
            },

            ColorSpecialEffect::None => {
                first_target
            }
        }
    }

    pub fn blend_single_target(&self, first_target: u16) -> u16 {
        match self.select.special_effect() {
            ColorSpecialEffect::AlphaBlending => {
                first_target
            },

            ColorSpecialEffect::BrightnessIncrease => {
                self.brightness_increase(first_target)
            },

            ColorSpecialEffect::BrightnessDecrease => {
                self.brightness_decrease(first_target)
            },

            ColorSpecialEffect::None => {
                first_target
            }
        }
    }

    pub fn brightness_increase(&self, first_target: u16) -> u16 {
        // I = I1st + (31-I1st)*EVY
        // where I is the separate R, G, and B components
        let evy = self.brightness.evy_coeff(); // this is actually evy * 16
        let (r1, g1, b1) = pixel_components(first_target);
        let r = std::cmp::min(31, r1 + ((31 - r1)*evy)/16);
        let g = std::cmp::min(31, g1 + ((31 - g1)*evy)/16);
        let b = std::cmp::min(31, b1 + ((31 - b1)*evy)/16);
        rgb16(r, g, b)
    }

    pub fn brightness_decrease(&self, first_target: u16) -> u16 {
        // I = I1st - (I1st)*EVY
        // where I is the separate R, G, and B components
        let evy = self.brightness.evy_coeff(); // this is actually evy * 16
        let (r1, g1, b1) = pixel_components(first_target);
        let r = std::cmp::min(31, r1 - (r1*evy)/16);
        let g = std::cmp::min(31, g1 - (g1*evy)/16);
        let b = std::cmp::min(31, b1 - (b1*evy)/16);
        rgb16(r, g, b)
    }

    pub fn alpha_blend(&self, first_target: u16, second_target: u16) -> u16 {
        // I = MIN ( 31, I1st*EVA + I2nd*EVB )
        // where I is the separate R, G, and B components
        let eva = self.alpha.eva_coeff(); // this is actually eva * 16
        let evb = self.alpha.evb_coeff(); // this is actually evb * 16

        let (r1, g1, b1) = pixel_components(first_target);
        let (r2, g2, b2) = pixel_components(second_target);
        let r = std::cmp::min(31, (r1*eva)/16 + (r2*evb)/16);
        let g = std::cmp::min(31, (g1*eva)/16 + (g2*evb)/16);
        let b = std::cmp::min(31, (b1*eva)/16 + (b2*evb)/16);
        rgb16(r, g, b)
    }
}

#[derive(Clone, Copy)]
pub struct WindowRect {
    left: u16,
    top: u16,

    /// This will actually contain right + 1
    right: u16,
    /// This will actually contain bottom + 1
    bottom: u16,
}

impl WindowRect {
    pub fn new(left: u16, top: u16, right: u16, bottom: u16) -> WindowRect {
        WindowRect {
            left: std::cmp::min(239, left),
            top: std::cmp::min(159, top),
            right: std::cmp::min(240, right),
            bottom: std::cmp::min(160, bottom),
        }
    }

    #[inline]
    pub fn in_bounds(&self, x: u16, y: u16) -> bool {
        let h = if self.left > self.right {
            x >= self.left || x < self.right
        } else {
            x >= self.left && x < self.right
        };

        let v = if self.top > self.bottom {
            y >= self.top || y < self.bottom
        } else {
            y >= self.top && y < self.bottom
        };

        return h & v;
    }
}

#[derive(Clone, Copy)]
pub struct Windows {
    dispcnt:    RegDISPCNT,
    win0:       WindowRect,
    win1:       WindowRect,
    winin:      RegWININ,
    winout:     RegWINOUT,
}

impl Windows {
    pub fn new(dispcnt: RegDISPCNT, win0h: RegWINH, win0v: RegWINV, win1h: RegWINH, win1v: RegWINV, winin: RegWININ, winout: RegWINOUT) -> Windows {
        Windows {
            dispcnt: dispcnt,
            win0:   WindowRect::new(win0h.left(), win0v.top(), win0h.right(), win0v.bottom()),
            win1:   WindowRect::new(win1h.left(), win1v.top(), win1h.right(), win1v.bottom()),
            winin:  winin,
            winout: winout,
        }
    }

    pub fn check_window_bounds(&self, layer: u16, x: u16, y: u16, raw_pixels: &RawLine) -> (/* show pixel */ bool, /* special effects */ bool) {
        if self.dispcnt.display_window0() && self.win0.in_bounds(x, y) {
            if self.winin.is_in_window0(layer) {
                return (true, self.winin.win0_special_effects());
            } else {
                return (false, false);
            }
        }

        if self.dispcnt.display_window1() && self.win1.in_bounds(x, y) {
            if self.winin.is_in_window1(layer) {
                return (true, self.winin.win1_special_effects());
            } else {
                return (false, false);
            }
        }

        if self.dispcnt.display_obj_window() && raw_pixels[x as usize].obj_window {
            if self.winout.is_in_window_obj(layer) {
                return (true, self.winout.winobj_special_effects());
            } else {
                return (false, false);
            }
        }

        if self.winout.is_in_window_out(layer) {
            return (true, self.winout.winout_special_effects());
        } else {
            (false, false)
        }
    }

    /// Returns true if any of the windows are enabled
    #[inline(always)]
    pub fn enabled(&self) -> bool {
        (self.dispcnt.inner & (0xE000)) != 0
    }
}

#[inline]
pub fn poke_bg_pixel(line: u32, bg: u16, offset: usize, color: u16, bg_priority: u8, raw_pixels: &mut RawLine, effects: SpecialEffects, windows: Windows) {
    if (color & 0x8000) == 0 { return }

    let enable_special_effects = if windows.enabled() {
        let (show_pixel, enable_special_effects) = windows.check_window_bounds(bg, offset as u16, line as u16, raw_pixels);
        if !show_pixel { return }
        enable_special_effects
    } else { true };

    let pixel = &mut raw_pixels[offset];
    if pixel.top.priority > bg_priority {
        pixel.bottom = pixel.top;
        pixel.top = RawPixelLayer {
            color: color,
            semi_transparent_obj: false,
            first_target: enable_special_effects & effects.select.is_first_target(bg),
            second_target: enable_special_effects & effects.select.is_second_target(bg),
            priority: bg_priority,
        };
    } else if pixel.bottom.priority > bg_priority {
        pixel.bottom = RawPixelLayer {
            color: color,
            semi_transparent_obj: false,
            first_target: enable_special_effects & effects.select.is_first_target(bg),
            second_target: enable_special_effects & effects.select.is_second_target(bg),
            priority: bg_priority,
        };
    }
}

/// This expects that objects are always drawn before any backgrounds. It owill only check the
/// priority of the top layer.
#[inline]
pub fn poke_obj_pixel(line: u32, offset: usize, color: u16, obj_priority: u8, obj_mode: ObjMode, raw_pixels: &mut RawLine, effects: SpecialEffects, windows: Windows) {
    if (color & 0x8000) == 0 { return }

    if obj_mode == ObjMode::OBJWindow {
        raw_pixels[offset].obj_window = true;
    } else if raw_pixels[offset].top.priority > obj_priority {
        let enable_special_effects = if windows.enabled() {
            let (show_pixel, enable_special_effects) = windows.check_window_bounds(4, offset as u16, line as u16, raw_pixels);
            if !show_pixel { return }
            enable_special_effects
        } else { true };

        raw_pixels[offset].bottom = raw_pixels[offset].top;
        raw_pixels[offset].top = RawPixelLayer {
            color: color,
            semi_transparent_obj: enable_special_effects & (obj_mode == ObjMode::SemiTransparent),
            first_target: enable_special_effects & effects.select.is_first_target(4),
            second_target: enable_special_effects & effects.select.is_second_target(4),
            priority: obj_priority,
        };
    }
}

pub fn blend_raw_pixels(raw_line: &RawLine, out_line: &mut Line, effects: SpecialEffects) {
    for idx in 0..240 {
        let raw = raw_line.get(idx).unwrap();

        if raw.top.first_target || raw.top.semi_transparent_obj {
            if raw.bottom.second_target {
                if raw.top.semi_transparent_obj {
                    out_line[idx] = effects.alpha_blend(raw.top.color, raw.bottom.color);
                } else {
                    out_line[idx] = effects.blend(raw.top.color, raw.bottom.color);
                }
            } else {
                // @NOTE some emulators seem to have different behavior here for semi-transparent
                // objects and opt not to allow brightness effects for them at all. I no longer
                // have real hardware to test this on unfortunately so for now I'm just going to
                // follow my interpretation of GBATek:
                //  Semi-Transparent OBJs
                //      OBJs that are defined as 'Semi-Transparent' in OAM memory are always selected as 1st Target
                //      (regardless of BLDCNT Bit 4), and are always using Alpha Blending mode (regardless of BLDCNT Bit 6-7).
                //
                //         *** specifically this part ***
                //         vvvvvvvvvvvvvvvvvvvvvvvvvvvvvv         
                //
                //      The BLDCNT register may be used to perform Brightness effects on the OBJ (and/or other BG/BD layers).
                //      However, if a semi-transparent OBJ pixel does overlap a 2nd target pixel, then semi-transparency becomes
                //      priority, and the brightness effect will not take place (neither on 1st, nor 2nd target).
                out_line[idx] = effects.blend_single_target(raw.top.color);
            }
        } else {
            out_line[idx] = raw.top.color;
        };
    }
}

#[inline]
pub fn get_compositing_info(ioregs: &IORegisters) -> (SpecialEffects, Windows) {
    (
        SpecialEffects::new(ioregs.bldcnt, ioregs.bldalpha, ioregs.bldy),
        Windows::new(ioregs.dispcnt, ioregs.win0h, ioregs.win0v, ioregs.win1h, ioregs.win1v, ioregs.winin, ioregs.winout),
    )
}

#[inline(always)]
pub fn pixel_components(pixel: u16) -> (u16, u16, u16) {
    (
        pixel & 0x1F,
        (pixel >> 5) & 0x1F,
        (pixel >> 10) & 0x1F,
    )
}

#[inline(always)]
pub fn rgb16(r: u16, g: u16, b: u16) -> u16 {
    (r & 0x1F) | ((g & 0x1F) << 5) | ((b & 0x1F) << 10) | 0x8000
}
