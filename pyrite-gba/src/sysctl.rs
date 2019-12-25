use crate::hardware::Region;
use pyrite_common::bits;

macro_rules! set_timings {
    ($Width:ident, $Region:expr, 1, $FirstAccess:expr, $SecondAccess:expr) => {
        $Region.$Width = AccessCycles {
            nonsequential: 1 + $FirstAccess,
            sequential: 1 + $SecondAccess,
        };
    };

    ($Width:ident, $Region:expr, 2, $FirstAccess:expr, $SecondAccess:expr) => {
        $Region.$Width = AccessCycles {
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

impl AccessCycles {
    /// Get sequential or non-sequential timing as a u32.
    #[inline]
    pub fn get(&self, seq: bool) -> u32 {
        if seq {
            self.sequential as u32
        } else {
            self.nonsequential as u32
        }
    }
}

impl Default for AccessCycles {
    fn default() -> AccessCycles {
        AccessCycles {
            nonsequential: 1,
            sequential: 1,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RegionCycles {
    pub word: AccessCycles,
    pub halfword: AccessCycles,
    pub byte: AccessCycles,
}

pub struct GbaSystemControl {
    pub ram_cycles: RegionCycles,
    pub gamepak_cycles: [RegionCycles; 3],
    pub sram_cycles: RegionCycles,

    pub stop: bool,
    pub halt: bool,

    // registers:
    pub reg_waitcnt: u16,
    pub reg_postflg: bool,
}

impl GbaSystemControl {
    pub fn new() -> GbaSystemControl {
        GbaSystemControl {
            ram_cycles: RegionCycles::default(),
            gamepak_cycles: [
                RegionCycles::default(),
                RegionCycles::default(),
                RegionCycles::default(),
            ],
            sram_cycles: RegionCycles::default(),

            stop: false,
            halt: false,

            reg_waitcnt: 0,
            reg_postflg: false,
        }
    }

    pub fn update_ram_cycles(&mut self, internal_memory_control: u32) {
        let ram_cycles = 15 - bits!(internal_memory_control, 24, 27) as u8;

        set_timings!(byte, self.ram_cycles, 1, ram_cycles, ram_cycles);
        set_timings!(halfword, self.ram_cycles, 1, ram_cycles, ram_cycles);
        // 16bit bus so a 32bit access is 2 16bit accesses
        set_timings!(word, self.ram_cycles, 2, ram_cycles, ram_cycles);
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
            byte,
            self.gamepak_cycles[0],
            1,
            waitstate0_first_access_halfword,
            waitstate0_second_access_halfword
        );

        set_timings!(
            halfword,
            self.gamepak_cycles[0],
            1,
            waitstate0_first_access_halfword,
            waitstate0_second_access_halfword
        );

        set_timings!(
            word,
            self.gamepak_cycles[0],
            2,
            waitstate0_first_access_halfword,
            waitstate0_second_access_halfword
        );

        // WAITSTATE 1
        set_timings!(
            byte,
            self.gamepak_cycles[1],
            1,
            waitstate1_first_access_halfword,
            waitstate1_second_access_halfword
        );

        set_timings!(
            halfword,
            self.gamepak_cycles[1],
            1,
            waitstate1_first_access_halfword,
            waitstate1_second_access_halfword
        );

        set_timings!(
            word,
            self.gamepak_cycles[1],
            2,
            waitstate1_first_access_halfword,
            waitstate1_second_access_halfword
        );

        // WAITSTATE 2
        set_timings!(
            byte,
            self.gamepak_cycles[2],
            1,
            waitstate2_first_access_halfword,
            waitstate2_second_access_halfword
        );

        set_timings!(
            halfword,
            self.gamepak_cycles[2],
            1,
            waitstate2_first_access_halfword,
            waitstate2_second_access_halfword
        );

        set_timings!(
            word,
            self.gamepak_cycles[2],
            2,
            waitstate2_first_access_halfword,
            waitstate2_second_access_halfword
        );

        // SRAM
        set_timings!(
            byte,
            self.sram_cycles,
            1,
            sram_first_access_byte,
            sram_first_access_byte
        );
        set_timings!(
            halfword,
            self.sram_cycles,
            1,
            sram_first_access_byte,
            sram_first_access_byte
        );
        set_timings!(
            word,
            self.sram_cycles,
            1,
            sram_first_access_byte,
            sram_first_access_byte
        );
    }
}
