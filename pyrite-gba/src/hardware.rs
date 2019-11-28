use crate::ioregs;
use crate::sysctl;
use crate::lcd::GbaLCD;
use crate::lcd::palette::GbaPalette;
use crate::keypad::GbaKeypad;
use crate::util::memory::*;

use pyrite_arm::memory::ArmMemory;
use sysctl::GbaSystemControl;

use pyrite_common::{ bits_b };

pub const CART0: u32 = 0x08000000;
pub const CART1: u32 = 0x0A000000;
pub const CART2: u32 = 0x0C000000;

pub type BIOS   = [u8;  16 * 1024];
pub type EWRAM  = [u8; 256 * 1024];
pub type IWRAM  = [u8;  32 * 1024];
pub type VRAM   = [u8;  96 * 1024];
pub type OAM    = [u8;   1 * 1024];

pub struct GbaHardware {
    // garden variety memory:
    pub(crate) bios:            [u8;  16 * 1024],
    pub(crate) ewram:           [u8; 256 * 1024],
    pub(crate) iwram:           [u8;  32 * 1024],
    pub(crate) vram:            [u8;  96 * 1024],
    pub(crate) oam:             [u8;   1 * 1024],
    pub(crate) pal:             GbaPalette,
    pub(crate) gamepak:         Vec<u8>,
    pub(crate) sram:            SRAM,

    // :o
    pub(crate) sysctl:  GbaSystemControl,
    pub(crate) ramctl:  GbaRAMControl,
    pub lcd:     GbaLCD,
    pub keypad:  GbaKeypad,

    /// This singular purpose of this is to make 8bit writes to larger IO registers consistent by
    /// storing the values that were last written to them.
    ioreg_bytes:                [u8; 0x20A],

    /// Last code read with read_code_halfword or read_code_word.
    last_code_read:     u32,

    /// If this is set to true, reading from the BIOS will be allowed.
    /// If this is false, all reads from the BIOS will return the last read opcode.
    allow_bios_access:  bool,
}

impl GbaHardware {
    pub fn new() -> GbaHardware {
        GbaHardware {
            bios:           [0u8;  16 * 1024],
            ewram:          [0u8; 256 * 1024],
            iwram:          [0u8;  32 * 1024],
            vram:           [0u8;  96 * 1024],
            oam:            [0u8;  1 * 1024],
            pal:            GbaPalette::new(),
            gamepak:        Vec::new(),
            sram:           SRAM::new(),

            sysctl:         GbaSystemControl::new(),
            ramctl:         GbaRAMControl::new(),
            lcd:            GbaLCD::new(),
            keypad:         GbaKeypad::new(),

            ioreg_bytes:    [0u8; 0x20A],
            last_code_read: 0,

            // @TODO implement
            allow_bios_access: true,
        }
    }

    pub fn set_bios_rom(&mut self, data: &[u8]) {
        assert!(data.len() <= 1024 * 16, "BIOS length cannot be greater than 16KB");
        (&mut self.bios[0..data.len()]).copy_from_slice(data);
    }

    pub fn set_gamepak_rom(&mut self, data: Vec<u8>) {
        self.gamepak = data;
    }

    pub fn read32(&self, addr: u32) -> u32 {
        let addr = addr & 0xFFFFFFFC; // word align the address

        match region_of(addr) {
            0x00 => self.bios_read32(addr),
            0x01 => { Self::bad_read(32, addr, "unused region 0x01"); self.last_code_read },
            0x02 => {
                if self.ramctl.disabled {
                    self.last_code_read
                } else if self.ramctl.external {
                    read_u32(&self.ewram, addr as usize % (256 * 1024))
                } else {
                    read_u32(&self.iwram, addr as usize % ( 32 * 1024))
                }
            }

            0x03 => {
                if self.ramctl.disabled {
                    self.last_code_read
                } else {
                    read_u32(&self.iwram, addr as usize % ( 32 * 1024))
                }
            }

            0x04 => self.io_read32(addr),
            0x05 => self.pal.read32(addr as usize % (1 * 1024)),
            0x06 => read_u32(&self.vram, Self::vram_off(addr)),
            0x07 => read_u32(&self.oam, addr as usize % (  1 * 1024)),
            0x08 | 0x09 => self.gamepak_read32(addr, CART0),
            0x0A | 0x0B => self.gamepak_read32(addr, CART1),
            0x0C | 0x0D => self.gamepak_read32(addr, CART2),
            0x0E => self.sram.read32(addr).unwrap_or(self.last_code_read),
            0x0F => { Self::bad_read(32, addr, "unused region 0x0F"); self.last_code_read },
            _ => unreachable!(),
        }
    }

