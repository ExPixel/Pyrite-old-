
use super::{align16, align32, get_halfword_of_word, set_halfword_of_word};
#[derive(Default)]
pub struct IORegisters {
    // LCD I/O Registers
    pub dispcnt: RegDISPCNT,
    pub green_swap: Reg16,
    pub dispstat: RegDISPSTAT,
    pub vcount: RegVCOUNT,
    pub bg_cnt:  [RegBGxCNT; 4],
    pub bg_hofs: [RegBGxHOFS; 4], 
    pub bg_vofs: [RegBGxVOFS; 4], 
    pub bg2pa: Reg16,
    pub bg2pb: Reg16,
    pub bg2pc: Reg16,
    pub bg2pd: Reg16,
    pub bg2x: Reg32,
    pub bg2y: Reg32,
    pub bg3pa: Reg16,
    pub bg3pb: Reg16,
    pub bg3pc: Reg16,
    pub bg3pd: Reg16,
    pub bg3x: Reg32,
    pub bg3y: Reg32,
    pub win0h: Reg16,
    pub win1h: Reg16,
    pub win0v: Reg16,
    pub win1v: Reg16,
    pub winin: Reg16,
    pub winout: Reg16,
    pub mosaic: Reg16,
    pub bldcnt: Reg16,
    pub bldalpha: Reg16,
    pub bldy: Reg16,

    // Sound Registers
    pub sound1cnt_l: Reg16,
    pub sound1cnt_h: Reg16,
    pub sound1cnt_x: Reg16,

    pub sound2cnt_l: Reg16,
    pub sound2cnt_h: Reg16,

    pub sound3cnt_l: Reg16,
    pub sound3cnt_h: Reg16,
    pub sound3cnt_x: Reg16,

    pub sound4cnt_l: Reg16,
    pub sound4cnt_h: Reg16,

    pub soundcnt_l: Reg16,
    pub soundcnt_h: Reg16,
    pub soundcnt_x: Reg16,

    pub bios_soundbias: Reg16,

    pub wave_ram: [u8; 16],
    pub fifo_a: Reg32,
    pub fifo_b: Reg32,

    // DMA Transfer Channels
    pub dma0sad: Reg32,
    pub dma0dad: Reg32,
    pub dma0cnt_l: Reg16,
    pub dma0cnt_h: Reg16,
    pub dma1sad: Reg32,
    pub dma1dad: Reg32,
    pub dma1cnt_l: Reg16,
    pub dma1cnt_h: Reg16,
    pub dma2sad: Reg32,
    pub dma2dad: Reg32,
    pub dma2cnt_l: Reg16,
    pub dma2cnt_h: Reg16,
    pub dma3sad: Reg32,
    pub dma3dad: Reg32,
    pub dma3cnt_l: Reg16,
    pub dma3cnt_h: Reg16,

    // Timer Registers
    pub tm0cnt_l: Reg16,
    pub tm0cnt_h: Reg16,
    pub tm1cnt_l: Reg16,
    pub tm1cnt_h: Reg16,
    pub tm2cnt_l: Reg16,
    pub tm2cnt_h: Reg16,
    pub tm3cnt_l: Reg16,
    pub tm3cnt_h: Reg16,

    // Serial Communication (1)
    /// This an `siomulti1` make up `SIODATA32`
    pub siomulti0: Reg16,
    /// This an `siomulti0` make up `SIODATA32`
    pub siomulti1: Reg16,
    pub siomulti2: Reg16,
    pub siomulti3: Reg16,
    pub siocnt: Reg16,
    /// This is also `SIODATA8`
    pub siomlt_send: Reg16,

    // Keypad Input
    pub keyinput: Reg16,
    pub keycnt: Reg16,

    // Serial Communication (2)
    pub rcnt: Reg16,
    /// IR Register
    pub infrared: Reg16,
    pub joycnt: Reg16,
    pub joy_recv: Reg32,
    pub joy_trans: Reg32,
    pub joystat: Reg16,

    // Interrupt, Waitstate, Power-Down Control
    /// IE register
    pub interrupt_enable: Reg16,
    /// IF register (Interrupt Request Flags / IRQ Acknowledge)
    pub interrupt_flags: Reg16,
    pub waitcnt: RegWAITCNT,
    pub ime: Reg16,
    pub postflg: Reg8,
    pub haltcnt: Reg8,

    pub internal_memory_control: RegIMC,
}

impl IORegisters {
    pub fn new() -> IORegisters {
        IORegisters::default()
    }

    pub fn read_byte(&self, addr: u32) -> Option<u8> {
        self.read_halfword(addr).map(|hw| {
            if (addr & 1) == 0 {
                hw as u8
            } else {
                (hw >> 8) as u8
            }
        })
    }

