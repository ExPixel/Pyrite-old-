use pyrite_common::bits;
use crate::hardware::region_of;

pub struct GbaSystemControl {
    /// nonsequential and sequential (respectively) cycles for 8bit accesses.
    cycles_byte:        [(/* nonsequential */ u8, /* sequential */ u8); 16],
    /// nonsequential and sequential (respectively) cycles for 16bit accesses.
    cycles_halfword:    [(/* nonsequential */ u8, /* sequential */ u8); 16],
    /// nonsequential and sequential (respectively) cycles for 32bit accesses.
    cycles_word:        [(/* nonsequential */ u8, /* sequential */ u8); 16],

    pub stop: bool,
    pub halt: bool,

    // registers:
    pub reg_waitcnt: u16,
    pub reg_postflg: bool,
}

impl GbaSystemControl {
    pub fn new() -> GbaSystemControl {
        GbaSystemControl {
            cycles_byte:            [(1, 1); 16],
            cycles_halfword:        [(1, 1); 16],
            cycles_word:            [(1, 1); 16],

            stop: false,
            halt: false,

            reg_waitcnt: 0,
            reg_postflg: false,
        }
    }

    pub fn update_ram_cycles(&mut self, internal_memory_control: u32) {
        let ram_cycles = 15 - bits!(internal_memory_control, 24, 27) as u8;

        self.cycles_byte[REGION_EWRAM as usize] = (1 + ram_cycles, 1 + ram_cycles);
        self.cycles_halfword[REGION_EWRAM as usize] = (1 + ram_cycles, 1 + ram_cycles);
        // 16bit bus so a 32bit access is 2 16bit accesses
        self.cycles_word[REGION_EWRAM as usize] = (2 + ram_cycles + ram_cycles, 2 + ram_cycles + ram_cycles);
    }

    pub fn set_reg_waitcnt(&mut self, waitcnt: u16) {
        const CART_FIRST_ACCESS: [u32; 4] = [4, 3, 2, 8];
        const CART0_SECOND_ACCESS: [u32; 2] = [2, 1];
        const CART1_SECOND_ACCESS: [u32; 2] = [4, 1];
        const CART2_SECOND_ACCESS: [u32; 2] = [8, 1];

        self.reg_waitcnt = waitcnt;

        let sram_first_access_byte = CART_FIRST_ACCESS[bits!(self.reg_waitcnt, 0, 1) as usize] as u8;
        let waitstate0_first_access_halfword = CART_FIRST_ACCESS[bits!(self.reg_waitcnt, 2, 3) as usize] as u8;
        let waitstate0_second_access_halfword = CART0_SECOND_ACCESS[bits!(self.reg_waitcnt, 4, 4) as usize] as u8;
        let waitstate1_first_access_halfword = CART_FIRST_ACCESS[bits!(self.reg_waitcnt, 5, 6) as usize] as u8;
        let waitstate1_second_access_halfword = CART1_SECOND_ACCESS[bits!(self.reg_waitcnt, 7, 7) as usize] as u8;
        let waitstate2_first_access_halfword = CART_FIRST_ACCESS[bits!(self.reg_waitcnt, 8, 9) as usize] as u8;
        let waitstate2_second_access_halfword = CART2_SECOND_ACCESS[bits!(self.reg_waitcnt, 10, 10) as usize] as u8;

        // WAITSTATE 0
        self.cycles_byte[REGION_CART0_L as usize] = (
            1 + waitstate0_first_access_halfword,
            1 + waitstate0_second_access_halfword,
        );
        self.cycles_byte[REGION_CART0_H as usize] = self.cycles_byte[REGION_CART0_L as usize];
        self.cycles_halfword[REGION_CART0_L as usize] = (
            1 + waitstate0_first_access_halfword,
            1 + waitstate0_second_access_halfword,
        );
        self.cycles_halfword[REGION_CART0_H as usize] = self.cycles_halfword[REGION_CART0_L as usize];
        self.cycles_word[REGION_CART0_L as usize] = (
            2 + waitstate0_first_access_halfword + waitstate0_second_access_halfword,
            2 + waitstate0_second_access_halfword + waitstate0_second_access_halfword,
        );
        self.cycles_word[REGION_CART0_H as usize] = self.cycles_word[REGION_CART0_L as usize];

        // WAITSTATE 1
        self.cycles_byte[REGION_CART1_L as usize] = (
            1 + waitstate1_first_access_halfword,
            1 + waitstate1_second_access_halfword,
        );
        self.cycles_byte[REGION_CART1_H as usize] = self.cycles_byte[REGION_CART1_L as usize];
        self.cycles_halfword[REGION_CART1_L as usize] = (
            1 + waitstate1_first_access_halfword,
            1 + waitstate1_second_access_halfword,
        );
        self.cycles_halfword[REGION_CART1_H as usize] = self.cycles_halfword[REGION_CART1_L as usize];
        self.cycles_word[REGION_CART1_L as usize] = (
            2 + waitstate1_first_access_halfword + waitstate1_second_access_halfword,
            2 + waitstate1_second_access_halfword + waitstate1_second_access_halfword,
        );
        self.cycles_word[REGION_CART1_H as usize] = self.cycles_word[REGION_CART1_L as usize];

        // WAITSTATE 2
        self.cycles_byte[REGION_CART2_L as usize] = (
            1 + waitstate2_first_access_halfword,
            1 + waitstate2_second_access_halfword,
        );
        self.cycles_byte[REGION_CART2_H as usize] = self.cycles_byte[REGION_CART2_L as usize];
        self.cycles_halfword[REGION_CART2_L as usize] = (
            1 + waitstate2_first_access_halfword,
            1 + waitstate2_second_access_halfword,
        );
        self.cycles_halfword[REGION_CART2_H as usize] = self.cycles_halfword[REGION_CART2_L as usize];
        self.cycles_word[REGION_CART2_L as usize] = (
            2 + waitstate2_first_access_halfword + waitstate2_second_access_halfword,
            2 + waitstate2_second_access_halfword + waitstate2_second_access_halfword,
        );
        self.cycles_word[REGION_CART2_H as usize] = self.cycles_word[REGION_CART2_L as usize];

        // SRAM
        self.cycles_byte[REGION_SRAM as usize] = (1 + sram_first_access_byte, 1 + sram_first_access_byte);
        self.cycles_halfword[REGION_SRAM as usize] = (1 + sram_first_access_byte, 1 + sram_first_access_byte);
        self.cycles_word[REGION_SRAM as usize] = (1 + sram_first_access_byte, 1 + sram_first_access_byte);

    }