    pub fn read16(&self, addr: u32) -> u16 {
        let addr = addr & 0xFFFFFFFE; // halfword align the address


        match region_of(addr) {
            0x00 => self.bios_read16(addr),
            0x01 => { Self::bad_read(16, addr, "unused region 0x01"); halfword_of_word(self.last_code_read, addr) }
            0x02 if !self.ramctl.disabled && self.ramctl.external => read_u16(&self.ewram, addr as usize % (256 * 1024)),
            0x02 if !self.ramctl.disabled => read_u16(&self.iwram, addr as usize % ( 32 * 1024)),
            0x03 if !self.ramctl.disabled => read_u16(&self.iwram, addr as usize % ( 32 * 1024)),
            0x05 => self.pal.read16(addr as usize % (1 * 1024)),
            0x06 => read_u16(&self.vram, Self::vram_off(addr)),
            0x07 => read_u16(&self.oam, addr as usize % (  1 * 1024)),
            0x04 => self.io_read16(addr),
            0x08 | 0x09 => self.gamepak_read16(addr, CART0),
            0x0A | 0x0B => self.gamepak_read16(addr, CART1),
            0x0C | 0x0D => self.gamepak_read16(addr, CART2),
            0x0E => self.sram.read16(addr).unwrap_or(halfword_of_word(self.last_code_read, addr)),
            0x0F => { Self::bad_read(16, addr, "unused region 0x0F"); halfword_of_word(self.last_code_read, addr) }
            _ => unreachable!(),
        }
    }

    pub fn read8(&self, addr: u32) -> u8 {
        match region_of(addr) {
            0x00 => self.bios_read8(addr),
            0x01 => { Self::bad_read(8, addr, "unused region 0x01"); byte_of_word(self.last_code_read, addr) }
            0x02 if !self.ramctl.disabled && self.ramctl.external => self.ewram[addr as usize % (256 * 1024)],
            0x02 if !self.ramctl.disabled => self.iwram[addr as usize % ( 32 * 1024)],
            0x03 if !self.ramctl.disabled => self.iwram[addr as usize % ( 32 * 1024)],
            0x05 => self.pal.read8(addr as usize % (1 * 1024)),
            0x07 => self.oam[addr as usize % (1 * 1024)],
            0x04 => self.io_read8(addr),
            0x08 | 0x09 => self.gamepak_read8(addr, CART0),
            0x0A | 0x0B => self.gamepak_read8(addr, CART1),
            0x0C | 0x0D => self.gamepak_read8(addr, CART2),
            0x0E => self.sram.read8(addr).unwrap_or(byte_of_word(self.last_code_read, addr)),
            0x0F => { Self::bad_read(8, addr, "unused region 0x0F"); byte_of_word(self.last_code_read, addr) }
            _ => unreachable!(),
        }
    }

    pub fn write32(&mut self, addr: u32, data: u32) -> bool {
        let addr = addr & 0xFFFFFFFC; // word align the address

        match region_of(addr) {
            0x02 if !self.ramctl.disabled &&  self.ramctl.external => write_u32(&mut self.ewram, addr as usize % (256 * 1024), data),
            0x02 if !self.ramctl.disabled => write_u32(&mut self.iwram, addr as usize % ( 32 * 1024), data),
            0x03 if !self.ramctl.disabled => write_u32(&mut self.iwram, addr as usize % ( 32 * 1024), data),
            0x04 => return self.io_write32(addr, data),
            0x05 => self.pal.write32(addr as usize % (1 * 1024), data),
            0x06 => write_u32(&mut self.vram, Self::vram_off(addr), data),
            0x07 => write_u32(&mut self.oam, addr as usize % (1 * 1024), data),
            0x08 | 0x09 => return self.gamepak_write32(addr, data, CART0),
            0x0A | 0x0B => return self.gamepak_write32(addr, data, CART1),
            0x0C | 0x0D => return self.gamepak_write32(addr, data, CART2),
            0x0E => return self.sram.write32(addr, data),
            _ => {
                eprintln!("out of range 32-bit write to memory address 0x{:08X}", addr);
                return false;
            }
        }
        return true;
    }