    #[inline]
    pub fn read_halfword(&self, unaligned_addr: u32) -> Option<u16> {
        self.internal_read_halfword(unaligned_addr)
    }

    pub fn read_word(&self, unaligned_addr: u32) -> Option<u32> {
        let aligned_addr = align32(unaligned_addr);
        let lo = self.read_halfword(aligned_addr);
        let hi = self.read_halfword(aligned_addr + 2);

        // Works like above Unused Memory when the entire 32bit memory fragment is Unused (eg.
        // 0E0h) and/or Write-Only (eg. DMA0SAD). And otherwise, returns zero if the lower 16bit
        // fragment is readable (eg. 04Ch=MOSAIC, 04Eh=NOTUSED/ZERO).
        match (lo, hi) {
            (Some(l), Some(h)) => {
                Some((l as u32) | ((h as u32) << 16))
            },

            (Some(_), None) => {
                Some(0)
            },

            _ => None,
        }
    }

    pub fn write_byte(&mut self, addr: u32, value: u8) -> Result<(), &'static str> {
        if let Some(hw) = self.read_halfword(addr) {
            let new_value = if (addr & 1) == 0 {
                (hw & 0xFF00) | (value as u16)
            } else {
                (hw & 0x00FF) | ((value as u16) << 8)
            };
            self.write_halfword(addr, new_value)?;
        }
        Ok(())
    }

    #[inline]
    pub fn write_halfword(&mut self, unaligned_addr: u32, value: u16) -> Result<(), &'static str> {
        self.internal_write_halfword(unaligned_addr, value)
    }

    pub fn write_word(&mut self, unaligned_addr: u32, value: u32) -> Result<(), &'static str> {
        let aligned_addr = align32(unaligned_addr);
        self.write_halfword(aligned_addr, value as u16)?;
        self.write_halfword(aligned_addr + 2, (value >> 16) as u16)?;
        Ok(())
    }

    fn internal_read_halfword(&self, unaligned_addr: u32) -> Option<u16> {
        let aligned_addr = align16(unaligned_addr);

        // This is internal memory control or a mirror
        if (aligned_addr & 0x0F00FFFF) == 0x04000800 {
            return Some(get_halfword_of_word(self.internal_memory_control.inner, aligned_addr))
        }

        let ioreg_off = aligned_addr & 0xFFFE;
        match ioreg_off {
            // LCD I/O Registers
            0x0000 => Some(self.dispcnt.inner),
            0x0002 => Some(self.green_swap.inner),
            0x0004 => Some(self.dispstat.inner),
            0x0006 => Some(self.vcount.inner),
            0x0008 => Some(self.bg_cnt[0].inner),
            0x000A => Some(self.bg_cnt[1].inner),
            0x000C => Some(self.bg_cnt[2].inner),
            0x000E => Some(self.bg_cnt[3].inner),
            0x0010 => Some(self.bg_hofs[0].inner),
            0x0012 => Some(self.bg_vofs[0].inner),
            0x0014 => Some(self.bg_hofs[1].inner),
            0x0016 => Some(self.bg_vofs[1].inner),
            0x0018 => Some(self.bg_hofs[2].inner),
            0x001A => Some(self.bg_vofs[2].inner),
            0x001C => Some(self.bg_hofs[3].inner),
            0x001E => Some(self.bg_vofs[3].inner),
            0x0020 => Some(self.bg2pa.inner),
            0x0022 => Some(self.bg2pb.inner),
            0x0024 => Some(self.bg2pc.inner),
            0x0026 => Some(self.bg2pd.inner),
            0x0028 | 0x002A => Some(get_halfword_of_word(self.bg2x.inner, aligned_addr)),
            0x002C | 0x002E => Some(get_halfword_of_word(self.bg2y.inner, aligned_addr)),
            0x0030 => Some(self.bg3pa.inner),
            0x0032 => Some(self.bg3pb.inner),
            0x0034 => Some(self.bg3pc.inner),
            0x0036 => Some(self.bg3pd.inner),
            0x0038 | 0x003A => Some(get_halfword_of_word(self.bg3x.inner, aligned_addr)),
            0x003C | 0x003E => Some(get_halfword_of_word(self.bg3y.inner, aligned_addr)),
            0x0040 => Some(self.win0h.inner),
            0x0042 => Some(self.win1h.inner),
            0x0044 => Some(self.win0v.inner),
            0x0046 => Some(self.win1v.inner),
            0x0048 => Some(self.winin.inner),
            0x004A => Some(self.winout.inner),
            0x004C => Some(self.mosaic.inner),
            0x004E => None, // Not Used
            0x0050 => Some(self.bldcnt.inner),
            0x0052 => Some(self.bldalpha.inner),
            0x0054 => Some(self.bldy.inner),
            0x0056 => None, // Not Used

            // Sound Registers
            0x0060 => Some(self.sound1cnt_l.inner),
            0x0062 => Some(self.sound1cnt_h.inner),
            0x0064 => Some(self.sound1cnt_x.inner),
            0x0066 => None, // Not Used
            0x0068 => Some(self.sound2cnt_l.inner),
            0x006A => None, // Not Used
            0x006C => Some(self.sound2cnt_h.inner),
            0x006E => None, // Not Used
            0x0070 => Some(self.sound3cnt_l.inner),
            0x0072 => Some(self.sound3cnt_h.inner),
            0x0074 => Some(self.sound3cnt_x.inner),
            0x0076 => None, // Not Used
            0x0078 => Some(self.sound4cnt_l.inner),
            0x007A => None, // Not Used
            0x007C => Some(self.sound4cnt_h.inner),
            0x007E => None, // Not Used
            0x0080 => Some(self.soundcnt_l.inner),
            0x0082 => Some(self.soundcnt_h.inner),
            0x0084 => Some(self.soundcnt_x.inner),
            0x0086 => None, // Not Used
            0x0088 => Some(self.bios_soundbias.inner),
            0x008A => None, // Not Used
            0x0090..=0x009E => {
                let wave_ram_off = (ioreg_off - 0x0090) as usize;
                let lo = self.wave_ram[wave_ram_off] as u16;
                let hi = self.wave_ram[wave_ram_off + 1] as u16;
                Some(lo | (hi << 8))
            },
            0x00A0 | 0x00A2 => Some(get_halfword_of_word(self.fifo_a.inner, aligned_addr)),
            0x00A4 | 0x00A6 => Some(get_halfword_of_word(self.fifo_b.inner, aligned_addr)),
            0x00A8 => None, // Not Used

            // DMA Transfer Channels
            0x00B0 | 0x00B2 => Some(get_halfword_of_word(self.dma0sad.inner, aligned_addr)),
            0x00B4 | 0x00B6 => Some(get_halfword_of_word(self.dma0dad.inner, aligned_addr)),
            0x00B8 => Some(self.dma0cnt_l.inner),
            0x00BA => Some(self.dma0cnt_h.inner),
            0x00BC | 0x00BE => Some(get_halfword_of_word(self.dma1sad.inner, aligned_addr)),
            0x00C0 | 0x00C2 => Some(get_halfword_of_word(self.dma1dad.inner, aligned_addr)),
            0x00C4 => Some(self.dma1cnt_l.inner),
            0x00C6 => Some(self.dma1cnt_h.inner),
            0x00C8 | 0x00CA => Some(get_halfword_of_word(self.dma2sad.inner, aligned_addr)),
            0x00CC | 0x00CE => Some(get_halfword_of_word(self.dma2dad.inner, aligned_addr)),
            0x00D0 => Some(self.dma2cnt_l.inner),
            0x00D2 => Some(self.dma2cnt_h.inner),
            0x00D4 | 0x00D6 => Some(get_halfword_of_word(self.dma3sad.inner, aligned_addr)),
            0x00D8 | 0x00DA => Some(get_halfword_of_word(self.dma3dad.inner, aligned_addr)),
            0x00DC => Some(self.dma3cnt_l.inner),
            0x00DE => Some(self.dma3cnt_h.inner),
            0x00E0 => None, // Not Used

            // Timer Registers
            0x0100 => Some(self.tm0cnt_l.inner),
            0x0102 => Some(self.tm0cnt_h.inner),
            0x0104 => Some(self.tm1cnt_l.inner),
            0x0106 => Some(self.tm1cnt_h.inner),
            0x0108 => Some(self.tm2cnt_l.inner),
            0x010A => Some(self.tm2cnt_h.inner),
            0x010C => Some(self.tm3cnt_l.inner),
            0x010E => Some(self.tm3cnt_h.inner),
            0x0110 => None, // Not Used

            // Serial Communication (1)
            0x0120 => Some(self.siomulti0.inner),
            0x0122 => Some(self.siomulti1.inner),
            0x0124 => Some(self.siomulti2.inner),
            0x0126 => Some(self.siomulti3.inner),
            0x0128 => Some(self.siocnt.inner),
            0x012A => Some(self.siomlt_send.inner),
            0x012C => None, // Not Used

            // Keypad Input
            0x0130 => Some(self.keyinput.inner),
            0x0132 => Some(self.keycnt.inner),

            // Serial Communication (2)
            0x0134 => Some(self.rcnt.inner),
            0x0136 => Some(self.infrared.inner),
            0x0138 => None, // Not Used
            0x0140 => Some(self.joycnt.inner),
            0x0142 => None, // Not Used
            0x0150 | 0x0152 => Some(get_halfword_of_word(self.joy_recv.inner, aligned_addr)),
            0x0154 | 0x0156 => Some(get_halfword_of_word(self.joy_trans.inner, aligned_addr)),
            0x0158 => Some(self.joystat.inner),
            0x015A => None, // Not Used

            // Interrupt, Waitstate, and Power-Down Control
            0x0200 => Some(self.interrupt_enable.inner),
            0x0202 => Some(self.interrupt_flags.inner),
            0x0204 => Some(self.waitcnt.inner),
            0x0206 => None, // Not Used
            0x0208 => Some(self.ime.inner),
            0x020A => None, // Not Used
            0x0300 => {
                let lo = self.postflg.inner as u16;
                let hi = self.haltcnt.inner as u16;
                Some(lo | (hi << 8))
            },

            0x0302 => None, // Not Used
            0x0410 => None, // Not Used
            0x0411 => None, // Not Used

            _ => None,
        }
    }

    fn internal_write_halfword(&mut self, unaligned_addr: u32, value: u16) -> Result<(), &'static str> {
        let aligned_addr = align16(unaligned_addr);

        // This is internal memory control or a mirror
        if (aligned_addr & 0x0F00FFFF) == 0x04000800 {
            self.internal_memory_control.inner = set_halfword_of_word(self.internal_memory_control.inner, aligned_addr, value);
            return Ok(())
        }

        let ioreg_off = aligned_addr & 0xFFFE;
        match ioreg_off {
            // LCD I/O Registers
            0x0000 => self.dispcnt.inner = value,
            0x0002 => self.green_swap.inner = value,
            0x0004 => self.dispstat.inner = value,
            0x0006 => self.vcount.inner = value,
            0x0008 => self.bg_cnt[0].inner = value,
            0x000A => self.bg_cnt[1].inner = value,
            0x000C => self.bg_cnt[2].inner = value,
            0x000E => self.bg_cnt[3].inner = value,
            0x0010 => self.bg_hofs[0].inner = value,
            0x0012 => self.bg_vofs[0].inner = value,
            0x0014 => self.bg_hofs[1].inner = value,
            0x0016 => self.bg_vofs[1].inner = value,
            0x0018 => self.bg_hofs[2].inner = value,
            0x001A => self.bg_vofs[2].inner = value,
            0x001C => self.bg_hofs[3].inner = value,
            0x001E => self.bg_vofs[3].inner = value,
            0x0020 => self.bg2pa.inner = value,
            0x0022 => self.bg2pb.inner = value,
            0x0024 => self.bg2pc.inner = value,
            0x0026 => self.bg2pd.inner = value,
            0x0028 | 0x002A => self.bg2x.inner = set_halfword_of_word(self.bg2x.inner, aligned_addr, value),
            0x002C | 0x002E => self.bg2y.inner = set_halfword_of_word(self.bg2y.inner, aligned_addr, value),
            0x0030 => self.bg3pa.inner = value,
            0x0032 => self.bg3pb.inner = value,
            0x0034 => self.bg3pc.inner = value,
            0x0036 => self.bg3pd.inner = value,
            0x0038 | 0x003A => self.bg3x.inner = set_halfword_of_word(self.bg3x.inner, aligned_addr, value),
            0x003C | 0x003E => self.bg3y.inner = set_halfword_of_word(self.bg3y.inner, aligned_addr, value),
            0x0040 => self.win0h.inner = value,
            0x0042 => self.win1h.inner = value,
            0x0044 => self.win0v.inner = value,
            0x0046 => self.win1v.inner = value,
            0x0048 => self.winin.inner = value,
            0x004A => self.winout.inner = value,
            0x004C => self.mosaic.inner = value,
            0x004E => (), // Not Used
            0x0050 => self.bldcnt.inner = value,
            0x0052 => self.bldalpha.inner = value,
            0x0054 => self.bldy.inner = value,
            0x0056 => (), // Not Used

            // Sound Registers
            0x0060 => self.sound1cnt_l.inner = value,
            0x0062 => self.sound1cnt_h.inner = value,
            0x0064 => self.sound1cnt_x.inner = value,
            0x0066 => (), // Not Used
            0x0068 => self.sound2cnt_l.inner = value,
            0x006A => (), // Not Used
            0x006C => self.sound2cnt_h.inner = value,
            0x006E => (), // Not Used
            0x0070 => self.sound3cnt_l.inner = value,
            0x0072 => self.sound3cnt_h.inner = value,
            0x0074 => self.sound3cnt_x.inner = value,
            0x0076 => (), // Not Used
            0x0078 => self.sound4cnt_l.inner = value,
            0x007A => (), // Not Used
            0x007C => self.sound4cnt_h.inner = value,
            0x007E => (), // Not Used
            0x0080 => self.soundcnt_l.inner = value,
            0x0082 => self.soundcnt_h.inner = value,
            0x0084 => self.soundcnt_x.inner = value,
            0x0086 => (), // Not Used
            0x0088 => self.bios_soundbias.inner = value,
            0x008A => (), // Not Used
            0x0090..=0x009E => {
                let wave_ram_off = (ioreg_off - 0x0090) as usize;
                self.wave_ram[wave_ram_off] = value as u8;
                self.wave_ram[wave_ram_off + 1] = (value >> 8) as u8;
            },
            0x00A0 | 0x00A2 => self.fifo_a.inner = set_halfword_of_word(self.fifo_a.inner, aligned_addr, value),
            0x00A4 | 0x00A6 => self.fifo_b.inner = set_halfword_of_word(self.fifo_b.inner, aligned_addr, value),
            0x00A8 => (), // Not Used

            // DMA Transfer Channels
            0x00B0 | 0x00B2 => self.dma0sad.inner = set_halfword_of_word(self.dma0sad.inner, aligned_addr, value),
            0x00B4 | 0x00B6 => self.dma0dad.inner = set_halfword_of_word(self.dma0dad.inner, aligned_addr, value),
            0x00B8 => self.dma0cnt_l.inner = value,
            0x00BA => self.dma0cnt_h.inner = value,
            0x00BC | 0x00BE => self.dma1sad.inner = set_halfword_of_word(self.dma1sad.inner, aligned_addr, value),
            0x00C0 | 0x00C2 => self.dma1dad.inner = set_halfword_of_word(self.dma1dad.inner, aligned_addr, value),
            0x00C4 => self.dma1cnt_l.inner = value,
            0x00C6 => self.dma1cnt_h.inner = value,
            0x00C8 | 0x00CA => self.dma2sad.inner = set_halfword_of_word(self.dma2sad.inner, aligned_addr, value),
            0x00CC | 0x00CE => self.dma2dad.inner = set_halfword_of_word(self.dma2dad.inner, aligned_addr, value),
            0x00D0 => self.dma2cnt_l.inner = value,
            0x00D2 => self.dma2cnt_h.inner = value,
            0x00D4 | 0x00D6 => self.dma3sad.inner = set_halfword_of_word(self.dma3sad.inner, aligned_addr, value),
            0x00D8 | 0x00DA => self.dma3dad.inner = set_halfword_of_word(self.dma3dad.inner, aligned_addr, value),
            0x00DC => self.dma3cnt_l.inner = value,
            0x00DE => self.dma3cnt_h.inner = value,
            0x00E0 => (), // Not Used

            // Timer Registers
            0x0100 => self.tm0cnt_l.inner = value,
            0x0102 => self.tm0cnt_h.inner = value,
            0x0104 => self.tm1cnt_l.inner = value,
            0x0106 => self.tm1cnt_h.inner = value,
            0x0108 => self.tm2cnt_l.inner = value,
            0x010A => self.tm2cnt_h.inner = value,
            0x010C => self.tm3cnt_l.inner = value,
            0x010E => self.tm3cnt_h.inner = value,
            0x0110 => (), // Not Used

            // Serial Communication (1)
            0x0120 => self.siomulti0.inner = value,
            0x0122 => self.siomulti1.inner = value,
            0x0124 => self.siomulti2.inner = value,
            0x0126 => self.siomulti3.inner = value,
            0x0128 => self.siocnt.inner = value,
            0x012A => self.siomlt_send.inner = value,
            0x012C => (), // Not Used

            // Keypad Input
            0x0130 => self.keyinput.inner = value,
            0x0132 => self.keycnt.inner = value,

            // Serial Communication (2)
            0x0134 => self.rcnt.inner = value,
            0x0136 => self.infrared.inner = value,
            0x0138 => (), // Not Used
            0x0140 => self.joycnt.inner = value,
            0x0142 => (), // Not Used
            0x0150 | 0x0152 => self.joy_recv.inner = set_halfword_of_word(self.joy_recv.inner, aligned_addr, value),
            0x0154 | 0x0156 => self.joy_trans.inner = set_halfword_of_word(self.joy_trans.inner, aligned_addr, value),
            0x0158 => self.joystat.inner = value,
            0x015A => (), // Not Used

            // Interrupt, Waitstate, and Power-Down Control
            0x0200 => self.interrupt_enable.inner = value,
            0x0202 => self.interrupt_flags.inner = value,
            0x0204 => self.waitcnt.inner = value,
            0x0206 => (), // Not Used
            0x0208 => self.ime.inner = value,
            0x020A => (), // Not Used
            0x0300 => {
                self.postflg.inner = value as u8;
                self.haltcnt.inner = (value >> 8) as u8;
            },

            0x0302 => (), // Not Used
            0x0410 => (), // Not Used
            0x0411 => (), // Not Used

            _ => {
                if aligned_addr > 0x3FF {
                    return Err("I/O port address out of range");
                }

                // @TODO I should probably make the out of range addresses that the GBA BIOS writes
                // to exceptions to this error since I guess they are technically valid then???
            },
        }
        return Ok(())
    }
}

