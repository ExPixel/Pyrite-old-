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