    pub fn write16(&mut self, addr: u32, data: u16) -> bool {
        let addr = addr & 0xFFFFFFFE; // halfword align the address

        match region_of(addr) {
            0x02 if !self.ramctl.disabled &&  self.ramctl.external => write_u16(&mut self.ewram, addr as usize % (256 * 1024), data),
            0x02 if !self.ramctl.disabled => write_u16(&mut self.iwram, addr as usize % ( 32 * 1024), data),
            0x03 if !self.ramctl.disabled => write_u16(&mut self.iwram, addr as usize % ( 32 * 1024), data),
            0x04 => return self.io_write16(addr, data),
            0x05 => self.pal.write16(addr as usize % (1 * 1024), data),
            0x06 => write_u16(&mut self.vram, Self::vram_off(addr), data),
            0x07 => write_u16(&mut self.oam, addr as usize % (1 * 1024), data),
            0x08 | 0x09 => return self.gamepak_write16(addr, data, CART0),
            0x0A | 0x0B => return self.gamepak_write16(addr, data, CART1),
            0x0C | 0x0D => return self.gamepak_write16(addr, data, CART2),
            0x0E => return self.sram.write16(addr, data),
            _ => {
                eprintln!("out of range 16-bit write to memory address 0x{:08X}", addr);
                return false;
            }
        }
        return true;
    }

    pub fn write8(&mut self, addr: u32, data: u8) -> bool {
        let addr = addr & 0xFFFFFFFE; // halfword align the address

        match region_of(addr) {
            0x02 if !self.ramctl.disabled &&  self.ramctl.external => self.ewram[addr as usize % (256 * 1024)] = data,
            0x02 if !self.ramctl.disabled => self.iwram[addr as usize % ( 32 * 1024)] = data,
            0x03 if !self.ramctl.disabled => self.iwram[addr as usize % ( 32 * 1024)] = data,
            0x04 => return self.io_write8(addr, data),
            0x05 => self.pal.write8(addr as usize % (1 * 1024), data),
            0x07 => self.oam[addr as usize % (1 * 1024)] = data,
            0x08 | 0x09 => return self.gamepak_write8(addr, data, CART0),
            0x0A | 0x0B => return self.gamepak_write8(addr, data, CART1),
            0x0C | 0x0D => return self.gamepak_write8(addr, data, CART2),
            0x0E => return self.sram.write8(addr, data),
            _ => {
                eprintln!("out of range 16-bit write to memory address 0x{:08X}", addr);
                return false;
            }
        }
        return true;
    }

    fn bios_read32(&self, addr: u32) -> u32 {
        if self.allow_bios_access && addr <= (16 * 1024 - 4) {
            read_u32(&self.bios, addr as usize)
        } else {
            self.last_code_read
        }
    }

    fn bios_read16(&self, addr: u32) -> u16 {
        if self.allow_bios_access && addr <= (16 * 1024 - 4) {
            read_u16(&self.bios, addr as usize)
        } else {
            halfword_of_word(self.last_code_read, addr)
        }
    }

    fn bios_read8(&self, addr: u32) -> u8 {
        if self.allow_bios_access && addr <= (16 * 1024 - 4) {
            self.bios[addr as usize]
        } else {
            byte_of_word(self.last_code_read, addr)
        }
    }

    fn gamepak_read32(&self, addr: u32, cart_offset: u32) -> u32 {
        let offset = (addr - cart_offset) as usize;
        if offset <= (self.gamepak.len() - 4) {
            unsafe {
                read_u32_unchecked(&self.gamepak, offset)
            }
        } else {
            let lo = (addr >> 1) & 0xFFFF;
            let hi = (lo + 1) & 0xFFFF;
            return lo | (hi << 16);
        }
    }

    fn gamepak_read16(&self, addr: u32, cart_offset: u32) -> u16 {
        let offset = (addr - cart_offset) as usize;
        if offset <= (self.gamepak.len() - 2) {
            unsafe {
                read_u16_unchecked(&self.gamepak, offset)
            }
        } else {
            return (addr >> 1) as u16;
        }
    }