trait PrimitiveConv<Out> {
    fn conv(self) -> Out;
}

impl<T> PrimitiveConv<T> for T {
    fn conv(self) -> Self {
        self
    }
}

macro_rules! simple_impl_conv {
    ($TypeA:ty, $TypeB:ty) => {
        impl PrimitiveConv<$TypeA> for $TypeB {
            fn conv(self) -> $TypeA {
                self as $TypeA
            }
        }

        impl PrimitiveConv<$TypeB> for $TypeA {
            fn conv(self) -> $TypeB {
                self as $TypeB
            }
        }
    }
}

macro_rules! bool_impl_conv {
    ($Type:ty) => {
        impl PrimitiveConv<bool> for $Type {
            fn conv(self) -> bool {
                if self != 0 { true } else { false }
            }
        }

        impl PrimitiveConv<$Type> for bool {
            fn conv(self) -> $Type {
                if self { 1 } else { 0 }
            }
        }
    }
}

simple_impl_conv!( u8, u16);
simple_impl_conv!( u8, u32);
simple_impl_conv!(u16, u32);
bool_impl_conv!( u8);
bool_impl_conv!(u16);
bool_impl_conv!(u32);


macro_rules! ioreg {
    (
        $TypeName:ident: $InnerType:ty {
            $( $FieldGet:ident, $FieldSet:ident: $FieldType:ty = [$FieldStart:expr, $FieldEnd:expr], )*
        }
    ) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub struct $TypeName {
            pub inner: $InnerType,
        }

        impl $TypeName {
            pub fn new(value: $InnerType) -> $TypeName {
                $TypeName {
                    inner: value
                }
            }

            $(
                #[inline]
                pub fn $FieldGet(&self) -> $FieldType {
                    PrimitiveConv::conv((self.inner >> $FieldStart) & ((1 << ($FieldEnd - $FieldStart + 1)) - 1))
                }

                #[inline]
                pub fn $FieldSet(&mut self, value: $FieldType) {
                    let value: $InnerType = PrimitiveConv::<$InnerType>::conv(value);
                    self.inner = (self.inner & !(((1 << ($FieldEnd - $FieldStart + 1)) - 1) << $FieldStart)) | ((value & ((1 << ($FieldEnd - $FieldStart + 1)) - 1)) << $FieldStart)
                }
            )*
        }

        impl Default for $TypeName {
            fn default() -> $TypeName {
                $TypeName::new(0)
            }
        }
    };
}

