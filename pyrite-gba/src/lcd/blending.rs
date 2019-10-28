use super::Line;

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

#[derive(Copy, Clone)]
pub struct PixelInfo {
    /// This is true if the current pixel at this position is selected as a first target pixel in
    /// the color special effects register.
    pub is_first_target:        bool,

    /// This is true if the current pixel at this position is selected as a second target pixel in
    /// the color special effects register.
    pub is_second_target:       bool,

    /// The priority assigned to the current pixel in this position.
    pub priority:               u8,
}

#[inline]
pub fn poke_bg_pixel(offset: usize, color: u16, bg_priority: u8, out: &mut Line, pixel_info: &mut [PixelInfo; 240]) {
    let current_priority = pixel_info[offset].priority & 0xF;
    if (color & 0x8000) != 0 && current_priority > bg_priority {
        pixel_info[offset].priority = bg_priority | (0xF0);
        out[offset] = color;
    }
}

#[inline]
pub fn poke_obj_pixel(offset: usize, color: u16, obj_priority: u8, out: &mut Line, pixel_info: &mut [PixelInfo; 240]) {
    let is_bg = (pixel_info[offset].priority & 0xF0) != 0;
    let current_priority = pixel_info[offset].priority & 0xF;
    if (color & 0x8000) != 0 && (current_priority > obj_priority || (current_priority == obj_priority && is_bg)) {
        // offset should never be out of bounds here
        unsafe {
            (*pixel_info.get_unchecked_mut(offset)).priority = obj_priority;
            *out.get_unchecked_mut(offset) = color;
        }
    }
}