    fn gamepak_read8(&self, addr: u32, cart_offset: u32) -> u8 {
        let offset = (addr - cart_offset) as usize;
        if offset <= (self.gamepak.len() - 2) {
            self.gamepak[offset]
        } else {
            byte_of_halfword((addr >> 1) as u16, addr)
        }
    }

    fn gamepak_write32(&mut self, addr: u32, _value: u32, _cart_offset: u32) -> bool {
        eprintln!("unimplemented 32-bit write to GamePak address 0x{:08X}", addr);
        false
    }

    fn gamepak_write16(&mut self, addr: u32, _value: u16, _cart_offset: u32) -> bool {
        eprintln!("unimplemented 16-bit write to GamePak address 0x{:08X}", addr);
        false
    }

    fn gamepak_write8(&mut self, addr: u32, _value: u8, _cart_offset: u32) -> bool {
        eprintln!("unimplemented 8-bit write to GamePak address 0x{:08X}", addr);
        false
    }

    fn io_read32(&self, addr: u32) -> u32 {
        // the address is 32-bit aligned by this point so adding 2 like this is safe.
        let offset_lo = Self::io_off(addr);
        let offset_hi = offset_lo + 2;

        match (self.io_read_reg(offset_lo), self.io_read_reg(offset_hi)) {
            (Some(lo), Some(hi)) => {
                return (lo as u32) | ((hi as u32) << 16);
            },

            (Some(lo), None) => {
                return lo as u32;
            },

            (None, Some(hi)) => {
                return (hi as u32) << 16;
            },

            (None, None) => {
                return self.last_code_read;
            },
        }
    }

    fn io_read16(&self, addr: u32) -> u16 {
        self.io_read_reg(Self::io_off(addr)).unwrap_or(halfword_of_word(self.last_code_read, addr))
    }

    fn  io_read8(&self, addr: u32) -> u8 {
        let offset = Self::io_off(addr);
        match offset {
            ioregs::POSTFLG => {
                if self.sysctl.reg_postflg {
                    1
                } else {
                    0
                }
            },

            offset => {
                let halfword_offset = offset & 0xFFFE;
                if let Some(halfword) = self.io_read_reg(halfword_offset) {
                    let shift = (offset & 1) << 3;
                    (halfword >> shift) as u8
                } else {
                    byte_of_word(self.last_code_read, addr)
                }
            },
        }
    }

    fn io_write32(&mut self, addr: u32, data: u32) -> bool {
        // the address is 32-bit aligned by this point so adding 2 like this is safe.
        let offset_lo = Self::io_off(addr);

        match offset_lo {
            ioregs::BG2X => {
                self.lcd.registers.bg2_affine_params.x = data;
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            },

            ioregs::BG2Y => {
                self.lcd.registers.bg2_affine_params.y = data;
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            },

            ioregs::BG3X => {
                self.lcd.registers.bg3_affine_params.x = data;
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            },

            ioregs::BG3Y => {
                self.lcd.registers.bg3_affine_params.y = data;
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            },

            _ => {
                let offset_hi = offset_lo + 2;
                let lo_write = self.io_write_reg(offset_lo, data as u16);
                let hi_write = self.io_write_reg(offset_hi, (data >> 16) as u16);
                return lo_write | hi_write;
            }
        }
        return true;
    }

    fn io_write16(&mut self, addr: u32, data: u16) -> bool {
        self.io_write_reg(Self::io_off(addr), data)
    }

    fn io_write8(&mut self, addr: u32, data:  u8) -> bool {
        const IMC_END: u16 = ioregs::IMC + 3;

        let offset = Self::io_off(addr);
        match offset {
            ioregs::POSTFLG => {
                self.sysctl.reg_postflg = (data & 1) != 0;
                return true;
            },

            ioregs::HALTCNT => {
                if (data & 1) == 0 {
                    self.sysctl.halt = true;
                } else {
                    self.sysctl.stop = true;
                }
                return true;
            },

            0x000..=0x208 => {
                let halfword_offset = offset & 0xFFFE;
                let mut halfword = read_u16(&self.ioreg_bytes, halfword_offset as usize);
                let shift = (offset & 1) << 3;
                halfword = (halfword & (0xFF00 >> shift)) | ((data as u16) << shift);
                return self.io_write_reg(halfword_offset, halfword);
            },

            _ => {
                return false;
            }
        }
    }