// Generic 8bit register. (used as a placeholder)
ioreg! {
    Reg8: u8 {
    }
}

// Generic 16bit register. (used as a placeholder)
ioreg!{
    Reg16: u16 {
    }
}

// Generic 32bit register. (used as a placeholder)
ioreg!{
    Reg32: u32 {
    }
}

// 4000000h - DISPCNT - LCD Control (Read/Write)
//
//   Bit   Expl.
//   0-2   BG Mode                (0-5=Video Mode 0-5, 6-7=Prohibited)
//   3     Reserved / CGB Mode    (0=GBA, 1=CGB; can be set only by BIOS opcodes)
//   4     Display Frame Select   (0-1=Frame 0-1) (for BG Modes 4,5 only)
//   5     H-Blank Interval Free  (1=Allow access to OAM during H-Blank)
//   6     OBJ Character VRAM Mapping (0=Two dimensional, 1=One dimensional)
//   7     Forced Blank           (1=Allow FAST access to VRAM,Palette,OAM)
//   8     Screen Display BG0  (0=Off, 1=On)
//   9     Screen Display BG1  (0=Off, 1=On)
//   10    Screen Display BG2  (0=Off, 1=On)
//   11    Screen Display BG3  (0=Off, 1=On)
//   12    Screen Display OBJ  (0=Off, 1=On)
//   13    Window 0 Display Flag   (0=Off, 1=On)
//   14    Window 1 Display Flag   (0=Off, 1=On)
//   15    OBJ Window Display Flag (0=Off, 1=On)
ioreg! {
    RegDISPCNT: u16 {
        bg_mode, set_bg_mode: u16 = [0, 2],
        cgb_mode, set_cgb_mode: bool = [3, 3],
        frame, set_frame: u16 = [4, 4],
        hblank_interval_free, set_hblank_interval_free: bool = [5, 5],
        obj_one_dimensional, set_obj_one_dimensional: bool = [6, 6],
        forced_blank, set_forced_blank: bool = [7, 7],
        screen_display_bg0, set_screen_display_bg0: bool = [8, 8],
        screen_display_bg1, set_screen_display_bg1: bool = [9, 9],
        screen_display_bg2, set_screen_display_bg2: bool = [10, 10],
        screen_display_bg3, set_screen_display_bg3: bool = [11, 11],
        screen_display_obj, set_screen_display_obj: bool = [12, 12],
        display_window0, set_display_window0: bool = [13, 13],
        display_window1, set_display_window1: bool = [14, 14],
        display_obj_window, set_display_obj_window: bool = [15, 15],
    }
}

