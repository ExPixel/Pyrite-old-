use crate::dma::{DMAChannelIndex, GbaDMA};
use crate::ioregs;
use crate::irq::GbaInterruptControl;
use crate::keypad::GbaKeypad;
use crate::lcd::palette::GbaPalette;
use crate::lcd::GbaLCD;
use crate::sysctl::GbaSystemControl;
use crate::timers::{GbaTimers, TimerIndex};
use crate::util::memory::*;
use pyrite_arm::memory::ArmMemory;
use pyrite_common::bits_b;

pub type BIOS = [u8; 16 * 1024];
pub type EWRAM = [u8; 256 * 1024];
pub type IWRAM = [u8; 32 * 1024];
pub type VRAM = [u8; 96 * 1024];
pub type OAM = [u8; 1 * 1024];

pub struct GbaHardware {
    // garden variety memory:
    pub(crate) bios: BIOS,
    pub(crate) ewram: EWRAM,
    pub(crate) iwram: IWRAM,
    pub(crate) vram: VRAM,
    pub(crate) oam: OAM,
    pub(crate) pal: GbaPalette,
    pub(crate) gamepak: Vec<u8>,

    pub(crate) sysctl: GbaSystemControl,
    pub(crate) ramctl: GbaRAMControl,
    pub lcd: GbaLCD,
    pub keypad: GbaKeypad,
    pub irq: GbaInterruptControl,
    pub dma: GbaDMA,
    pub timers: GbaTimers,

    pub(crate) events: HardwareEventQueue,

    /// This singular purpose of this is to make 8bit writes to larger IO registers consistent by
    /// storing the values that were last written to them.
    ioreg_bytes: [u8; 0x20C],

    /// Last code read with read_code_halfword or read_code_word.
    last_code_read: u32,

    /// If this is set to true, reading from the BIOS will be allowed.
    /// If this is false, all reads from the BIOS will return the last read opcode.
    allow_bios_access: bool,

    /// Using a cell here just allows me to mutate this bool without redoing the entire memory API.
    /// This flag is set after any bad access. Reading this flag via `pop_bad_access` will clear
    /// it.
    bad_access: std::cell::Cell<bool>,
}

impl GbaHardware {
    pub fn new() -> GbaHardware {
        GbaHardware {
            bios: [0u8; 16 * 1024],
            ewram: [0u8; 256 * 1024],
            iwram: [0u8; 32 * 1024],
            vram: [0u8; 96 * 1024],
            oam: [0u8; 1 * 1024],
            pal: GbaPalette::new(),
            gamepak: Vec::new(),

            sysctl: GbaSystemControl::new(),
            ramctl: GbaRAMControl::new(),
            lcd: GbaLCD::new(),
            keypad: GbaKeypad::new(),
            irq: GbaInterruptControl::new(),
            dma: GbaDMA::new(),
            timers: GbaTimers::new(),

            events: HardwareEventQueue::new(),

            ioreg_bytes: [0u8; 0x20C],
            last_code_read: 0,

            // @TODO implement
            allow_bios_access: true,

            bad_access: std::cell::Cell::new(false),
        }
    }

    pub fn set_bios_rom(&mut self, data: &[u8]) {
        assert!(
            data.len() <= 1024 * 16,
            "BIOS length cannot be greater than 16KB"
        );
        (&mut self.bios[0..data.len()]).copy_from_slice(data);
    }

    pub fn set_gamepak_rom(&mut self, data: Vec<u8>) {
        self.gamepak = data;
    }

    pub fn view32(&self, addr: u32) -> u32 {
        const BAD_VALUE: u32 = 0xDEADBEEF;

        let addr = addr & 0xFFFFFFFC; // word align the address

        match Region::from_address(addr) {
            Region::BIOS => {
                if addr < 0x4000 {
                    self.bios_read32(addr)
                } else {
                    BAD_VALUE
                }
            }
            Region::Unused0x1 => BAD_VALUE,
            Region::ExternalRAM => {
                if self.ramctl.disabled {
                    BAD_VALUE
                } else if self.ramctl.external {
                    read_u32(&self.ewram, addr as usize % (256 * 1024))
                } else {
                    read_u32(&self.iwram, addr as usize % (32 * 1024))
                }
            }

            Region::InternalRAM => {
                if self.ramctl.disabled {
                    BAD_VALUE
                } else {
                    read_u32(&self.iwram, addr as usize % (32 * 1024))
                }
            }

            Region::IORegisters => self.io_read32(addr, false),
            Region::Palette => self.pal.read32(addr as usize % (1 * 1024)),
            Region::VRAM => read_u32(&self.vram, Self::vram_off(addr)),
            Region::OAM => read_u32(&self.oam, addr as usize % (1 * 1024)),
            Region::GamePak0Lo
            | Region::GamePak0Hi
            | Region::GamePak1Lo
            | Region::GamePak1Hi
            | Region::GamePak2Lo
            | Region::GamePak2Hi => self.gamepak_read32(addr, false),
            Region::SRAM => BAD_VALUE,
            Region::Unused0xF => BAD_VALUE,
        }
    }

    pub fn view16(&self, addr: u32) -> u16 {
        pub const BAD_VALUE: u16 = 0xDEAD;

        let addr = addr & 0xFFFFFFFE; // halfword align the address

        match Region::from_address(addr) {
            Region::BIOS => {
                if addr < 0x4000 {
                    self.bios_read16(addr)
                } else {
                    BAD_VALUE
                }
            }
            Region::Unused0x1 => BAD_VALUE,
            Region::ExternalRAM => {
                if self.ramctl.disabled {
                    BAD_VALUE
                } else if self.ramctl.external {
                    read_u16(&self.ewram, addr as usize % (256 * 1024))
                } else {
                    read_u16(&self.iwram, addr as usize % (32 * 1024))
                }
            }
            Region::InternalRAM => {
                if self.ramctl.disabled {
                    BAD_VALUE
                } else {
                    read_u16(&self.iwram, addr as usize % (32 * 1024))
                }
            }
            Region::IORegisters => self.io_read16(addr, false),
            Region::Palette => self.pal.read16(addr as usize % (1 * 1024)),
            Region::VRAM => read_u16(&self.vram, Self::vram_off(addr)),
            Region::OAM => read_u16(&self.oam, addr as usize % (1 * 1024)),
            Region::GamePak0Lo
            | Region::GamePak0Hi
            | Region::GamePak1Lo
            | Region::GamePak1Hi
            | Region::GamePak2Lo
            | Region::GamePak2Hi => self.gamepak_read16(addr, false),
            Region::SRAM => BAD_VALUE,
            Region::Unused0xF => BAD_VALUE,
        }
    }