    fn io_write_reg(&mut self, offset: u16, data: u16) -> bool {
        /// Sets the 16bit value in a word.
        macro_rules! setw {
            ($Word:expr, $Value:expr) => {{
                let shift = (offset as u32 & 0x10) << 3;
                ($Word & !(0xFFFF << shift)) | (($Value as u32) << shift)
            }};

            ($Word:expr) => {{
                let shift = (offset as u32 & 0x10) << 3;
                ($Word & !(0xFFFF << shift)) | ((data as u32) << shift)
            }};
        }

        match offset {
            // LCD
            ioregs::DISPCNT => self.lcd.registers.dispcnt.value = data,
            ioregs::DISPSTAT => self.lcd.registers.set_dispstat(data),
            ioregs::BG0CNT => self.lcd.registers.bg_cnt[0].value = data,
            ioregs::BG1CNT => self.lcd.registers.bg_cnt[1].value = data,
            ioregs::BG2CNT => self.lcd.registers.bg_cnt[2].value = data,
            ioregs::BG3CNT => self.lcd.registers.bg_cnt[3].value = data,
            ioregs::BG0HOFS => self.lcd.registers.bg_ofs[0].x = data,
            ioregs::BG1HOFS => self.lcd.registers.bg_ofs[1].x = data,
            ioregs::BG2HOFS => self.lcd.registers.bg_ofs[2].x = data,
            ioregs::BG3HOFS => self.lcd.registers.bg_ofs[3].x = data,
            ioregs::BG0VOFS => self.lcd.registers.bg_ofs[0].y = data,
            ioregs::BG1VOFS => self.lcd.registers.bg_ofs[1].y = data,
            ioregs::BG2VOFS => self.lcd.registers.bg_ofs[2].y = data,
            ioregs::BG3VOFS => self.lcd.registers.bg_ofs[3].y = data,
            ioregs::BG2X => {
                self.lcd.registers.bg2_affine_params.x = setw!(self.lcd.registers.bg2_affine_params.x);
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            }
            ioregs::BG2X_HI     => {
                self.lcd.registers.bg2_affine_params.x = setw!(self.lcd.registers.bg2_affine_params.x);
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            },
            ioregs::BG2Y => {
                self.lcd.registers.bg2_affine_params.y = setw!(self.lcd.registers.bg2_affine_params.y);
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            }
            ioregs::BG2Y_HI => {
                self.lcd.registers.bg2_affine_params.y = setw!(self.lcd.registers.bg2_affine_params.y);
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            },

            ioregs::BG2PA => self.lcd.registers.bg2_affine_params.set_a(data),
            ioregs::BG2PB => self.lcd.registers.bg2_affine_params.set_a(data),
            ioregs::BG2PC => self.lcd.registers.bg2_affine_params.set_a(data),
            ioregs::BG2PD => self.lcd.registers.bg2_affine_params.set_a(data),

            ioregs::BG3X => {
                self.lcd.registers.bg3_affine_params.x = setw!(self.lcd.registers.bg3_affine_params.x);
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            }
            ioregs::BG3X_HI     => {
                self.lcd.registers.bg3_affine_params.x = setw!(self.lcd.registers.bg3_affine_params.x);
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            },
            ioregs::BG3Y => {
                self.lcd.registers.bg3_affine_params.y = setw!(self.lcd.registers.bg3_affine_params.y);
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            }
            ioregs::BG3Y_HI => {
                self.lcd.registers.bg3_affine_params.y = setw!(self.lcd.registers.bg3_affine_params.y);
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            },

            ioregs::BG3PA => self.lcd.registers.bg3_affine_params.set_a(data),
            ioregs::BG3PB => self.lcd.registers.bg3_affine_params.set_a(data),
            ioregs::BG3PC => self.lcd.registers.bg3_affine_params.set_a(data),
            ioregs::BG3PD => self.lcd.registers.bg3_affine_params.set_a(data),

            // Keypad Input
            ioregs::KEYCNT      => self.keypad.control = data,

            // System Control
            ioregs::WAITCNT => self.sysctl.set_reg_waitcnt(data),
            ioregs::IMC | ioregs::IMC_HI => {
                self.ramctl.set_reg_control(setw!(self.ramctl.reg_control, data));
                self.sysctl.update_ram_cycles(self.ramctl.reg_control);
            },

            _ => {
                // eprintln!("write to bad IO register offset 0x{:04X}", offset);
                return false;
            },
        }
        write_u16(&mut self.ioreg_bytes, offset as usize, data);
        return true;
    }