// 4000004h - DISPSTAT - General LCD Status (Read/Write)
// Display status and Interrupt control. The H-Blank conditions are generated once per scanline, including for the 'hidden' scanlines during V-Blank.
//
//   Bit   Expl.
//   0     V-Blank flag   (Read only) (1=VBlank) (set in line 160..226; not 227)
//   1     H-Blank flag   (Read only) (1=HBlank) (toggled in all lines, 0..227)
//   2     V-Counter flag (Read only) (1=Match)  (set in selected line)     (R)
//   3     V-Blank IRQ Enable         (1=Enable)                          (R/W)
//   4     H-Blank IRQ Enable         (1=Enable)                          (R/W)
//   5     V-Counter IRQ Enable       (1=Enable)                          (R/W)
//   6     Not used (0) / DSi: LCD Initialization Ready (0=Busy, 1=Ready)   (R)
//   7     Not used (0) / NDS: MSB of V-Vcount Setting (LYC.Bit8) (0..262)(R/W)
//   8-15  V-Count Setting (LYC)      (0..227)                            (R/W)
ioreg! {
    RegDISPSTAT: u16 {
        vblank, set_vblank: bool = [0, 0],
        hvlank, set_hblank: bool = [1, 1],
        vcounter, set_vcounter: bool = [2, 2],
        vblank_irq_enable, set_vblank_irq_enable: bool = [3, 3],
        hblank_irq_enable, set_hblank_irq_enable: bool = [4, 4],
        vcount_irq_enable, set_vcount_irq_enable: bool = [5, 5],
        vcount_setting, set_vcount_setting: u16 = [8, 15],
    }
}

