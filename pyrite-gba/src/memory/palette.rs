pub struct Palette {
    bg_colors:  [u16; 256],
    obj_colors: [u16; 256],
}

impl Palette {
    pub fn new() -> Palette {
        Palette {
            bg_colors:  [0u16; 256],
            obj_colors: [0u16; 256],
        }
    }

    pub fn store8(&mut self, addr: u32, value: u8) {
        let hw = self.load16(addr & 0xFFFFFFFE);
        let new_value = if (addr & 1) == 0 {
            (hw & 0xFF00) | (value as u16)
        } else {
            (hw & 0x00FF) | ((value as u16) << 8)
        };
        self.store16(addr & 0xFFFFFFFE, new_value);
    }

    pub fn store16(&mut self, addr: u32, value: u16) {
        match addr {
            0x000..=0x1FE => {
                self.bg_colors[(addr / 2) as usize] = value;
            },
            0x200..=0x3FE => {
                self.obj_colors[((addr - 0x200) / 2) as usize] = value;
            },
            _ => {
                // @TODO maybe I should just log an error instead.
                panic!("bad palette write to address 0x{:08X}", addr);
            }
        }
    }

    pub fn store32(&mut self, addr: u32, value: u32) {
        let aligned_addr = addr & 0xFFFFFFFC;
        self.store16(aligned_addr, value as u16);
        self.store16(aligned_addr + 2, (value >> 16) as u16);
    }

    pub fn load8(&self, addr: u32) -> u8 {
        let hw = self.load16(addr & 0xFFFFFFFE);
        if (addr & 1) == 0 {
            hw as u8
        } else {
            (hw >> 8) as u8
        }
    }

    pub fn load16(&self, addr: u32) -> u16 {
        match addr {
            0x000..=0x1FE =>  self.bg_colors[(addr / 2) as usize],
            0x200..=0x3FE => self.obj_colors[((addr - 0x200) / 2) as usize],
            _ => {
                // @TODO maybe I should just log an error instead.
                panic!("bad palette write to address 0x{:08X}", addr);
            }
        }
    }

    pub fn load32(&self, addr: u32) -> u32 {
        let lo = self.load16(addr) as u32;
        let hi = self.load16(addr + 2) as u32;
        (hi << 16) | lo
    }

    #[inline]
    pub fn get_bg16(&self, palette_index: u8, color_index: u8) -> u16 {
        debug_assert!(palette_index < 16, "palette index must be less than 16");
        debug_assert!(color_index < 16, "color index must be less than 16");
        self.bg_colors[((palette_index * 16) + color_index) as usize]
    }

    #[inline]
    pub fn get_bg256(&self, color_index: u8) -> u16 {
        self.bg_colors[color_index as usize]
    }

    #[inline]
    pub fn get_obj16(&self, palette_index: u8, color_index: u8) -> u16 {
        debug_assert!(palette_index < 16, "palette index must be less than 16");
        debug_assert!(color_index < 16, "color index must be less than 16");
        self.obj_colors[((palette_index * 16) + color_index) as usize]
    }

    #[inline]
    pub fn get_obj256(&self, color_index: u8) -> u16 {
        self.obj_colors[color_index as usize]
    }
}

// pub fn u16_to_pixel(p16: u16) -> u32 {
//     // (
//     //     (( p16        & 0x1F) as u8) * 8,
//     //     (((p16 >>  5) & 0x1F) as u8) * 8,
//     //     (((p16 >> 10) & 0x1F) as u8) * 8,
//     // )

//     let mut r = (p16 & 0x1F) as u32;
//     let mut g = ((p16 >>  5) & 0x1F) as u32;
//     let mut b = ((p16 >> 10) & 0x1F) as u32;

//     r = ((r * 255) / 31) & 0xFF;
//     r = ((g * 255) / 31) & 0xFF;
//     b = ((b * 255) / 31) & 0xFF;

//     r | (g << 8) | (b << 16)
// }