    fn io_read_reg(&self, offset: u16) -> Option<u16> {
        /// Sets the 16bit value in a word.
        macro_rules! getw {
            ($Word:expr) => {{
                let shift = (offset & 0x10) << 3;
                Some(($Word >> shift) as u16)
            }}
        }

        match offset {
            // LCD
            ioregs::DISPCNT     => Some(self.lcd.registers.dispcnt.value),
            ioregs::DISPSTAT    => Some(self.lcd.registers.dispstat.value),
            ioregs::VCOUNT      => Some(self.lcd.registers.line),
            ioregs::BG0CNT      => Some(self.lcd.registers.bg_cnt[0].value),
            ioregs::BG1CNT      => Some(self.lcd.registers.bg_cnt[0].value),
            ioregs::BG2CNT      => Some(self.lcd.registers.bg_cnt[0].value),
            ioregs::BG3CNT      => Some(self.lcd.registers.bg_cnt[0].value),
            ioregs::WININ       => Some(self.lcd.registers.winin.value()),
            ioregs::WINOUT      => Some(self.lcd.registers.winout.value()),
            ioregs::BLDCNT      => Some(self.lcd.registers.effects.value()),
            ioregs::BLDALPHA    => Some(self.lcd.registers.alpha),

            // Keypad Input
            ioregs::KEYINPUT    => Some(self.keypad.input),
            ioregs::KEYCNT      => Some(self.keypad.control),

            // System Control
            ioregs::WAITCNT => Some(self.sysctl.reg_waitcnt),
            ioregs::IMC => getw!(self.ramctl.reg_control),

            _ => {
                // eprintln!("read from bad IO register offset 0x{:04X}", offset);
                None
            }
        }
    }

    /// Converts an address into an offset into the IO registers (in the range 0x000 to 0x800)
    /// taking into account that address 0x04000800 is mirrored every 64K.
    fn io_off(addr: u32) -> u16 {
        if addr < 0x04000800 { return (addr & 0xFFF) as u16; }
        if (addr & 0xFF00FFFC) == 0x04000800 { return (addr & 0x0803) as u16; }
        return 0xFFFF;
    }

    /// Converts an address into an offset into VRAM taking into account VRAM's mirroring
    /// characteristics.
    fn vram_off(addr: u32) -> usize {
        // Even though VRAM is sized 96K (64K+32K), it is repeated in steps of 128K (64K+32K+32K, the two
        let vram128 = addr % (128 * 1024); // offset in a 128KB block

        if vram128 > (96 * 1024) {
            // this means that this address is in the later 32KB block so we just subtract 32KB to
            // mirror the previous one:
            vram128 as usize - (32 * 1024)
        } else {
            vram128 as usize
        }
    }

    #[cold]
    fn bad_read(bits: u8, addr: u32, message: &'static str) {
        println!("bad {}-bit read at 0x{:08X}: {}", bits, addr, message);
   }
}


impl ArmMemory for GbaHardware {
    fn on_internal_cycles(&mut self, _icycles: u32) {
        /* NOP */
    }