// 4000006h - VCOUNT - Vertical Counter (Read only)
// Indicates the currently drawn scanline, values in range from 160..227 indicate 'hidden' scanlines within VBlank area.
//
//   Bit   Expl.
//   0-7   Current Scanline (LY)      (0..227)                              (R)
//   8     Not used (0) / NDS: MSB of Current Scanline (LY.Bit8) (0..262)   (R)
//   9-15  Not Used (0)
//
// Note: This is much the same than the 'LY' register of older gameboys.
ioreg! {
    RegVCOUNT: u16 {
        current_scanline, set_current_scanline: u16 = [0, 7],
    }
}

// 4000800h - 32bit - Undocumented - Internal Memory Control (R/W)
// Supported by GBA and GBA SP only - NOT supported by DS (even in GBA mode).
// Also supported by GBA Micro - but crashes on "overclocked" WRAM setting.
// Initialized to 0D000020h (by hardware). Unlike all other I/O registers, this register is mirrored across the whole I/O area (in increments of 64K, ie. at 4000800h, 4010800h, 4020800h, ..., 4FF0800h)
//
//   Bit   Expl.
//   0     Disable 32K+256K WRAM (0=Normal, 1=Disable) (when off: empty/prefetch)
//   1-3   Unknown          (Read/Write-able)
//   4     Unknown          (Always zero, not used or write only)
//   5     Enable 256K WRAM (0=Disable, 1=Normal) (when off: mirror of 32K WRAM)
//   6-23  Unknown          (Always zero, not used or write only)
//   24-27 Wait Control WRAM 256K (0-14 = 15..1 Waitstates, 15=Lockup)
//   28-31 Unknown          (Read/Write-able)
//
// The default value 0Dh in Bits 24-27 selects 2 waitstates for 256K WRAM (ie. 3/3/6 cycles 8/16/32bit accesses). The fastest possible setting would be 0Eh (1 waitstate, 2/2/4 cycles for 8/16/32bit), that works on GBA and GBA SP only, the GBA Micro locks up with that setting (it's on-chip RAM is too slow, and works only with 2 or more waitstates).
ioreg! {
    RegIMC: u32 {
        ram_disabled, set_ram_disabled: bool = [0, 0],
        external_ram_enabled, set_external_ram_enabled: bool = [5, 5],
        external_ram_wait, set_external_ram_wait: u32 = [24, 27],
    }
}