    pub fn view8(&self, addr: u32) -> u8 {
        const BAD_VALUE: u8 = 0xDE;

        match Region::from_address(addr) {
            Region::BIOS => {
                if addr < 0x4000 {
                    self.bios_read8(addr)
                } else {
                    BAD_VALUE
                }
            }
            Region::Unused0x1 => BAD_VALUE,
            Region::ExternalRAM => {
                if self.ramctl.disabled {
                    BAD_VALUE
                } else if self.ramctl.external {
                    self.ewram[addr as usize % (256 * 1024)]
                } else {
                    self.iwram[addr as usize % (32 * 1024)]
                }
            }
            Region::InternalRAM => {
                if self.ramctl.disabled {
                    BAD_VALUE
                } else {
                    self.iwram[addr as usize % (32 * 1024)]
                }
            }
            Region::IORegisters => self.io_read8(addr, false),
            Region::Palette => self.pal.read8(addr as usize % (1 * 1024)),
            Region::VRAM => self.vram[Self::vram_off(addr)],
            Region::OAM => self.oam[addr as usize % (1 * 1024)],
            Region::GamePak0Lo
            | Region::GamePak0Hi
            | Region::GamePak1Lo
            | Region::GamePak1Hi
            | Region::GamePak2Lo
            | Region::GamePak2Hi => self.gamepak_read8(addr, false),
            Region::SRAM => BAD_VALUE,
            Region::Unused0xF => BAD_VALUE,
        }
    }

    fn bios_read32(&self, addr: u32) -> u32 {
        if self.allow_bios_access && addr <= (16 * 1024 - 4) {
            read_u32(&self.bios, addr as usize)
        } else {
            self.bad_read(32, addr, "out of BIOS range or no permission");
            self.last_code_read
        }
    }

    fn bios_read16(&self, addr: u32) -> u16 {
        if self.allow_bios_access && addr <= (16 * 1024 - 4) {
            read_u16(&self.bios, addr as usize)
        } else {
            self.bad_read(16, addr, "out of BIOS range or no permission");
            halfword_of_word(self.last_code_read, addr)
        }
    }

    fn bios_read8(&self, addr: u32) -> u8 {
        if self.allow_bios_access && addr <= (16 * 1024 - 4) {
            self.bios[addr as usize]
        } else {
            self.bad_read(8, addr, "out of BIOS range or no permission");
            byte_of_word(self.last_code_read, addr)
        }
    }

    // #NOTE this function assumes that the address being passed to it is aligned to multiple of 4
    // bytes.
    fn gamepak_read32(&self, addr: u32, display_error: bool) -> u32 {
        let offset = (addr & 0x01FFFFFF) as usize;
        if offset > self.gamepak.len() {
            if display_error {
                self.bad_read(32, addr, "out of cartridge range");
            }
            let lo = (addr >> 1) & 0xFFFF;
            let hi = (lo + 1) & 0xFFFF;
            return lo | (hi << 16);
        }
        return unsafe { read_u32_unchecked(&self.gamepak, offset) };
    }

    // #NOTE this function assumes that the address being passed to it is aligned to a multiple of
    // 2 bytes.
    fn gamepak_read16(&self, addr: u32, display_error: bool) -> u16 {
        let offset = (addr & 0x01FFFFFF) as usize;
        if offset > self.gamepak.len() {
            if display_error {
                self.bad_read(16, addr, "out of cartridge range");
            }
            return (addr >> 1) as u16;
        }
        return unsafe { read_u16_unchecked(&self.gamepak, offset) };
    }

    fn gamepak_read8(&self, addr: u32, display_error: bool) -> u8 {
        let offset = (addr & 0x01FFFFFF) as usize;
        if offset <= (self.gamepak.len() - 2) {
            self.gamepak[offset]
        } else {
            if display_error {
                self.bad_read(8, addr, "out of cartridge range");
            }
            byte_of_halfword((addr >> 1) as u16, addr)
        }
    }

    #[cold]
    fn gamepak_write32(&mut self, addr: u32, value: u32, display_error: bool) -> bool {
        if display_error {
            self.bad_write(32, addr, value, "gamepak");
        }
        false
    }

    #[cold]
    fn gamepak_write16(&mut self, addr: u32, value: u16, display_error: bool) -> bool {
        if display_error {
            self.bad_write(16, addr, value as u32, "gamepak");
        }
        false
    }

    #[cold]
    fn gamepak_write8(&mut self, addr: u32, value: u8, display_error: bool) -> bool {
        if display_error {
            self.bad_write(8, addr, value as u32, "gamepak");
        }
        false
    }

    fn sram_read32(&self, addr: u32) -> u32 {
        self.bad_read(32, addr, "sram");
        0xC2C2C2C2
    }

    fn sram_read16(&self, addr: u32) -> u16 {
        self.bad_read(16, addr, "sram");
        0xC2C2
    }

    fn sram_read8(&self, addr: u32) -> u8 {
        self.bad_read(8, addr, "sram");
        0xC2
    }

    fn sram_write32(&mut self, addr: u32, value: u32) -> bool {
        self.bad_write(32, addr, value, "sram");
        false
    }

    fn sram_write16(&mut self, addr: u32, value: u16) -> bool {
        self.bad_write(16, addr, value as u32, "sram");
        false
    }

    fn sram_write8(&mut self, addr: u32, value: u8) -> bool {
        self.bad_write(8, addr, value as u32, "sram");
        false
    }

    fn io_read32(&self, addr: u32, display_error: bool) -> u32 {
        // the address is 32-bit aligned by this point so adding 2 like this is safe.
        let offset_lo = Self::io_off(addr);
        let offset_hi = offset_lo + 2;

        match (self.io_read_reg(offset_lo), self.io_read_reg(offset_hi)) {
            (Some(lo), Some(hi)) => {
                return (lo as u32) | ((hi as u32) << 16);
            }

            (Some(lo), None) => {
                return lo as u32;
            }

            (None, Some(hi)) => {
                return (hi as u32) << 16;
            }

            (None, None) => {
                if display_error {
                    self.bad_read(32, addr, "invalid IO register");
                }
                return self.last_code_read;
            }
        }
    }

    fn io_read16(&self, addr: u32, display_error: bool) -> u16 {
        if let Some(value) = self.io_read_reg(Self::io_off(addr)) {
            value
        } else {
            if display_error {
                self.bad_read(16, addr, "invalid IO register");
            }
            halfword_of_word(self.last_code_read, addr)
        }
    }