    fn read_code_word(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u32 {
        *cycles += self.sysctl.get_word_cycles(addr, seq);
        let code = self.read32(addr);
        self.last_code_read = code;
        return code;
    }

    fn read_code_halfword(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u16 {
        *cycles += self.sysctl.get_halfword_cycles(addr, seq);
        let code = self.read32(addr);
        self.last_code_read = code;
        return halfword_of_word(code, addr);
    }

    fn read_data_word(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u32 {
        *cycles += self.sysctl.get_word_cycles(addr, seq);
        return self.read32(addr);
    }

    fn read_data_halfword(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u16 {
        *cycles += self.sysctl.get_halfword_cycles(addr, seq);
        return self.read16(addr);
    }

    fn read_data_byte(&mut self, addr: u32, seq: bool, cycles: &mut u32) ->  u8 {
        *cycles += self.sysctl.get_byte_cycles(addr, seq);
        return self.read8(addr);
    }

    fn write_data_word(&mut self, addr: u32, data: u32, seq: bool, cycles: &mut u32) {
        *cycles += self.sysctl.get_word_cycles(addr, seq);
        self.write32(addr, data);
    }

    fn write_data_halfword(&mut self, addr: u32, data: u16, seq: bool, cycles: &mut u32) {
        *cycles += self.sysctl.get_halfword_cycles(addr, seq);
        self.write16(addr, data);
    }

    fn write_data_byte(&mut self, addr: u32, data:  u8, seq: bool, cycles: &mut u32) {
        *cycles += self.sysctl.get_byte_cycles(addr, seq);
        self.write8(addr, data);
    }

    fn     code_cycles_word(&mut self, addr: u32, seq: bool) -> u32 { self.sysctl.get_word_cycles(addr, seq) }
    fn code_cycles_halfword(&mut self, addr: u32, seq: bool) -> u32 { self.sysctl.get_halfword_cycles(addr, seq) }
}

#[inline(always)]
const fn set_byte_of_word(word: u32, value: u8, off: u32) -> u32 {
    let shift = (off as u32 & 0x3) << 3;
    (word & !(0xFF << shift)) | ((value as u32) << shift)
}

#[inline(always)]
const fn set_halfword_of_word(word: u32, value: u16, off: u32) -> u32 {
    let shift = (off as u32 & 0x10) << 3;
    (word & !(0xFFFF << shift)) | ((value as u32) << shift)
}

/// Select the first halfword or the second halfword a full 32-bit word depending on the given address.
#[inline(always)]
const fn halfword_of_word(word: u32, addr: u32) -> u16 {
    (word >> ((addr & 0x2) * 8)) as u16
}

/// Select a byte of a 32-bit word depending on the given address.
#[inline(always)]
const fn byte_of_word(word: u32, addr: u32) -> u8 {
    (word >> ((addr % 4) * 8)) as u8
}

/// Select a byte of a 16-bit word depending on the given address.
#[inline(always)]
const fn byte_of_halfword(halfword: u16, addr: u32) -> u8 {
    (halfword >> ((addr % 2) * 8)) as u8
}

pub struct GbaRAMControl {
    /// True if RAM is disabled.
    disabled:   bool,

    /// True if external work RAM is enabled.
    external:   bool,

    /// Memory control register.
    reg_control: u32,
}

impl GbaRAMControl {
    pub fn new() -> GbaRAMControl {
        GbaRAMControl {
            disabled: false,
            external: true,
            reg_control: 0,
        }
    }

    /// Called after the internal memory control has been updated and internal values need to be
    /// changed.
    #[inline]
    pub fn set_reg_control(&mut self, value: u32) {
        self.reg_control = value;
        self.disabled = bits_b!(self.reg_control, 0, 0);
        self.external = bits_b!(self.reg_control, 5, 5);
    }
}

pub struct SRAM( u32 );

impl SRAM {
    pub fn new() -> SRAM {
        SRAM ( 0 )
    }

    pub fn read32(&self, addr: u32) -> Option<u32> {
        eprintln!("unimplemented 32-bit read from SRAM address 0x{:08X}", addr);
        Some(0xC2C2C2C2)
    }

    pub fn read16(&self, addr: u32) -> Option<u16> {
        eprintln!("unimplemented 16-bit read from SRAM address 0x{:08X}", addr);
        Some(0xC2C2)
    }

    pub fn read8(&self, addr: u32) -> Option<u8> {
        eprintln!("unimplemented 8-bit read from SRAM address 0x{:08X}", addr);
        Some(0xC2)
    }

    pub fn write32(&mut self, addr: u32, data: u32) -> bool {
        eprintln!("unimplemented 32-bit write to SRAM address 0x{:08X} [data = 0x{:08X}]", addr, data);
        false
    }

    pub fn write16(&mut self, addr: u32, data: u16) -> bool {
        eprintln!("unimplemented 16-bit write to SRAM address 0x{:08X} [data = 0x{:04X}]", addr, data);
        false
    }

    pub fn write8(&mut self, addr: u32, data: u8) -> bool {
        eprintln!("unimplemented 8-bit write to SRAM address 0x{:08X} [data = 0x{:02X}]", addr, data);
        false
    }
}

#[inline(always)]
pub const fn region_of(addr: u32) -> u32 {
    (addr >> 24) & 0xF
}