// 4000204h - WAITCNT - Waitstate Control (R/W)
// This register is used to configure game pak access timings. The game pak ROM is mirrored to three address regions at 08000000h, 0A000000h, and 0C000000h, these areas are called Wait State 0-2. Different access timings may be assigned to each area (this might be useful in case that a game pak contains several ROM chips with different access times each).
//
//   Bit   Expl.
//   0-1   SRAM Wait Control          (0..3 = 4,3,2,8 cycles)
//   2-3   Wait State 0 First Access  (0..3 = 4,3,2,8 cycles)
//   4     Wait State 0 Second Access (0..1 = 2,1 cycles)
//   5-6   Wait State 1 First Access  (0..3 = 4,3,2,8 cycles)
//   7     Wait State 1 Second Access (0..1 = 4,1 cycles; unlike above WS0)
//   8-9   Wait State 2 First Access  (0..3 = 4,3,2,8 cycles)
//   10    Wait State 2 Second Access (0..1 = 8,1 cycles; unlike above WS0,WS1)
//   11-12 PHI Terminal Output        (0..3 = Disable, 4.19MHz, 8.38MHz, 16.78MHz)
//   13    Not used
//   14    Game Pak Prefetch Buffer (Pipe) (0=Disable, 1=Enable)
//   15    Game Pak Type Flag  (Read Only) (0=GBA, 1=CGB) (IN35 signal)
//   16-31 Not used
ioreg! {
    RegWAITCNT: u16 {
        sram_wait_control, set_sram_wait_control: u16 = [0, 1],
        waitstate0_first_access, set_waitstate0_first_access: u16 = [2, 3],
        waitstate0_second_access, set_waitstate0_second_access: u16 = [4, 4],
        waitstate1_first_access, set_waitstate1_first_access: u16 = [5, 6],
        waitstate1_second_access, set_waitstate1_second_access: u16 = [7, 7],
        waitstate2_first_access, set_waitstate2_first_access: u16 = [8, 9],
        waitstate2_second_access, set_waitstate2_second_access: u16 = [10, 10],
        phi_terminal_output, set_phi_terminal_output: u16 = [11, 12],
        gamepak_prefetch, set_gamepak_prefetch: bool = [14, 14],
        gamepak_type_flag, set_gamepak_type_flag: bool = [15, 15],
    }
}

