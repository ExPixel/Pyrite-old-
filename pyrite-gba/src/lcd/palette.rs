pub struct GbaPalette {
     bg: [u16; 256],
    obj: [u16; 256],
}

impl GbaPalette {
    pub const fn new() -> GbaPalette {
        GbaPalette {
             bg: [0u16; 256],
            obj: [0u16; 256],
        }
    }

    pub fn backdrop(&self) -> u16 {
        self.bg[0] | 0x8000
    }

    pub fn bg256(&self, index: usize) -> u16 {
        return self.bg[index] | 0x8000;
    }

    pub fn obj256(&self, index: usize) -> u16 {
        return self.bg[index] | 0x8000;
    }

    pub fn bg16(&self, palette: usize, index: usize) -> u16 {
        return self.bg[palette*16 + index] | 0x8000;
    }

    pub fn obj16(&self, palette: usize, index: usize) -> u16 {
        return self.obj[palette*16 + index] | 0x8000;
    }

    pub fn write32(&mut self, offset: usize, value: u32) {
        self.write16(offset, value as u16);
        self.write16(offset + 2, (value >> 16) as u16);
    }

    pub fn write16(&mut self, offset: usize, value: u16) {
        if offset < 0x200 { self.bg[offset / 2] = value; return; }
        if offset < 0x400 { self.obj[(offset  - 0x200) / 2] = value; return; }
    }

    pub fn write8(&mut self, offset: usize, value: u8) {
        let halfword = self.read16(offset & !1);
        let shift = (offset & 1) << 3;
        self.write16(offset & !1, (halfword & (0xFF00 >> shift)) | ((value as u16) << shift));
    }

    pub fn read32(&self, offset: usize) -> u32 {
        let lo = self.read16(offset) as u32;
        let hi = self.read16(offset + 2) as u32;
        return lo | (hi << 16);
    }

    pub fn read16(&self, offset: usize) -> u16 {
        if offset < 0x200 { return  self.bg[offset / 2]; }
        if offset < 0x400 { return self.obj[(offset - 0x200) / 2]; }
        return 0;
    }

    pub fn read8(&self, offset: usize) -> u8 {
        let halfword = self.read16(offset & !1);
        let shift = (offset & 1) << 3;
        return (halfword >> shift) as u8;
    }
}