    pub fn get_word_cycles(&self, addr: u32, seq: bool) -> u32 {
        match region_of(addr) {
            region @ 0..=15 => {
                if seq {
                    self.cycles_word[region as usize].1 as u32
                } else {
                    self.cycles_word[region as usize].0 as u32
                }
            },
            _ => return 1,
        }
    }

    pub fn get_halfword_cycles(&self, addr: u32, seq: bool) -> u32 {
        match region_of(addr) {
            region @ 0..=15 => {
                if seq {
                    self.cycles_halfword[region as usize].1 as u32
                } else {
                    self.cycles_halfword[region as usize].0 as u32
                }
            },
            _ => return 1,
        }
    }

    pub fn get_byte_cycles(&self, addr: u32, seq: bool) -> u32 {
        match region_of(addr) {
            region @ 0..=15 => {
                if seq {
                    self.cycles_byte[region as usize].1 as u32
                } else {
                    self.cycles_byte[region as usize].0 as u32
                }
            },
            _ => return 1,
        }
    }
}
// const REGION_BIOS: u32      = 0x00;
// const REGION_UNUSED: u32    = 0x01;
const REGION_EWRAM: u32     = 0x02;
// const REGION_IWRAM: u32     = 0x03;
// const REGION_IOREG: u32     = 0x04;
// const REGION_PAL: u32       = 0x05;
// const REGION_VRAM: u32      = 0x06;
// const REGION_OAM: u32       = 0x07;
const REGION_CART0_L: u32   = 0x08;
const REGION_CART0_H: u32   = 0x09;
const REGION_CART1_L: u32   = 0x0A;
const REGION_CART1_H: u32   = 0x0B;
const REGION_CART2_L: u32   = 0x0C;
const REGION_CART2_H: u32   = 0x0D;
const REGION_SRAM: u32      = 0x0E;