    fn io_read8(&self, addr: u32, display_error: bool) -> u8 {
        let offset = Self::io_off(addr);
        match offset {
            ioregs::POSTFLG => {
                if self.sysctl.reg_postflg {
                    1
                } else {
                    0
                }
            }

            offset => {
                let halfword_offset = offset & 0xFFFE;
                if let Some(halfword) = self.io_read_reg(halfword_offset) {
                    let shift = (offset & 1) << 3;
                    (halfword >> shift) as u8
                } else {
                    if display_error {
                        self.bad_read(8, addr, "invalid IO register");
                    }
                    byte_of_word(self.last_code_read, addr)
                }
            }
        }
    }

    fn io_write32(&mut self, addr: u32, data: u32, display_error: bool) -> bool {
        // the address is 32-bit aligned by this point so adding 2 like this is safe.
        let offset_lo = Self::io_off(addr);

        match offset_lo {
            ioregs::BG2X => {
                self.lcd.registers.bg2_affine_params.set_x(data);
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            }

            ioregs::BG2Y => {
                self.lcd.registers.bg2_affine_params.set_y(data);
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            }

            ioregs::BG3X => {
                self.lcd.registers.bg3_affine_params.set_x(data);
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            }

            ioregs::BG3Y => {
                self.lcd.registers.bg3_affine_params.set_y(data);
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            }

            ioregs::DMA0SAD => {
                self.dma.channel_mut(DMAChannelIndex::DMA0).set_source(data);
            }

            ioregs::DMA1SAD => {
                self.dma.channel_mut(DMAChannelIndex::DMA1).set_source(data);
            }

            ioregs::DMA2SAD => {
                self.dma.channel_mut(DMAChannelIndex::DMA2).set_source(data);
            }

            ioregs::DMA3SAD => {
                self.dma.channel_mut(DMAChannelIndex::DMA3).set_source(data);
            }

            _ => {
                let offset_hi = offset_lo + 2;
                let lo_write = self.io_write_reg(offset_lo, data as u16);
                let hi_write = self.io_write_reg(offset_hi, (data >> 16) as u16);
                let success = lo_write | hi_write;

                if display_error && !success {
                    self.bad_write(32, addr, data, "invalid IO register");
                }

                return success;
            }
        }
        return true;
    }

    fn io_write16(&mut self, addr: u32, data: u16, display_error: bool) -> bool {
        let success = self.io_write_reg(Self::io_off(addr), data);
        if display_error && !success {
            self.bad_write(16, addr, data as u32, "invalid IO register");
        }
        return success;
    }

    fn io_write8(&mut self, addr: u32, data: u8, display_error: bool) -> bool {
        // const IMC_END: u16 = ioregs::IMC + 3;

        let offset = Self::io_off(addr);
        match offset {
            ioregs::POSTFLG => {
                self.sysctl.reg_postflg = (data & 1) != 0;
                return true;
            }

            ioregs::HALTCNT => {
                if (data & 1) == 0 {
                    self.events.push_halt_event();
                } else {
                    self.events.push_stop_event();
                }
                return true;
            }

            // @TODO make 8bit writes to internal memory control (0x800) possible as well
            0x000..=0x208 => {
                let halfword_offset = offset & 0xFFFE;
                let mut halfword = read_u16(&self.ioreg_bytes, halfword_offset as usize);
                let shift = (offset & 1) << 3;
                halfword = (halfword & (0xFF00 >> shift)) | ((data as u16) << shift);
                let success = self.io_write_reg(halfword_offset, halfword);

                if display_error && !success {
                    self.bad_write(8, addr, data as u32, "invalid IO register");
                }
                return success;
            }

            _ => {
                return false;
            }
        }
    }

    fn io_write_reg(&mut self, offset: u16, data: u16) -> bool {
        /// Sets the 16bit value in a word.
        macro_rules! setw {
            ($Word:expr, $Value:expr) => {{
                let shift = (offset as u32 & 0x2) << 3;
                ($Word & !(0xFFFF << shift)) | (($Value as u32) << shift)
            }};

            ($Word:expr) => {{
                let shift = (offset as u32 & 0x2) << 3;
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
                self.lcd.registers.bg2_affine_params.set_x_lo(data);
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            }
            ioregs::BG2X_HI => {
                self.lcd.registers.bg2_affine_params.set_x_hi(data);
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            }
            ioregs::BG2Y => {
                self.lcd.registers.bg2_affine_params.set_y_lo(data);
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            }
            ioregs::BG2Y_HI => {
                self.lcd.registers.bg2_affine_params.set_y_hi(data);
                self.lcd.registers.bg2_affine_params.copy_reference_points();
            }

            ioregs::BG2PA => self.lcd.registers.bg2_affine_params.set_a(data),
            ioregs::BG2PB => self.lcd.registers.bg2_affine_params.set_b(data),
            ioregs::BG2PC => self.lcd.registers.bg2_affine_params.set_c(data),
            ioregs::BG2PD => self.lcd.registers.bg2_affine_params.set_d(data),

            ioregs::BG3X => {
                self.lcd.registers.bg3_affine_params.set_x_lo(data);
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            }
            ioregs::BG3X_HI => {
                self.lcd.registers.bg3_affine_params.set_x_hi(data);
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            }
            ioregs::BG3Y => {
                self.lcd.registers.bg3_affine_params.set_y_lo(data);
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            }
            ioregs::BG3Y_HI => {
                self.lcd.registers.bg3_affine_params.set_y_hi(data);
                self.lcd.registers.bg3_affine_params.copy_reference_points();
            }

            ioregs::BG3PA => self.lcd.registers.bg3_affine_params.set_a(data),
            ioregs::BG3PB => self.lcd.registers.bg3_affine_params.set_b(data),
            ioregs::BG3PC => self.lcd.registers.bg3_affine_params.set_c(data),
            ioregs::BG3PD => self.lcd.registers.bg3_affine_params.set_d(data),

            // Windows
            ioregs::WIN0H => self.lcd.registers.win0_bounds.set_h(data),
            ioregs::WIN1H => self.lcd.registers.win1_bounds.set_h(data),
            ioregs::WIN0V => self.lcd.registers.win0_bounds.set_v(data),
            ioregs::WIN1V => self.lcd.registers.win1_bounds.set_v(data),
            ioregs::WININ => self.lcd.registers.winin.set_value(data),
            ioregs::WINOUT => self.lcd.registers.winout.set_value(data),

            // Special Effects
            ioregs::MOSAIC => {
                self.lcd.registers.mosaic.set_value(data);
            }
            ioregs::MOSAIC_HI => { /* NOP */ }
            ioregs::BLDCNT => self.lcd.registers.effects.set_value(data),
            ioregs::BLDALPHA => self.lcd.registers.alpha = data,
            ioregs::BLDY => self.lcd.registers.brightness = data,

            // Keypad Input
            ioregs::KEYCNT => self.keypad.control = data,

            // System Control
            ioregs::WAITCNT => self.sysctl.set_reg_waitcnt(data),
            ioregs::IMC | ioregs::IMC_HI => {
                self.ramctl
                    .set_reg_control(setw!(self.ramctl.reg_control, data));
                self.sysctl.update_ram_cycles(self.ramctl.reg_control);
            }

            // Interrupt Control
            ioregs::IME => {
                self.irq.master_enable = (data & 1) != 0;
            }
            ioregs::IME_HI => { /* NOP */ }
            ioregs::IE => self.irq.enabled = data,
            ioregs::IF => self.irq.write_if(data),

            // DMA 0
            ioregs::DMA0SAD => self
                .dma
                .channel_mut(DMAChannelIndex::DMA0)
                .set_source_lo(data),
            ioregs::DMA0SAD_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA0)
                .set_source_hi(data),
            ioregs::DMA0DAD => self
                .dma
                .channel_mut(DMAChannelIndex::DMA0)
                .set_destination_lo(data),
            ioregs::DMA0DAD_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA0)
                .set_destination_hi(data),
            ioregs::DMA0CNT_L => self.dma.channel_mut(DMAChannelIndex::DMA0).set_count(data),
            ioregs::DMA0CNT_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA0)
                .set_control(data, &mut self.events),