/// 4000008h - BG0CNT - BG0 Control (R/W) (BG Modes 0,1 only)
/// 400000Ah - BG1CNT - BG1 Control (R/W) (BG Modes 0,1 only)
/// 400000Ch - BG2CNT - BG2 Control (R/W) (BG Modes 0,1,2 only)
/// 400000Eh - BG3CNT - BG3 Control (R/W) (BG Modes 0,2 only)
///
///   Bit   Expl.
///   0-1   BG Priority           (0-3, 0=Highest)
///   2-3   Character Base Block  (0-3, in units of 16 KBytes) (=BG Tile Data)
///   4-5   Not used (must be zero) (except in NDS mode: MSBs of char base)
///   6     Mosaic                (0=Disable, 1=Enable)
///   7     Colors/Palettes       (0=16/16, 1=256/1)
///   8-12  Screen Base Block     (0-31, in units of 2 KBytes) (=BG Map Data)
///   13    BG0/BG1: Not used (except in NDS mode: Ext Palette Slot for BG0/BG1)
///   13    BG2/BG3: Display Area Overflow (0=Transparent, 1=Wraparound)
///   14-15 Screen Size (0-3)
///
/// Internal Screen Size (dots) and size of BG Map (bytes):
///
///   Value  Text Mode      Rotation/Scaling Mode
///   0      256x256 (2K)   128x128   (256 bytes)
///   1      512x256 (4K)   256x256   (1K)
///   2      256x512 (4K)   512x512   (4K)
///   3      512x512 (8K)   1024x1024 (16K)
ioreg! {
    RegBGCNT: u16 {
        priority, set_priority: u16 = [0, 1],
        char_base_block, set_char_base_block: u16 = [2, 3],
        mosaic, set_mosaic: bool = [6, 6],
        pal256, set_pal256: bool = [7, 7],
        screen_base_block, set_screen_base_block: u16 = [8, 12],
        display_area_overflow, set_display_area_overflow: bool = [13, 13],
        screen_size, set_screen_size: u16 = [14, 15],
    }
}

ioreg! {
    RegBGxCNT: u16 {
        priority, set_priority: u16 = [0, 1],
        char_base_block, set_char_base_block: u16 = [2, 3],
        mosaic, set_mosaic: bool = [6, 6],
        pal256, set_pal256: bool = [7, 7],
        screen_base_block, set_screen_base_block: u16 = [8, 12],
        display_area_overflow, set_display_area_overflow: bool = [13, 13],
        screen_size, set_screen_size: u16 = [14, 15],
    }
}

ioreg! {
    RegBGxHOFS: u16 {
        offset, set_offset: u16 = [0, 8],
    }
}

ioreg! {
    RegBGxVOFS: u16 {
        offset, set_offset: u16 = [0, 8],
    }
}
