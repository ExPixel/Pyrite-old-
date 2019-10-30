use crate::memory::ioreg::{ IORegisters, RegEffectsSelect, RegAlpha, RegBrightness, ColorSpecialEffect };
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
pub struct Windows {
    win0h:  u16,
    win0v:  u16,
    win1h:  u16,
    win1v:  u16,
    winin:  u16,
    winout: u16,
}

impl Windows {
    pub fn new() -> Windows {
        // @TODO implement this :P
        Windows {
            win0h:  0,
            win0v:  0,
            win1h:  0,
            win1v:  0,
            winin:  0,
            winout: 0,
        }
    }
}

#[inline]
pub fn poke_bg_pixel(bg: u16, offset: usize, color: u16, bg_priority: u8, raw_pixels: &mut [RawPixel; 240], effects: SpecialEffects, _windows: Windows) {
    if (color & 0x8000) == 0 { return }
    let pixel = &mut raw_pixels[offset];
    if pixel.top.priority > bg_priority {
        pixel.bottom = pixel.top;
        pixel.top = RawPixelLayer {
            color: color,
            semi_transparent_obj: false,
            first_target: effects.select.is_first_target(bg),
            second_target: effects.select.is_second_target(bg),
            priority: bg_priority,
        };
    } else if pixel.bottom.priority > bg_priority {
        pixel.bottom = RawPixelLayer {
            color: color,
            semi_transparent_obj: false,
            first_target: effects.select.is_first_target(bg),
            second_target: effects.select.is_second_target(bg),
            priority: bg_priority,
        };
    }
    // if (color & 0x8000) != 0 {
    //     if pixel_info[offset].priority > bg_priority  {
    //         pixel_info[offset].priority = bg_priority;
    //         pixel_info[offset].is_semi_transparent_obj = false;
    //         if effects.select.is_first_target(bg) {
    //             if pixel_info[offset].is_second_target {
    //                 out[offset] = effects.blend(color, out[offset]);
    //             } else {
    //                 out[offset] = effects.blend(color, 0);
    //             }
    //             pixel_info[offset].is_second_target = false;
    //             pixel_info[offset].first_target_color = color;
    //         } else {
    //             pixel_info[offset].first_target_color = 0;
    //             pixel_info[offset].is_second_target = effects.select.is_second_target(bg);
    //             out[offset] = color;
    //         }
    //     } else if pixel_info[offset].first_target_color != 0 && effects.select.is_second_target(bg) {
    //         // here we draw a second target pixel that would have been at a higher priority than
    //         // the previous second target pixel but is at a lower priority than the current first
    //         // target pixel. So we blend with the current high priority first target pixel.
    //         // @NOTE: having a higher priority NUMBER means that a layer has a LOWER priority.
    //         //          e.g. priority 0 is higher than priority 2

    //         if pixel_info[offset].is_semi_transparent_obj {
    //             // semi-transparent objects always force alpha blending as the first target
    //             out[offset] = effects.alpha_blend(pixel_info[offset].first_target_color, color);
    //         } else {
    //             out[offset] = effects.blend(pixel_info[offset].first_target_color, color);
    //         }
    //     }
    // }
}

/// This expects that objects are always drawn before any backgrounds. It owill only check the
/// priority of the top layer.
#[inline]
pub fn poke_obj_pixel(offset: usize, color: u16, obj_priority: u8, obj_mode: ObjMode, raw_pixels: &mut [RawPixel; 240], effects: SpecialEffects, _windows: Windows) {
    if (color & 0x8000) == 0 { return }
    let pixel = &mut raw_pixels[offset];
    if obj_mode == ObjMode::OBJWindow {
        pixel.obj_window = true;
    } else if pixel.top.priority > obj_priority {
        pixel.bottom = pixel.top;
        pixel.top = RawPixelLayer {
            color: color,
            semi_transparent_obj: obj_mode == ObjMode::SemiTransparent,
            first_target: effects.select.is_first_target(4),
            second_target: effects.select.is_second_target(4),
            priority: obj_priority,
        };
    }
    // if (color & 0x8000) != 0 && pixel_info[offset].priority > obj_priority {
    //     // offset should never be out of bounds here
    //     unsafe {
    //         let is_semi_transparent = obj_mode == ObjMode::SemiTransparent;
    //         let info = pixel_info.get_unchecked_mut(offset);
    //         if effects.select.is_first_target(4) || is_semi_transparent {
    //             (*info).first_target_color = color;
    //             (*info).is_semi_transparent_obj = is_semi_transparent;

    //             *out.get_unchecked_mut(offset) = if is_semi_transparent && (*info).is_second_target {
    //                 effects.alpha_blend(color, *out.get_unchecked(offset))
    //             } else if !is_semi_transparent {
    //                 match effects.select.special_effect() {
    //                     ColorSpecialEffect::BrightnessIncrease => effects.brightness_increase(color),
    //                     ColorSpecialEffect::BrightnessDecrease => effects.brightness_decrease(color),
    //                     _ => color,
    //                 }
    //             } else {
    //                 color
    //             };
    //         } else {
    //             (*info).first_target_color = 0;
    //             (*info).is_semi_transparent_obj = false;
    //             (*info).is_second_target = effects.select.is_second_target(4);
    //             *out.get_unchecked_mut(offset) = color;
    //         }
    //         (*info).priority = obj_priority;
    //     }
    // }
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
        Windows::new(),
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