            // DMA 1
            ioregs::DMA1SAD => self
                .dma
                .channel_mut(DMAChannelIndex::DMA1)
                .set_source_lo(data),
            ioregs::DMA1SAD_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA1)
                .set_source_hi(data),
            ioregs::DMA1DAD => self
                .dma
                .channel_mut(DMAChannelIndex::DMA1)
                .set_destination_lo(data),
            ioregs::DMA1DAD_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA1)
                .set_destination_hi(data),
            ioregs::DMA1CNT_L => self.dma.channel_mut(DMAChannelIndex::DMA1).set_count(data),
            ioregs::DMA1CNT_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA1)
                .set_control(data, &mut self.events),

            // DMA 2
            ioregs::DMA2SAD => self
                .dma
                .channel_mut(DMAChannelIndex::DMA2)
                .set_source_lo(data),
            ioregs::DMA2SAD_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA2)
                .set_source_hi(data),
            ioregs::DMA2DAD => self
                .dma
                .channel_mut(DMAChannelIndex::DMA2)
                .set_destination_lo(data),
            ioregs::DMA2DAD_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA2)
                .set_destination_hi(data),
            ioregs::DMA2CNT_L => self.dma.channel_mut(DMAChannelIndex::DMA2).set_count(data),
            ioregs::DMA2CNT_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA2)
                .set_control(data, &mut self.events),

            // DMA 3
            ioregs::DMA3SAD => self
                .dma
                .channel_mut(DMAChannelIndex::DMA3)
                .set_source_lo(data),
            ioregs::DMA3SAD_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA3)
                .set_source_hi(data),
            ioregs::DMA3DAD => self
                .dma
                .channel_mut(DMAChannelIndex::DMA3)
                .set_destination_lo(data),
            ioregs::DMA3DAD_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA3)
                .set_destination_hi(data),
            ioregs::DMA3CNT_L => self.dma.channel_mut(DMAChannelIndex::DMA3).set_count(data),
            ioregs::DMA3CNT_H => self
                .dma
                .channel_mut(DMAChannelIndex::DMA3)
                .set_control(data, &mut self.events),

            // TIMERS
            ioregs::TM0CNT_L => self.timers.write_timer_counter(TimerIndex::TM0, data),
            ioregs::TM0CNT_H => self.timers.write_timer_control(TimerIndex::TM0, data),
            ioregs::TM1CNT_L => self.timers.write_timer_counter(TimerIndex::TM1, data),
            ioregs::TM1CNT_H => self.timers.write_timer_control(TimerIndex::TM1, data),
            ioregs::TM2CNT_L => self.timers.write_timer_counter(TimerIndex::TM2, data),
            ioregs::TM2CNT_H => self.timers.write_timer_control(TimerIndex::TM2, data),
            ioregs::TM3CNT_L => self.timers.write_timer_counter(TimerIndex::TM3, data),
            ioregs::TM3CNT_H => self.timers.write_timer_control(TimerIndex::TM3, data),

            _ => {
                return false;
            }
        }
        write_u16(&mut self.ioreg_bytes, offset as usize, data);
        return true;
    }

    fn io_read_reg(&self, offset: u16) -> Option<u16> {
        macro_rules! getw {
            ($Word:expr) => {{
                let shift = (offset & 0x2) << 3;
                Some(($Word >> shift) as u16)
            }};
        }

        match offset {
            // LCD
            ioregs::DISPCNT => Some(self.lcd.registers.dispcnt.value),
            ioregs::DISPSTAT => Some(self.lcd.registers.dispstat.value),
            ioregs::VCOUNT => Some(self.lcd.registers.line),
            ioregs::BG0CNT => Some(self.lcd.registers.bg_cnt[0].value),
            ioregs::BG1CNT => Some(self.lcd.registers.bg_cnt[1].value),
            ioregs::BG2CNT => Some(self.lcd.registers.bg_cnt[2].value),
            ioregs::BG3CNT => Some(self.lcd.registers.bg_cnt[3].value),
            ioregs::WININ => Some(self.lcd.registers.winin.value()),
            ioregs::WINOUT => Some(self.lcd.registers.winout.value()),
            ioregs::BLDCNT => Some(self.lcd.registers.effects.value()),
            ioregs::BLDALPHA => Some(self.lcd.registers.alpha),

            // Keypad Input
            ioregs::KEYINPUT => Some(self.keypad.input),
            ioregs::KEYCNT => Some(self.keypad.control),

            // System Control
            ioregs::WAITCNT => Some(self.sysctl.reg_waitcnt),
            ioregs::IMC => getw!(self.ramctl.reg_control),

            // DMA
            ioregs::DMA0CNT_H => Some(self.dma.channel(DMAChannelIndex::DMA0).control()),
            ioregs::DMA1CNT_H => Some(self.dma.channel(DMAChannelIndex::DMA1).control()),
            ioregs::DMA2CNT_H => Some(self.dma.channel(DMAChannelIndex::DMA2).control()),
            ioregs::DMA3CNT_H => Some(self.dma.channel(DMAChannelIndex::DMA3).control()),

            // TIMERS
            ioregs::TM0CNT_L => Some(self.timers.read_timer_counter(TimerIndex::TM0)),
            ioregs::TM0CNT_H => Some(self.timers.read_timer_control(TimerIndex::TM0)),
            ioregs::TM1CNT_L => Some(self.timers.read_timer_counter(TimerIndex::TM1)),
            ioregs::TM1CNT_H => Some(self.timers.read_timer_control(TimerIndex::TM1)),
            ioregs::TM2CNT_L => Some(self.timers.read_timer_counter(TimerIndex::TM2)),
            ioregs::TM2CNT_H => Some(self.timers.read_timer_control(TimerIndex::TM2)),
            ioregs::TM3CNT_L => Some(self.timers.read_timer_counter(TimerIndex::TM3)),
            ioregs::TM3CNT_H => Some(self.timers.read_timer_control(TimerIndex::TM3)),

            // Interrupt Control
            ioregs::IME => Some(self.irq.master_enable as u16),
            ioregs::IE => Some(self.irq.enabled),
            ioregs::IF => Some(self.irq.read_if()),
            _ => None,
        }
    }

    /// Converts an address into an offset into the IO registers (in the range 0x000 to 0x800)
    /// taking into account that address 0x04000800 is mirrored every 64K.
    fn io_off(addr: u32) -> u16 {
        if addr < 0x04000800 {
            return (addr & 0xFFF) as u16;
        }
        if (addr & 0xFF00FFFC) == 0x04000800 {
            return (addr & 0x0803) as u16;
        }
        return 0xFFFC;
    }

    /// Converts an address into an offset into VRAM taking into account VRAM's mirroring
    /// characteristics.
    fn vram_off(addr: u32) -> usize {
        // Even though VRAM is sized 96K (64K+32K), it is repeated in steps of 128K (64K+32K+32K,
        // the two 32K blocks itself being mirrors of each other).
        let vram128 = addr % (128 * 1024); // offset in a 128KB block

        if vram128 >= (96 * 1024) {
            // this means that this address is in the later 32KB block so we just subtract 32KB to
            // mirror the previous one:
            vram128 as usize - (32 * 1024)
        } else {
            vram128 as usize
        }
    }

    #[inline(never)]
    #[cold]
    fn bad_read(&self, bits: u8, addr: u32, message: &'static str) {
        self.bad_access.set(true);
        log::warn!("bad {}-bit read from 0x{:08X}: {}", bits, addr, message);
    }

    #[inline(never)]
    #[cold]
    fn bad_write(&self, bits: u8, addr: u32, value: u32, message: &'static str) {
        self.bad_access.set(true);
        log::warn!(
            "bad {}-bit write of value 0x{:X} to 0x{:08X}: {}",
            bits,
            value,
            addr,
            message
        );
    }

    pub fn pop_bad_access(&mut self) -> bool {
        self.bad_access.replace(false)
    }
}

