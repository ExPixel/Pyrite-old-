use crate::hardware::Region;
use pyrite_common::bits;

macro_rules! set_timings {
    ($Array:expr, $Region:expr, 1, $FirstAccess:expr, $SecondAccess:expr) => {
        $Array[$Region.index()] = AccessCycles {
            nonsequential: 1 + $FirstAccess,
            sequential: 1 + $SecondAccess,
        };
    };

    ($Array:expr, $Region:expr, 2, $FirstAccess:expr, $SecondAccess:expr) => {
        $Array[$Region.index()] = AccessCycles {
            nonsequential: 2 + $FirstAccess + $SecondAccess,
            sequential: 2 + $SecondAccess + $SecondAccess,
        };
    };
}

#[derive(Debug, Clone, Copy)]
pub struct AccessCycles {
    pub nonsequential: u8,
    pub sequential: u8,
}

impl Default for AccessCycles {
    fn default() -> AccessCycles {
        AccessCycles {
            nonsequential: 1,
            sequential: 1,
        }
    }
}

pub struct GbaSystemControl {
    /// nonsequential and sequential (respectively) cycles for 8bit accesses.
    cycles_byte: [AccessCycles; Region::count()],
    /// nonsequential and sequential (respectively) cycles for 16bit accesses.
    cycles_halfword: [AccessCycles; Region::count()],
    /// nonsequential and sequential (respectively) cycles for 32bit accesses.
    cycles_word: [AccessCycles; Region::count()],

    pub stop: bool,
    pub halt: bool,

    // registers:
    pub reg_waitcnt: u16,
    pub reg_postflg: bool,
}

impl GbaSystemControl {
    pub fn new() -> GbaSystemControl {
        GbaSystemControl {
            cycles_byte: [AccessCycles::default(); Region::count()],
            cycles_halfword: [AccessCycles::default(); Region::count()],
            cycles_word: [AccessCycles::default(); Region::count()],

            stop: false,
            halt: false,

            reg_waitcnt: 0,
            reg_postflg: false,
        }
    }

    pub fn update_ram_cycles(&mut self, internal_memory_control: u32) {
        let ram_cycles = 15 - bits!(internal_memory_control, 24, 27) as u8;

        set_timings!(
            self.cycles_byte,
            Region::ExternalRAM,
            1,
            ram_cycles,
            ram_cycles
        );
        set_timings!(
            self.cycles_halfword,
            Region::ExternalRAM,
            1,
            ram_cycles,
            ram_cycles
        );
        // 16bit bus so a 32bit access is 2 16bit accesses
        set_timings!(
            self.cycles_word,
            Region::ExternalRAM,
            2,
            ram_cycles,
            ram_cycles
        );
    }

    pub fn set_reg_waitcnt(&mut self, waitcnt: u16) {
        const CART_FIRST_ACCESS: [u32; 4] = [4, 3, 2, 8];
        const CART0_SECOND_ACCESS: [u32; 2] = [2, 1];
        const CART1_SECOND_ACCESS: [u32; 2] = [4, 1];
        const CART2_SECOND_ACCESS: [u32; 2] = [8, 1];

        self.reg_waitcnt = waitcnt;

        let sram_first_access_byte =
            CART_FIRST_ACCESS[bits!(self.reg_waitcnt, 0, 1) as usize] as u8;
        let waitstate0_first_access_halfword =
            CART_FIRST_ACCESS[bits!(self.reg_waitcnt, 2, 3) as usize] as u8;
        let waitstate0_second_access_halfword =
            CART0_SECOND_ACCESS[bits!(self.reg_waitcnt, 4, 4) as usize] as u8;
        let waitstate1_first_access_halfword =
            CART_FIRST_ACCESS[bits!(self.reg_waitcnt, 5, 6) as usize] as u8;
        let waitstate1_second_access_halfword =
            CART1_SECOND_ACCESS[bits!(self.reg_waitcnt, 7, 7) as usize] as u8;
        let waitstate2_first_access_halfword =
            CART_FIRST_ACCESS[bits!(self.reg_waitcnt, 8, 9) as usize] as u8;
        let waitstate2_second_access_halfword =
            CART2_SECOND_ACCESS[bits!(self.reg_waitcnt, 10, 10) as usize] as u8;

        // WAITSTATE 0
        set_timings!(
            self.cycles_byte,
            Region::GamePak0,
            1,
            waitstate0_first_access_halfword,
            waitstate0_second_access_halfword
        );

        set_timings!(
            self.cycles_halfword,
            Region::GamePak0,
            1,
            waitstate0_first_access_halfword,
            waitstate0_second_access_halfword
        );

        set_timings!(
            self.cycles_word,
            Region::GamePak0,
            2,
            waitstate0_first_access_halfword,
            waitstate0_second_access_halfword
        );

        // WAITSTATE 1
        set_timings!(
            self.cycles_byte,
            Region::GamePak1,
            1,
            waitstate1_first_access_halfword,
            waitstate1_second_access_halfword
        );

        set_timings!(
            self.cycles_halfword,
            Region::GamePak1,
            1,
            waitstate1_first_access_halfword,
            waitstate1_second_access_halfword
        );

        set_timings!(
            self.cycles_word,
            Region::GamePak1,
            2,
            waitstate1_first_access_halfword,
            waitstate1_second_access_halfword
        );

        // WAITSTATE 2
        set_timings!(
            self.cycles_byte,
            Region::GamePak2,
            1,
            waitstate2_first_access_halfword,
            waitstate2_second_access_halfword
        );

        set_timings!(
            self.cycles_halfword,
            Region::GamePak2,
            1,
            waitstate2_first_access_halfword,
            waitstate2_second_access_halfword
        );

        set_timings!(
            self.cycles_word,
            Region::GamePak2,
            2,
            waitstate2_first_access_halfword,
            waitstate2_second_access_halfword
        );

        // SRAM
        set_timings!(
            self.cycles_byte,
            Region::SRAM,
            1,
            sram_first_access_byte,
            sram_first_access_byte
        );
        set_timings!(
            self.cycles_halfword,
            Region::SRAM,
            1,
            sram_first_access_byte,
            sram_first_access_byte
        );
        set_timings!(
            self.cycles_word,
            Region::SRAM,
            1,
            sram_first_access_byte,
            sram_first_access_byte
        );
    }

    pub fn get_word_cycles(&self, addr: u32, seq: bool) -> u32 {
        if seq {
            self.cycles_word[Region::from_address(addr).index()].sequential as u32
        } else {
            self.cycles_word[Region::from_address(addr).index()].nonsequential as u32
        }
    }

    pub fn get_halfword_cycles(&self, addr: u32, seq: bool) -> u32 {
        if seq {
            self.cycles_halfword[Region::from_address(addr).index()].sequential as u32
        } else {
            self.cycles_halfword[Region::from_address(addr).index()].nonsequential as u32
        }
    }

    pub fn get_byte_cycles(&self, addr: u32, seq: bool) -> u32 {
        if seq {
            self.cycles_byte[Region::from_address(addr).index()].sequential as u32
        } else {
            self.cycles_byte[Region::from_address(addr).index()].nonsequential as u32
        }
    }
}