impl ArmMemory for GbaHardware {
    fn on_internal_cycles(&mut self, _icycles: u32) {
        /* NOP */
    }

    fn read_code_word(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u32 {
        // TODO get rid of last_code_read somehow
        self.last_code_read = self.read_data_word(addr, seq, cycles);
        return self.last_code_read;
    }

    fn read_code_halfword(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u16 {
        self.last_code_read = self.read_data_word(addr, seq, cycles);
        return halfword_of_word(self.last_code_read, addr);
    }

    fn read_data_word(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u32 {
        let addr = addr & 0xFFFFFFFC; // word align the address

        match Region::from_address(addr) {
            Region::BIOS => {
                *cycles += 1;
                self.bios_read32(addr)
            }
            Region::Unused0x1 => {
                *cycles += 1;
                self.bad_read(32, addr, "unused region 0x01");
                self.last_code_read
            }
            Region::ExternalRAM => {
                if self.ramctl.disabled {
                    *cycles += 1;
                    self.bad_read(32, addr, "disabled RAM");
                    self.last_code_read
                } else if !self.ramctl.external {
                    *cycles += 1;
                    read_u32(&self.iwram, addr as usize % (32 * 1024))
                } else {
                    *cycles += self.sysctl.ram_cycles.word.get(true); // sequential and non-sequential are the same
                    read_u32(&self.ewram, addr as usize % (256 * 1024))
                }
            }

            Region::InternalRAM => {
                *cycles += 1;
                if self.ramctl.disabled {
                    self.bad_read(32, addr, "disabled RAM");
                    self.last_code_read
                } else {
                    read_u32(&self.iwram, addr as usize % (32 * 1024))
                }
            }

            Region::IORegisters => {
                *cycles += 1;
                self.io_read32(addr, true)
            }
            Region::Palette => {
                *cycles += 2;
                self.pal.read32(addr as usize % (1 * 1024))
            }
            Region::VRAM => {
                *cycles += 2;
                read_u32(&self.vram, Self::vram_off(addr))
            }
            Region::OAM => {
                *cycles += 1;
                read_u32(&self.oam, addr as usize % (1 * 1024))
            }
            Region::GamePak0Lo | Region::GamePak0Hi => {
                *cycles += self.sysctl.gamepak_cycles[0].word.get(seq);
                self.gamepak_read32(addr, true)
            }
            Region::GamePak1Lo | Region::GamePak1Hi => {
                *cycles += self.sysctl.gamepak_cycles[1].word.get(seq);
                self.gamepak_read32(addr, true)
            }
            Region::GamePak2Lo | Region::GamePak2Hi => {
                *cycles += self.sysctl.gamepak_cycles[2].word.get(seq);
                self.gamepak_read32(addr, true)
            }
            Region::SRAM => {
                *cycles += self.sysctl.sram_cycles.word.get(true); // same for seq and nonseq
                self.sram_read32(addr)
            }
            Region::Unused0xF => {
                *cycles += 1;
                self.bad_read(32, addr, "unused region 0x0F");
                self.last_code_read
            }
        }
    }

    fn read_data_halfword(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u16 {
        let addr = addr & 0xFFFFFFFE; // halfword align the address

        match Region::from_address(addr) {
            Region::BIOS => {
                *cycles += 1;
                self.bios_read16(addr)
            }
            Region::Unused0x1 => {
                *cycles += 1;
                self.bad_read(16, addr, "unused region 0x01");
                halfword_of_word(self.last_code_read, addr)
            }
            Region::ExternalRAM => {
                if self.ramctl.disabled {
                    *cycles += 1;
                    self.bad_read(16, addr, "disabled RAM");
                    halfword_of_word(self.last_code_read, addr)
                } else if self.ramctl.external {
                    *cycles += 1;
                    read_u16(&self.ewram, addr as usize % (256 * 1024))
                } else {
                    *cycles += self.sysctl.ram_cycles.halfword.get(true); // same timing for seq and nonseq
                    read_u16(&self.iwram, addr as usize % (32 * 1024))
                }
            }
            Region::InternalRAM => {
                *cycles += 1;
                if self.ramctl.disabled {
                    self.bad_read(16, addr, "disabled RAM");
                    halfword_of_word(self.last_code_read, addr)
                } else {
                    read_u16(&self.iwram, addr as usize % (32 * 1024))
                }
            }
            Region::IORegisters => {
                *cycles += 1;
                self.io_read16(addr, true)
            }
            Region::Palette => {
                *cycles += 1;
                self.pal.read16(addr as usize % (1 * 1024))
            }
            Region::VRAM => {
                *cycles += 1;
                read_u16(&self.vram, Self::vram_off(addr))
            }
            Region::OAM => {
                *cycles += 1;
                read_u16(&self.oam, addr as usize % (1 * 1024))
            }
            Region::GamePak0Lo | Region::GamePak0Hi => {
                *cycles += self.sysctl.gamepak_cycles[0].halfword.get(seq);
                self.gamepak_read16(addr, true)
            }
            Region::GamePak1Lo | Region::GamePak1Hi => {
                *cycles += self.sysctl.gamepak_cycles[1].halfword.get(seq);
                self.gamepak_read16(addr, true)
            }
            Region::GamePak2Lo | Region::GamePak2Hi => {
                *cycles += self.sysctl.gamepak_cycles[2].halfword.get(seq);
                self.gamepak_read16(addr, true)
            }
            Region::SRAM => {
                *cycles += self.sysctl.sram_cycles.halfword.get(true); // same for seq and nonseq
                self.sram_read16(addr)
            }
            Region::Unused0xF => {
                *cycles += 1;
                self.bad_read(16, addr, "unused region 0x0F");
                halfword_of_word(self.last_code_read, addr)
            }
        }
    }

    fn read_data_byte(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u8 {
        match Region::from_address(addr) {
            Region::BIOS => {
                *cycles += 1;
                self.bios_read8(addr)
            }
            Region::Unused0x1 => {
                *cycles += 1;
                self.bad_read(8, addr, "unused region 0x01");
                byte_of_word(self.last_code_read, addr)
            }
            Region::ExternalRAM => {
                if self.ramctl.disabled {
                    *cycles += 1;
                    self.bad_read(8, addr, "disabled RAM");
                    byte_of_word(self.last_code_read, addr)
                } else if !self.ramctl.external {
                    *cycles += 1;
                    self.iwram[addr as usize % (32 * 1024)]
                } else {
                    *cycles += self.sysctl.ram_cycles.byte.get(true); // same for seq and nonseq
                    self.ewram[addr as usize % (256 * 1024)]
                }
            }
            Region::InternalRAM => {
                *cycles += 1;
                if self.ramctl.disabled {
                    self.bad_read(8, addr, "disabled RAM");
                    byte_of_word(self.last_code_read, addr)
                } else {
                    self.iwram[addr as usize % (32 * 1024)]
                }
            }
            Region::IORegisters => {
                *cycles += 1;
                self.io_read8(addr, true)
            }
            Region::Palette => {
                *cycles += 1;
                self.pal.read8(addr as usize % (1 * 1024))
            }
            Region::VRAM => {
                *cycles += 1;
                self.vram[Self::vram_off(addr)]
            }
            Region::OAM => {
                *cycles += 1;
                self.oam[addr as usize % (1 * 1024)]
            }
            Region::GamePak0Lo | Region::GamePak0Hi => {
                *cycles += self.sysctl.gamepak_cycles[0].byte.get(seq);
                self.gamepak_read8(addr, true)
            }
            Region::GamePak1Lo | Region::GamePak1Hi => {
                *cycles += self.sysctl.gamepak_cycles[1].byte.get(seq);
                self.gamepak_read8(addr, true)
            }
            Region::GamePak2Lo | Region::GamePak2Hi => {
                *cycles += self.sysctl.gamepak_cycles[2].byte.get(seq);
                self.gamepak_read8(addr, true)
            }
            Region::SRAM => {
                *cycles += self.sysctl.sram_cycles.byte.get(true); // same for seq and nonseq
                self.sram_read8(addr)
            }
            Region::Unused0xF => {
                self.bad_read(8, addr, "unused region 0x0F");
                byte_of_word(self.last_code_read, addr)
            }
        }
    }

    fn write_data_word(&mut self, addr: u32, data: u32, seq: bool, cycles: &mut u32) {
        let addr = addr & 0xFFFFFFFC; // word align the address

        match Region::from_address(addr) {
            Region::ExternalRAM => {
                if self.ramctl.disabled {
                    *cycles += 1;
                    self.bad_write(32, addr, data, "disabled RAM");
                } else if !self.ramctl.external {
                    *cycles += 1;
                    write_u32(&mut self.iwram, addr as usize % (32 * 1024), data)
                } else {
                    *cycles += self.sysctl.ram_cycles.word.get(true); // same for seq and nonseq
                    write_u32(&mut self.ewram, addr as usize % (256 * 1024), data)
                }
            }
            Region::InternalRAM => {
                *cycles += 1;
                if self.ramctl.disabled {
                    self.bad_write(32, addr, data, "disabled RAM");
                } else {
                    write_u32(&mut self.iwram, addr as usize % (32 * 1024), data)
                }
            }
            Region::IORegisters => {
                *cycles += 1;
                self.io_write32(addr, data, true);
            }
            Region::Palette => {
                *cycles += 2;
                self.pal.write32(addr as usize % (1 * 1024), data)
            }
            Region::VRAM => {
                *cycles += 2;
                write_u32(&mut self.vram, Self::vram_off(addr), data)
            }
            Region::OAM => {
                *cycles += 1;
                write_u32(&mut self.oam, addr as usize % (1 * 1024), data)
            }
            Region::GamePak0Lo | Region::GamePak0Hi => {
                *cycles += self.sysctl.gamepak_cycles[0].word.get(seq);
                self.gamepak_write32(addr, data, true);
            }
            Region::GamePak1Lo | Region::GamePak1Hi => {
                *cycles += self.sysctl.gamepak_cycles[1].word.get(seq);
                self.gamepak_write32(addr, data, true);
            }
            Region::GamePak2Lo | Region::GamePak2Hi => {
                *cycles += self.sysctl.gamepak_cycles[2].word.get(seq);
                self.gamepak_write32(addr, data, true);
            }
            Region::SRAM => {
                *cycles += self.sysctl.sram_cycles.word.get(true); // same for seq and nonseq
                self.sram_write32(addr, data);
            }
            _ => {
                *cycles += 1;
                self.bad_write(32, addr, data, "out of range memory address");
            }
        }
    }

    fn write_data_halfword(&mut self, addr: u32, data: u16, seq: bool, cycles: &mut u32) {
        let addr = addr & 0xFFFFFFFE; // halfword align the address

        match Region::from_address(addr) {
            Region::ExternalRAM => {
                if self.ramctl.disabled {
                    *cycles += 1;
                    self.bad_write(16, addr, data as u32, "disabled RAM");
                } else if !self.ramctl.external {
                    *cycles += 1;
                    write_u16(&mut self.iwram, addr as usize % (32 * 1024), data);
                } else {
                    *cycles += self.sysctl.ram_cycles.halfword.get(true); // same for seq and nonseq
                    write_u16(&mut self.ewram, addr as usize % (256 * 1024), data);
                }
            }
            Region::InternalRAM => {
                *cycles += 1;
                if self.ramctl.disabled {
                    self.bad_write(16, addr, data as u32, "disabled RAM");
                } else {
                    write_u16(&mut self.iwram, addr as usize % (32 * 1024), data)
                }
            }
            Region::IORegisters => {
                *cycles += 1;
                self.io_write16(addr, data, true);
            }
            Region::Palette => {
                *cycles += 1;
                self.pal.write16(addr as usize % (1 * 1024), data)
            }
            Region::VRAM => {
                *cycles += 1;
                write_u16(&mut self.vram, Self::vram_off(addr), data)
            }
            Region::OAM => {
                *cycles += 1;
                write_u16(&mut self.oam, addr as usize % (1 * 1024), data)
            }
            Region::GamePak0Lo | Region::GamePak0Hi => {
                *cycles += self.sysctl.gamepak_cycles[0].halfword.get(seq);
                self.gamepak_write16(addr, data, true);
            }
            Region::GamePak1Lo | Region::GamePak1Hi => {
                *cycles += self.sysctl.gamepak_cycles[1].halfword.get(seq);
                self.gamepak_write16(addr, data, true);
            }
            Region::GamePak2Lo | Region::GamePak2Hi => {
                *cycles += self.sysctl.gamepak_cycles[2].halfword.get(seq);
                self.gamepak_write16(addr, data, true);
            }
            Region::SRAM => {
                *cycles += self.sysctl.sram_cycles.halfword.get(true); // same for seq and nonseq
                self.sram_write16(addr, data);
            }
            _ => {
                *cycles += 1;
                self.bad_write(16, addr, data as u32, "out of range memory address");
            }
        }
    }

    fn write_data_byte(&mut self, addr: u32, data: u8, seq: bool, cycles: &mut u32) {
        match Region::from_address(addr) {
            Region::ExternalRAM => {
                if self.ramctl.disabled {
                    *cycles += 1;
                    self.bad_write(8, addr, data as u32, "disabled RAM");
                } else if !self.ramctl.external {
                    *cycles += 1;
                    self.iwram[addr as usize % (32 * 1024)] = data
                } else {
                    *cycles += 1;
                    self.ewram[addr as usize % (256 * 1024)] = data
                }
            }
            Region::InternalRAM => {
                *cycles += 1;
                if self.ramctl.disabled {
                    self.bad_write(8, addr, data as u32, "disabled RAM");
                } else {
                    self.iwram[addr as usize % (32 * 1024)] = data
                }
            }
            Region::IORegisters => {
                *cycles += 1;
                self.io_write8(addr, data, true);
            }
            Region::Palette => {
                *cycles += 1;
                // Writes to BG (6000000h-600FFFFh) (or 6000000h-6013FFFh in Bitmap mode) and to
                // Palette (5000000h-50003FFh) are writing the new 8bit value to BOTH upper and
                // lower 8bits of the addressed halfword, ie. "[addr AND NOT 1]=data*101h".
                self.pal.write16(
                    (addr as usize & 0xFFFFFFFE) % (1 * 1024),
                    data as u16 * 0x101, // same as (data << 8) | data
                );
            }
            Region::VRAM => {
                *cycles += 1;
                // Writes to BG (6000000h-600FFFFh) (or 6000000h-6013FFFh in Bitmap mode) and to
                // Palette (5000000h-50003FFh) are writing the new 8bit value to BOTH upper and
                // lower 8bits of the addressed halfword, ie. "[addr AND NOT 1]=data*101h".
                if addr < 0x6014000 {
                    write_u16(
                        &mut self.vram,
                        Self::vram_off(addr) & 0xFFFFFFFE,
                        data as u16 * 0x101, // same as (data << 8) | data
                    );
                } else {
                    self.bad_write(8, addr, data as u32, "8-bit VRAM OBJ tiles write");
                }
            }
            Region::OAM => {
                // 8-bit writes to OAM are ignored
                *cycles += 1;
                self.bad_write(8, addr, data as u32, "8-bit OAM write");
                // self.oam[addr as usize % (1 * 1024)] = data
            }
            Region::GamePak0Lo | Region::GamePak0Hi => {
                *cycles += self.sysctl.gamepak_cycles[0].byte.get(seq);
                self.gamepak_write8(addr, data, true);
            }
            Region::GamePak1Lo | Region::GamePak1Hi => {
                *cycles += self.sysctl.gamepak_cycles[1].byte.get(seq);
                self.gamepak_write8(addr, data, true);
            }
            Region::GamePak2Lo | Region::GamePak2Hi => {
                *cycles += self.sysctl.gamepak_cycles[2].byte.get(seq);
                self.gamepak_write8(addr, data, true);
            }
            Region::SRAM => {
                *cycles += self.sysctl.sram_cycles.byte.get(true); // same for seq and nonseq
                self.sram_write8(addr, data);
            }
            _ => {
                *cycles += 1;
                self.bad_write(8, addr, data as u32, "out of range memory address");
            }
        }
    }

    fn view_word(&self, addr: u32) -> u32 {
        self.view32(addr)
    }
    fn view_halfword(&self, addr: u32) -> u16 {
        self.view16(addr)
    }
    fn view_byte(&self, addr: u32) -> u8 {
        self.view8(addr)
    }

    fn code_cycles_word(&mut self, addr: u32, seq: bool) -> u32 {
        match Region::from_address(addr) {
            Region::BIOS => 1,
            Region::Unused0x1 => 1,
            Region::ExternalRAM => {
                if self.ramctl.disabled || !self.ramctl.external {
                    1
                } else {
                    self.sysctl.ram_cycles.word.get(true) // sequential and non-sequential are the same
                }
            }
            Region::InternalRAM => 1,
            Region::IORegisters => 1,
            Region::Palette => 2,
            Region::VRAM => 2,
            Region::OAM => 1,
            Region::GamePak0Lo | Region::GamePak0Hi => self.sysctl.gamepak_cycles[0].word.get(seq),
            Region::GamePak1Lo | Region::GamePak1Hi => self.sysctl.gamepak_cycles[1].word.get(seq),
            Region::GamePak2Lo | Region::GamePak2Hi => self.sysctl.gamepak_cycles[2].word.get(seq),
            Region::SRAM => self.sysctl.sram_cycles.word.get(true), // same for seq and nonseq
            Region::Unused0xF => 1,
        }
    }
    fn code_cycles_halfword(&mut self, addr: u32, seq: bool) -> u32 {
        match Region::from_address(addr) {
            Region::BIOS => 1,
            Region::Unused0x1 => 1,
            Region::ExternalRAM => {
                if self.ramctl.disabled || !self.ramctl.external {
                    1
                } else {
                    self.sysctl.ram_cycles.halfword.get(true) // sequential and non-sequential are the same
                }
            }
            Region::InternalRAM => 1,
            Region::IORegisters => 1,
            Region::Palette => 1,
            Region::VRAM => 1,
            Region::OAM => 1,
            Region::GamePak0Lo | Region::GamePak0Hi => {
                self.sysctl.gamepak_cycles[0].halfword.get(seq)
            }
            Region::GamePak1Lo | Region::GamePak1Hi => {
                self.sysctl.gamepak_cycles[1].halfword.get(seq)
            }
            Region::GamePak2Lo | Region::GamePak2Hi => {
                self.sysctl.gamepak_cycles[2].halfword.get(seq)
            }
            Region::SRAM => self.sysctl.sram_cycles.word.get(true), // same for seq and nonseq
            Region::Unused0xF => 1,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// #[inline(always)]
// const fn set_byte_of_word(word: u32, value: u8, off: u32) -> u32 {
//     let shift = (off as u32 & 0x3) << 3;
//     (word & !(0xFF << shift)) | ((value as u32) << shift)
// }

// #[inline(always)]
// fn set_halfword_of_word(word: u32, value: u16, off: u32) -> u32 {
//     let shift = (off as u32 & 0x3) << 3;
//     (word & !(0xFFFF << shift)) | ((value as u32) << shift)
// }

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HardwareEvent {
    IRQ(crate::irq::Interrupt),
    DMA(crate::dma::DMAChannelIndex),
    Halt,
    Stop,
    None,
}

pub struct HardwareEventQueue {
    count: usize,
    pending: [HardwareEvent; 16],
}

impl HardwareEventQueue {
    pub fn new() -> HardwareEventQueue {
        HardwareEventQueue {
            count: 0,
            pending: [HardwareEvent::None; 16],
        }
    }

    #[inline]
    pub fn push_irq_event(&mut self, int: crate::irq::Interrupt) {
        self.push(HardwareEvent::IRQ(int));
    }

    #[inline]
    pub fn push_dma_event(&mut self, dma: crate::dma::DMAChannelIndex) {
        self.push(HardwareEvent::DMA(dma));
    }

    #[inline]
    pub fn push_halt_event(&mut self) {
        self.push(HardwareEvent::Halt);
    }

    #[inline]
    pub fn push_stop_event(&mut self) {
        self.push(HardwareEvent::Stop);
    }

    /// Push an event into the hardware event queue.
    #[inline]
    pub fn push(&mut self, event: HardwareEvent) {
        assert!(self.count < self.pending.len());
        self.pending[self.count] = event;
        self.count += 1;
    }

    /// @TODO: For now the return order for events is a bit weird and its expected that all events are
    /// going to be processed at once and we just pray that while processing events we don't fire
    /// enough to overfill the buffer. This would probably be solved if I could be bothered to use
    /// the CircularBuffer here instead of writing this comment :|
    #[inline]
    pub fn pop(&mut self) -> HardwareEvent {
        assert!(self.count > 0);
        self.count -= 1;
        std::mem::replace(&mut self.pending[self.count], HardwareEvent::None)
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }
}

pub struct GbaRAMControl {
    /// True if RAM is disabled.
    disabled: bool,

    /// True if external work RAM is enabled.
    external: bool,

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Region {
    BIOS = 0x0,
    Unused0x1 = 0x1,
    ExternalRAM = 0x2,
    InternalRAM = 0x3,
    IORegisters = 0x4,
    Palette = 0x5,
    VRAM = 0x6,
    OAM = 0x7,
    GamePak0Lo = 0x8,
    GamePak0Hi = 0x9,
    GamePak1Lo = 0xA,
    GamePak1Hi = 0xB,
    GamePak2Lo = 0xC,
    GamePak2Hi = 0xD,
    SRAM = 0xE,
    Unused0xF = 0xF,
}

impl Region {
    // /// Returns the total number of memory regions.
    // pub const fn count() -> usize {
    //     16
    // }

    // pub fn index(self) -> usize {
    //     self as u32 as usize
    // }

    pub fn from_address(address: u32) -> Region {
        match (address >> 24) & 0x0F {
            0x00 => Region::BIOS,
            0x01 => Region::Unused0x1,
            0x02 => Region::ExternalRAM,
            0x03 => Region::InternalRAM,
            0x04 => Region::IORegisters,
            0x05 => Region::Palette,
            0x06 => Region::VRAM,
            0x07 => Region::OAM,
            0x08 => Region::GamePak0Lo,
            0x09 => Region::GamePak0Hi,
            0x0A => Region::GamePak1Lo,
            0x0B => Region::GamePak1Hi,
            0x0C => Region::GamePak2Lo,
            0x0D => Region::GamePak2Hi,
            0x0E => Region::SRAM,
            0x0F => Region::Unused0xF,

            // We cover everything in the range (0x0, 0xF)
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}
