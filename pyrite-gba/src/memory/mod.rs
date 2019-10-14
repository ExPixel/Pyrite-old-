// I use a macro instead of a const fn because I need the types
// to be generic.
/// Converts kilobytes to bytes.
macro_rules! kb {
    ($Kilobytes:expr) => {
        $Kilobytes * 1024
    }
}

// @TODO implement this thing
macro_rules! gba_error {
    ($($arg:tt)*) => {
        println!($($arg)*)
    }
}

pub mod gamepak;
pub mod ioreg;
pub mod palette;

use pyrite_arm::ArmMemory;
use gamepak::GamePakROM;
use ioreg::IORegisters;
use palette::Palette;

/// Abstraction over GBA memory that provides timing and handling of reading unused memory.
pub struct GbaMemory {
    /// nonsequential and sequential (respectively) cycles for 8bit accesses.
    cycles_byte: [(/* nonsequential */ u8, /* sequential */ u8); 16],
    /// nonsequential and sequential (respectively) cycles for 16bit accesses.
    cycles_halfword: [(/* nonsequential */ u8, /* sequential */ u8); 16],
    /// nonsequential and sequential (respectively) cycles for 32bit accesses.
    cycles_word: [(/* nonsequential */ u8, /* sequential */ u8); 16],

    /// If this is true, reads from 00000000-00003FFF (BIOS ROM) will be allowed.
    /// If this is false, reads from BIOS ROM will return the most recently prefetched
    /// BIOS  opcode.
    pub bios_readable: bool,

    /// The most recently prefetched BIOS opcode.
    recent_bios_prefetch: u32,

    /// The most recently prefetched opcode.
    recent_prefetch: u32,

    pub ioregs: IORegisters,
    pub palette: Palette,
    pub gamepak: GamePakROM,

    // regions:
    pub mem_bios:   Vec<u8>,
    pub mem_ewram:  Vec<u8>,
    pub mem_iwram:  Vec<u8>,
    pub mem_vram:   Vec<u8>,
    pub mem_oam:    Vec<u8>,
}

impl GbaMemory {
    /// If initialize is true, the memory will be initialized in the same way
    /// the GBA's hardware would.
    pub fn new(initialize: bool) -> GbaMemory {
        let mut memory = GbaMemory {
            cycles_byte: [(0, 0); 16],
            cycles_halfword: [(0, 0); 16],
            cycles_word: [(0, 0); 16],

            bios_readable: true,
            recent_bios_prefetch: 0,
            recent_prefetch: 0,

            ioregs: IORegisters::new(),
            palette: Palette::new(),
            gamepak: GamePakROM::new(Vec::new()),

            mem_bios:   vec![0; REGION_BIOS_LEN],
            mem_ewram:  vec![0; REGION_EWRAM_LEN],
            mem_iwram:  vec![0; REGION_IWRAM_LEN],
            mem_vram:   vec![0; REGION_VRAM_LEN],
            mem_oam:    vec![0; REGION_OAM_LEN],
        };

        if initialize {
            memory.init();
        }

        return memory;
    }

    /// Initialize memory the same way GBA hardware would.
    pub fn init(&mut self) {
        const INTERNAL_MEMORY_CONTROL_DEFAULT: u32 = 0x0D000020;
        self.ioregs.internal_memory_control.inner = INTERNAL_MEMORY_CONTROL_DEFAULT;
        self.init_waitstates();
    }

    /// Set the binary for the GamePak ROM region.
    pub fn set_gamepak_rom(&mut self, data: Vec<u8>) {
        self.gamepak = GamePakROM::new(data);
    }

    // @TODO remove these
    pub fn gamepak_rom(&self) -> &GamePakROM {
        &self.gamepak
    }

    pub fn gamepak_rom_mut(&mut self) -> &mut GamePakROM {
        &mut self.gamepak
    }

    /// Set the binary for the BIOS memory region.
    /// This must be 16KB (1024 * 16 bytes) in length or this function
    /// will panic.
    pub fn set_bios(&mut self, data: Vec<u8>) {
        assert!(data.len() == REGION_BIOS_LEN, "BIOS binary must be 16KB in length");
        self.mem_bios = data;
    }

    pub fn code_access_byte_seq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_byte, region_of(addr), true)
    }

    pub fn code_access_halfword_seq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_halfword, region_of(addr), true)
    }

    pub fn code_access_word_seq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_word, region_of(addr), true)
    }

    pub fn code_access_byte_nonseq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_byte, region_of(addr), false)
    }

    pub fn code_access_halfword_nonseq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_halfword, region_of(addr), false)
    }

    pub fn code_access_word_nonseq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_word, region_of(addr), false)
    }

    pub fn data_access_byte_seq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_byte, region_of(addr), true)
    }

    pub fn data_access_halfword_seq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_halfword, region_of(addr), true)
    }

    pub fn data_access_word_seq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_word, region_of(addr), true)
    }

    pub fn data_access_byte_nonseq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_byte, region_of(addr), false)
    }

    pub fn data_access_halfword_nonseq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_halfword, region_of(addr), false)
    }

    pub fn data_access_word_nonseq(&self, addr: u32) -> u32 {
        Self::cycles_for_access(&self.cycles_word, region_of(addr), false)
    }

    fn init_waitstates(&mut self) {
        // Unused regions:
        self.cycles_byte[0x01] = (1, 1);
        self.cycles_byte[0x0F] = (1, 1);

        self.cycles_halfword[0x01] = (1, 1);
        self.cycles_halfword[0x0F] = (1, 1);

        self.cycles_word[0x01] = (1, 1);
        self.cycles_word[0x0F] = (1, 1);

        // BIOS, IWRAM, I/O, OAM
        // These regions have a waitstate of 0 and 32bit address buses so all reads and writes
        // only take one cycle (1 + WAIT per access)
        self.cycles_byte[REGION_BIOS as usize] = (1, 1);
        self.cycles_halfword[REGION_BIOS as usize] = (1, 1);
        self.cycles_word[REGION_BIOS as usize] = (1, 1);

        self.cycles_byte[REGION_IWRAM as usize] = (1, 1);
        self.cycles_halfword[REGION_IWRAM as usize] = (1, 1);
        self.cycles_word[REGION_IWRAM as usize] = (1, 1);

        self.cycles_byte[REGION_IOREG as usize] = (1, 1);
        self.cycles_halfword[REGION_IOREG as usize] = (1, 1);
        self.cycles_word[REGION_IOREG as usize] = (1, 1);

        self.cycles_byte[REGION_OAM as usize] = (1, 1);
        self.cycles_halfword[REGION_OAM as usize] = (1, 1);
        self.cycles_word[REGION_OAM as usize] = (1, 1);

        // Palette RAM / VRAM
        // These regions have a waitstate of 0 but 16bit access so 32bit access require 2 cycles (2 16bit accesses)
        // but all other accesses take one cycle (1 + WAIT per access)
        self.cycles_byte[REGION_PAL as usize] = (1, 1);
        self.cycles_halfword[REGION_PAL as usize] = (1, 1);
        self.cycles_word[REGION_PAL as usize] = (2, 2);

        self.cycles_byte[REGION_VRAM as usize] = (1, 1);
        self.cycles_halfword[REGION_VRAM as usize] = (1, 1);
        self.cycles_word[REGION_VRAM as usize] = (2, 2);

        self.update_ram_waitstates();
        self.update_gamepak_sram_waitstates();

        // Region        Bus   Read      Write     Cycles
        // BIOS ROM      32    8/16/32   -         1/1/1
        // Work RAM 32K  32    8/16/32   8/16/32   1/1/1
        // I/O           32    8/16/32   8/16/32   1/1/1
        // OAM           32    8/16/32   16/32     1/1/1 *
        // Work RAM 256K 16    8/16/32   8/16/32   3/3/6 **
        // Palette RAM   16    8/16/32   16/32     1/1/2 *
        // VRAM          16    8/16/32   16/32     1/1/2 *
        // GamePak ROM   16    8/16/32   -         5/5/8 **/***
        // GamePak Flash 16    8/16/32   16/32     5/5/8 **/***
        // GamePak SRAM  8     8         8         5     **
    }

    fn update_ram_waitstates(&mut self) {
        let ram_cycles = 15 - self.ioregs.internal_memory_control.external_ram_wait() as u8;

        self.cycles_byte[REGION_EWRAM as usize] = (1 + ram_cycles, 1 + ram_cycles);
        self.cycles_halfword[REGION_EWRAM as usize] = (1 + ram_cycles, 1 + ram_cycles);
        // 16bit bus so a 32bit access is 2 16bit accesses
        self.cycles_word[REGION_EWRAM as usize] = (2 + ram_cycles + ram_cycles, 2 + ram_cycles + ram_cycles);
    }

    fn update_gamepak_sram_waitstates(&mut self) {
        const CART_FIRST_ACCESS: [u32; 4] = [4, 3, 2, 8];
        const CART0_SECOND_ACCESS: [u32; 2] = [2, 1];
        const CART1_SECOND_ACCESS: [u32; 2] = [4, 1];
        const CART2_SECOND_ACCESS: [u32; 2] = [8, 1];

        let sram_first_access_byte = CART_FIRST_ACCESS[self.ioregs.waitcnt.sram_wait_control() as usize] as u8;
        let waitstate0_first_access_halfword = CART_FIRST_ACCESS[self.ioregs.waitcnt.waitstate0_first_access() as usize] as u8;
        let waitstate0_second_access_halfword = CART0_SECOND_ACCESS[self.ioregs.waitcnt.waitstate0_second_access() as usize] as u8;
        let waitstate1_first_access_halfword = CART_FIRST_ACCESS[self.ioregs.waitcnt.waitstate1_first_access() as usize] as u8;
        let waitstate1_second_access_halfword = CART1_SECOND_ACCESS[self.ioregs.waitcnt.waitstate1_second_access() as usize] as u8;
        let waitstate2_first_access_halfword = CART_FIRST_ACCESS[self.ioregs.waitcnt.waitstate2_first_access() as usize] as u8;
        let waitstate2_second_access_halfword = CART2_SECOND_ACCESS[self.ioregs.waitcnt.waitstate2_second_access() as usize] as u8;

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

    #[inline(always)]
    fn cycles_for_access(table: &[(u8, u8); 16], region: u32, sequential: bool) -> u32 {
        match region {
            0..=15 => {
                if sequential {
                    table[region as usize].1 as u32
                } else {
                    table[region as usize].0 as u32
                }
            },
            _ => return 1,
        }
    }

    /// Read an 8bit byte from an address.
    pub fn read_byte(&self, addr: u32) -> u8 {
        match region_of(addr) {
            REGION_BIOS if addr < 0x00004000 => {
                if self.bios_readable {
                    self.mem_bios[addr as usize]
                } else {
                    // The BIOS memory is protected against reading, the GBA allows to read opcodes
                    // or data only if the program counter is located inside of the BIOS area. If
                    // the program counter is not in the BIOS area, reading will return the most
                    // recent successfully fetched BIOS opcode.
                    get_byte_of_word(self.recent_bios_prefetch, addr)
                }
            },

            REGION_UNUSED | REGION_BIOS => {
                // Accessing unused memory at 00004000h-01FFFFFFh, and 10000000h-FFFFFFFFh (and
                // 02000000h-03FFFFFFh when RAM is disabled via Port 4000800h) returns the recently
                // pre-fetched opcode.
                get_byte_of_word(self.recent_prefetch, addr)
            },

            REGION_EWRAM => {
                if self.is_ram_disabled() {
                    get_byte_of_word(self.recent_prefetch, addr)
                } else if self.is_external_ram_disabled() {
                    self.mem_iwram[(addr as usize) % REGION_IWRAM_LEN]
                } else {
                    self.mem_ewram[(addr as usize) % REGION_EWRAM_LEN]
                }
            },

            REGION_IWRAM => {
                if self.is_ram_disabled() {
                    get_byte_of_word(self.recent_prefetch, addr)
                } else {
                    self.mem_iwram[(addr as usize) % REGION_IWRAM_LEN]
                }
            },

            REGION_IOREG => {
                if let Some(byte) = self.ioregs.read_byte(addr) {
                    byte
                } else {
                    get_byte_of_word(self.recent_prefetch, addr)
                }
            },

            REGION_PAL => {
                self.palette.load8(addr % REGION_PAL_LEN32)
            },

            REGION_VRAM => {
                self.mem_vram[to_vram_physical_addr(addr)]
            },

            REGION_OAM => {
                self.mem_oam[(addr as usize) % REGION_OAM_LEN]
            },

            REGION_CART0_L | REGION_CART0_H |
            REGION_CART1_L | REGION_CART1_H |
            REGION_CART2_L | REGION_CART2_H   => {
                self.gamepak.read_byte(addr)
            },

            REGION_SRAM => {
                unimplemented!("GbaMemory::read_byte(REGION_SRAM)");
            },

            _ => {
                self.bad_read(addr, 8, "bad region");
                get_byte_of_word(self.recent_prefetch, addr)
            },
        }
    }

    /// Read a 16bit halfword at an address.
    /// This function will always halfword align its argument (`addr & 0xFFFFFFFE`)
    pub fn read_halfword(&self, unaligned_addr: u32) -> u16 {
        let aligned_addr = align16(unaligned_addr);
        match region_of(aligned_addr) {
            REGION_BIOS if aligned_addr < 0x00004000 => {
                if self.bios_readable {
                    read16_le(&self.mem_bios, aligned_addr as usize)
                } else {
                    // The BIOS memory is protected against reading, the GBA allows to read opcodes
                    // or data only if the program counter is located inside of the BIOS area. If
                    // the program counter is not in the BIOS area, reading will return the most
                    // recent successfully fetched BIOS opcode.
                    get_halfword_of_word(self.recent_bios_prefetch, aligned_addr)
                }
            },

            REGION_UNUSED | REGION_BIOS => {
                // Accessing unused memory at 00004000h-01FFFFFFh, and 10000000h-FFFFFFFFh (and
                // 02000000h-03FFFFFFh when RAM is disabled via Port 4000800h) returns the recently
                // pre-fetched opcode.
                get_halfword_of_word(self.recent_prefetch, aligned_addr)
            },

            REGION_EWRAM => {
                if self.is_ram_disabled() {
                    get_halfword_of_word(self.recent_prefetch, aligned_addr)
                } else if self.is_external_ram_disabled() {
                    read16_le(&self.mem_iwram, (aligned_addr as usize) % REGION_IWRAM_LEN)
                } else {
                    read16_le(&self.mem_ewram, (aligned_addr as usize) % REGION_EWRAM_LEN)
                }
            },

            REGION_IWRAM => {
                if self.is_ram_disabled() {
                    get_halfword_of_word(self.recent_prefetch, aligned_addr)
                } else {
                    read16_le(&self.mem_iwram, (aligned_addr as usize) % REGION_IWRAM_LEN)
                }
            },

            REGION_IOREG => {
                if let Some(byte) = self.ioregs.read_halfword(aligned_addr) {
                    byte
                } else {
                    get_halfword_of_word(self.recent_prefetch, aligned_addr)
                }
            },

            REGION_PAL => {
                self.palette.load16(aligned_addr % REGION_PAL_LEN32)
            },

            REGION_VRAM => {
                read16_le(&self.mem_vram, to_vram_physical_addr(aligned_addr))
            },

            REGION_OAM => {
                read16_le(&self.mem_oam, (aligned_addr as usize) % REGION_OAM_LEN)
            },

            REGION_CART0_L | REGION_CART0_H |
            REGION_CART1_L | REGION_CART1_H |
            REGION_CART2_L | REGION_CART2_H   => {
                self.gamepak.read_halfword(aligned_addr)
            },

            REGION_SRAM => {
                unimplemented!("GbaMemory::read_halfword(REGION_SRAM)");
            },

            _ => {
                self.bad_read(aligned_addr, 16, "bad region");
                get_halfword_of_word(self.recent_prefetch, aligned_addr)
            },
        }
    }

    /// Read a 32bit word at an address.
    /// This function will always word align its argument (`addr & 0xFFFFFFFC`)
    pub fn read_word(&self, unaligned_addr: u32) -> u32 {
        let aligned_addr = align32(unaligned_addr);
        match region_of(aligned_addr) {
            REGION_BIOS if aligned_addr < 0x00004000 => {
                if self.bios_readable {
                    read32_le(&self.mem_bios, aligned_addr as usize)
                } else {
                    // The BIOS memory is protected against reading, the GBA allows to read opcodes
                    // or data only if the program counter is located inside of the BIOS area. If
                    // the program counter is not in the BIOS area, reading will return the most
                    // recent successfully fetched BIOS opcode.
                    self.recent_bios_prefetch
                }
            },

            REGION_UNUSED | REGION_BIOS => {
                // Accessing unused memory at 00004000h-01FFFFFFh, and 10000000h-FFFFFFFFh (and
                // 02000000h-03FFFFFFh when RAM is disabled via Port 4000800h) returns the recently
                // pre-fetched opcode.
                self.recent_prefetch
            },

            REGION_EWRAM => {
                if self.is_ram_disabled() {
                    self.recent_prefetch
                } else if self.is_ram_disabled() {
                    read32_le(&self.mem_iwram, (aligned_addr as usize) % REGION_IWRAM_LEN)
                } else {
                    read32_le(&self.mem_ewram, (aligned_addr as usize) % REGION_EWRAM_LEN)
                }
            },

            REGION_IWRAM => {
                if self.is_ram_disabled() {
                    self.recent_prefetch
                } else {
                    read32_le(&self.mem_iwram, (aligned_addr as usize) % REGION_IWRAM_LEN)
                }
            },

            REGION_IOREG => {
                if let Some(byte) = self.ioregs.read_word(aligned_addr) {
                    byte
                } else {
                    self.recent_prefetch
                }
            },

            REGION_PAL => {
                self.palette.load32(aligned_addr % REGION_PAL_LEN32)
            },

            REGION_VRAM => {
                read32_le(&self.mem_vram, to_vram_physical_addr(aligned_addr))
            },

            REGION_OAM => {
                read32_le(&self.mem_oam, (aligned_addr as usize) % REGION_OAM_LEN)
            },

            REGION_CART0_L | REGION_CART0_H |
            REGION_CART1_L | REGION_CART1_H |
            REGION_CART2_L | REGION_CART2_H   => {
                self.gamepak.read_word(aligned_addr)
            },

            REGION_SRAM => {
                unimplemented!("GbaMemory::read_word(REGION_SRAM)");
            },

            _ => {
                self.bad_read(aligned_addr, 32, "bad region");
                self.recent_prefetch
            },
        }
    }

    /// Write an 8bit value to an address.
    pub fn write_byte(&mut self, addr: u32, value: u8) {
        match region_of(addr) {
            REGION_BIOS if addr < 0x00004000 => {
                self.bad_write(addr, value as u32, 8, "attempt to write byte to readonly BIOS memory");
            },

            REGION_UNUSED | REGION_BIOS => {
                self.bad_write(addr, value as u32, 8, "attempt to write byte to unused memory region");
            },

            REGION_EWRAM => {
                if self.is_ram_disabled() {
                    self.bad_write(addr, value as u32, 8, "attempt to write to External RAM while RAM is disabled");
                } else if self.is_external_ram_disabled() {
                    self.mem_iwram[(addr as usize) % REGION_IWRAM_LEN] = value;
                } else {
                    self.mem_ewram[(addr as usize) % REGION_EWRAM_LEN] = value;
                }
            },

            REGION_IWRAM => {
                if self.is_ram_disabled() {
                    self.bad_write(addr, value as u32, 8, "attempt to write to Internal RAM while RAM is disabled");
                } else {
                    self.mem_iwram[(addr as usize) % REGION_IWRAM_LEN] = value;
                }
            },

            REGION_IOREG => {
                if let Err(msg) = self.ioregs.write_byte(addr, value) {
                    self.bad_write(addr, value as u32, 8, msg);
                } else {
                    if align32(addr) == 0x4000204 {
                        // if this is the WAITCNT register
                        self.update_gamepak_sram_waitstates();
                    } else if (align32(addr) & 0x0F00FFFF) == 0x04000800 {
                        // if this is the Internal Memory Control
                        self.update_ram_waitstates();
                    }
                }
            },

            REGION_PAL => {
                self.palette.store8(addr % REGION_PAL_LEN32, value);
            },

            REGION_VRAM => {
                self.mem_vram[to_vram_physical_addr(addr)] = value;
            },

            REGION_OAM => {
                self.mem_oam[(addr as usize) % REGION_OAM_LEN] = value;
            },

            REGION_CART0_L | REGION_CART0_H |
            REGION_CART1_L | REGION_CART1_H |
            REGION_CART2_L | REGION_CART2_H   => {
                if let Err(message) = self.gamepak.write_byte(addr, value) {
                    self.bad_write(addr, value as u32, 8, message);
                }
            },

            REGION_SRAM => {
                unimplemented!("GbaMemory::write_byte(REGION_SRAM)");
            },

            _ => {
                self.bad_write(addr, value as u32, 8, "bad region");
            }
        }
    }

    /// Write a 16bit halfword to an address.
    /// This function will always halfword align the address (`addr & 0xFFFFFFFE`)
    pub fn write_halfword(&mut self, unaligned_addr: u32, value: u16) {
        let aligned_addr = align16(unaligned_addr);
        match region_of(aligned_addr) {
            REGION_BIOS if aligned_addr < 0x00004000 => {
                self.bad_write(aligned_addr, value as u32, 16, "attempt to write byte to readonly BIOS memory");
            },

            REGION_UNUSED | REGION_BIOS => {
                self.bad_write(aligned_addr, value as u32, 16, "attempt to write byte to unused memory region");
            },

            REGION_EWRAM => {
                if self.is_ram_disabled() {
                    self.bad_write(aligned_addr, value as u32, 16, "attempt to write to External RAM while RAM is disabled");
                } else if self.is_external_ram_disabled() {
                    write16_le(&mut self.mem_iwram, (aligned_addr as usize) % REGION_IWRAM_LEN, value);
                } else {
                    write16_le(&mut self.mem_ewram, (aligned_addr as usize) % REGION_EWRAM_LEN, value);
                }
            },

            REGION_IWRAM => {
                if self.is_ram_disabled() {
                    self.bad_write(aligned_addr, value as u32, 16, "attempt to write to Internal RAM while RAM is disabled");
                } else {
                    write16_le(&mut self.mem_iwram, (aligned_addr as usize) % REGION_IWRAM_LEN, value);
                }
            },

            REGION_IOREG => {
                if let Err(msg) = self.ioregs.write_halfword(aligned_addr, value) {
                    self.bad_write(aligned_addr, value as u32, 16, msg);
                } else {
                    if align32(aligned_addr) == 0x4000204 {
                        // if this is the WAITCNT register
                        self.update_gamepak_sram_waitstates();
                    } else if (align32(aligned_addr) & 0x0F00FFFF) == 0x04000800 {
                        // if this is the Internal Memory Control
                        self.update_ram_waitstates();
                    }
                }
            },

            REGION_PAL => {
                self.palette.store16(aligned_addr % REGION_PAL_LEN32, value);
            },

            REGION_VRAM => {
                write16_le(&mut self.mem_vram, to_vram_physical_addr(aligned_addr), value);
            },

            REGION_OAM => {
                write16_le(&mut self.mem_oam, (aligned_addr as usize) % REGION_OAM_LEN, value);
            },

            REGION_CART0_L | REGION_CART0_H |
            REGION_CART1_L | REGION_CART1_H |
            REGION_CART2_L | REGION_CART2_H   => {
                if let Err(message) = self.gamepak.write_halfword(aligned_addr, value) {
                    self.bad_write(aligned_addr, value as u32, 16, message);
                }
            },

            REGION_SRAM => {
                unimplemented!("GbaMemory::write_halfword(REGION_SRAM)");
            },

            _ => {
                self.bad_write(aligned_addr, value as u32, 16, "bad region");
            }
        }
    }

    /// Write a 32bit word to an address.
    /// This function will always word align the address (`addr & 0xFFFFFFFC`)
    pub fn write_word(&mut self, unaligned_addr: u32, value: u32) {
        let aligned_addr = align32(unaligned_addr);
        match region_of(aligned_addr) {
            REGION_BIOS if aligned_addr < 0x00004000 => {
                self.bad_write(aligned_addr, value, 32, "attempt to write byte to readonly BIOS memory");
            },

            REGION_UNUSED | REGION_BIOS => {
                self.bad_write(aligned_addr, value, 32, "attempt to write byte to unused memory region");
            },

            REGION_EWRAM => {
                if self.is_ram_disabled() {
                    self.bad_write(aligned_addr, value, 32, "attempt to write to External RAM while RAM is disabled");
                } else if self.is_external_ram_disabled() {
                    write32_le(&mut self.mem_iwram, (aligned_addr as usize) % REGION_IWRAM_LEN, value);
                } else {
                    write32_le(&mut self.mem_ewram, (aligned_addr as usize) % REGION_EWRAM_LEN, value);
                }
            },

            REGION_IWRAM => {
                if self.is_ram_disabled() {
                    self.bad_write(aligned_addr, value, 32, "attempt to write to Internal RAM while RAM is disabled");
                } else {
                    write32_le(&mut self.mem_iwram, (aligned_addr as usize) % REGION_IWRAM_LEN, value);
                }
            },

            REGION_IOREG => {
                if let Err(msg) = self.ioregs.write_word(aligned_addr, value) {
                    self.bad_write(aligned_addr, value, 32, msg);
                } else {
                    if aligned_addr == 0x4000204 {
                        // if this is the WAITCNT register
                        self.update_gamepak_sram_waitstates();
                    } else if (aligned_addr & 0x0F00FFFF) == 0x04000800 {
                        // if this is the Internal Memory Control
                        self.update_ram_waitstates();
                    }
                }
            },

            REGION_PAL => {
                self.palette.store32(aligned_addr % REGION_PAL_LEN32, value);
            },

            REGION_VRAM => {
                write32_le(&mut self.mem_vram, to_vram_physical_addr(aligned_addr), value);
            },

            REGION_OAM => {
                write32_le(&mut self.mem_oam, (aligned_addr as usize) % REGION_OAM_LEN, value);
            },

            REGION_CART0_L | REGION_CART0_H |
            REGION_CART1_L | REGION_CART1_H |
            REGION_CART2_L | REGION_CART2_H   => {
                if let Err(message) = self.gamepak.write_word(aligned_addr, value) {
                    self.bad_write(aligned_addr, value, 32, message);
                }
            },

            REGION_SRAM => {
                unimplemented!("GbaMemory::write_word(REGION_SRAM)");
            },

            _ => {
                self.bad_write(aligned_addr, value, 32, "bad region");
            }
        }
    }

    /// Called after a bad read has occurred.
    #[cold]
    fn bad_read(&self, addr: u32, width: u8, message: &str) {
        let width_str = match width {
            8 => " 8bit",
            16 => " 16bit",
            32 => " 32bit",
            _ => "",
        };

        if message.len() > 0 {
            gba_error!("bad{} read from 0x{:08X}: {}", width_str, addr, message);
        } else {
            gba_error!("bad{} read from 0x{:08X}", width_str, addr);
        }
    }

    /// Called after a bad write has occurred.
    #[cold]
    fn bad_write(&self, addr: u32, value: u32, width: u8, mut message: &str) {
        if message.len() == 0 {
            message = "unknown cause";
        }

        match width {
            8 => {
                gba_error!("bad 8-bit write to 0x{:08X} of value 0x{:02X}: {}", addr, value, message);
            },

            16 => {
                gba_error!("bad 16-bit write to 0x{:08X} of value 0x{:02X}: {}", addr, value, message);
            },

            32 => {
                gba_error!("bad 32-bit write to 0x{:08X} of value 0x{:02X}: {}", addr, value, message);
            },

            _ => {
                gba_error!("bad (unknown width???) write to 0x{:08X} of value 0x{:02X}: {}", addr, value, message);
            },
        }
    }

    /// When this returns true, both internal and external work RAM are disabled and
    /// return the previously prefetched instruction when read.
    #[inline]
    pub fn is_ram_disabled(&self) -> bool {
        self.ioregs.internal_memory_control.ram_disabled()
    }

    /// When this returns true, external word ram becomes a mirror of internal work ram.
    #[inline]
    pub fn is_external_ram_disabled(&self) -> bool {
        !self.ioregs.internal_memory_control.external_ram_enabled()
    }

    /// Set the recent prefetch values that the memory should use when returning the value
    /// of invalid reads.
    pub fn set_prefetch(&mut self, prefetch_addr: u32, prefetch_value: u32) {
        self.recent_prefetch = prefetch_value;
        if prefetch_addr < 0x00004000 {
            self.recent_bios_prefetch = prefetch_value;
        }
    }
}

impl ArmMemory for GbaMemory {
    fn load8(&mut self, addr: u32) -> u8 {
        self.read_byte(addr)
    }

    fn view8(&self, addr: u32) -> u8 {
        self.read_byte(addr)
    }

    fn store8(&mut self, addr: u32, value: u8) {
        self.write_byte(addr, value);
    }

    fn load16(&mut self, addr: u32) -> u16 {
        self.read_halfword(addr)
    }

    fn view16(&self, addr: u32) -> u16 {
        self.read_halfword(addr)
    }

    fn store16(&mut self, addr: u32, value: u16) {
        self.write_halfword(addr, value);
    }

    fn load32(&mut self, addr: u32) -> u32 {
        self.read_word(addr)
    }

    fn view32(&self, addr: u32) -> u32 {
        self.read_word(addr)
    }

    fn store32(&mut self, addr: u32, value: u32) {
        self.write_word(addr, value);
    }

    fn code_access_seq8(&self, addr: u32) -> u32 {
        self.code_access_byte_seq(addr)
    }

    fn data_access_seq8(&self, addr: u32) -> u32 {
        self.data_access_byte_seq(addr)
    }

    fn code_access_nonseq8(&self, addr: u32) -> u32 {
        self.code_access_byte_nonseq(addr)
    }

    fn data_access_nonseq8(&self, addr: u32) -> u32 {
        self.data_access_byte_nonseq(addr)
    }

    fn code_access_seq16(&self, addr: u32) -> u32 {
        self.code_access_halfword_seq(addr)
    }

    fn data_access_seq16(&self, addr: u32) -> u32 {
        self.data_access_halfword_seq(addr)
    }

    fn code_access_nonseq16(&self, addr: u32) -> u32 {
        self.code_access_halfword_nonseq(addr)
    }

    fn data_access_nonseq16(&self, addr: u32) -> u32 {
        self.data_access_halfword_nonseq(addr)
    }

    fn code_access_seq32(&self, addr: u32) -> u32 {
        self.code_access_word_seq(addr)
    }

    fn data_access_seq32(&self, addr: u32) -> u32 {
        self.code_access_word_seq(addr)
    }

    fn code_access_nonseq32(&self, addr: u32) -> u32 {
        self.code_access_word_nonseq(addr)
    }

    fn data_access_nonseq32(&self, addr: u32) -> u32 {
        self.data_access_word_nonseq(addr)
    }
}

const REGION_BIOS: u32      = 0x00;
const REGION_UNUSED: u32    = 0x01;
const REGION_EWRAM: u32     = 0x02;
const REGION_IWRAM: u32     = 0x03;
const REGION_IOREG: u32     = 0x04;
const REGION_PAL: u32       = 0x05;
const REGION_VRAM: u32      = 0x06;
const REGION_OAM: u32       = 0x07;
const REGION_CART0_L: u32   = 0x08;
const REGION_CART0_H: u32   = 0x09;
const REGION_CART1_L: u32   = 0x0A;
const REGION_CART1_H: u32   = 0x0B;
const REGION_CART2_L: u32   = 0x0C;
const REGION_CART2_H: u32   = 0x0D;
const REGION_SRAM: u32      = 0x0E;

const REGION_BIOS_LEN: usize    = kb!(16);
const REGION_EWRAM_LEN: usize   = kb!(256);
const REGION_IWRAM_LEN: usize   = kb!(32);
#[cfg(test)] const REGION_PAL_LEN: usize     = kb!(1);
const REGION_PAL_LEN32: u32     = kb!(1);
const REGION_VRAM_LEN: usize    = kb!(96);
const REGION_OAM_LEN: usize     = kb!(1);

// fn is_cart_region(region: u32) -> bool {
//     match region {
//         REGION_CART0_L | REGION_CART0_H |
//         REGION_CART1_L | REGION_CART1_H |
//         REGION_CART2_L | REGION_CART2_H  => true,
//         _ => false,
//     }
// }

// fn get_region_cart(region: u32) -> Option<u32> {
//     match region {
//         REGION_CART0_L | REGION_CART0_H => Some(0),
//         REGION_CART1_L | REGION_CART1_H => Some(1),
//         REGION_CART2_L | REGION_CART2_H => Some(2),
//         _ => None,
//     }
// }

// like get_halfword_of_word but for bytes
const fn get_byte_of_word(word: u32, addr: u32) -> u8 {
    (word >> ((addr % 4) * 8)) as u8
}

// Given a 32bit word and a 16bit aligned 32bit address (that would be referencing the given word
// if it was word aligned), this function will return the half of the word referenced by the 16bit
// aligned word. I don't have a better way to describe this so here is an example:
//
// ```
// let word = 0xDEADBEEF;
//
// let lo = 0x04000000;
// let hi = 0x04000002;
//
// get_halfword_of_word(word, lo) // 0xBEEF
// get_halfword_of_word(word, hi) // 0xDEAD
// ```
#[inline(always)]
const fn get_halfword_of_word(word: u32, addr: u32) -> u16 {
    (word >> ((addr & 0x2) * 4)) as u16
}

// see: `get_halfword_of_word`
#[inline(always)]
fn set_halfword_of_word(word: u32, addr: u32, value: u16) -> u32 {
    let off = (addr & 0x2) * 4;
    (word & !(0xFFFF << off)) | ((value as u32) << off)
}

/// Converts a 32bit address inthe VRAM region into a physical offset into the VRAM buffer.
#[inline(always)]
fn to_vram_physical_addr(addr: u32) -> usize {
    // Even though VRAM is sized 96K (64K+32K), it is repeated in steps of 128K (64K+32K+32K, the two
    // 32K blocks itself being mirrors of each other).
    let vram128 = addr % kb!(128); // offset in 128KB block

    if vram128 >= kb!(96) {
        // this means that this address is in the later 32KB block so we just subtract 32KB to
        // mirror the previous one:
        vram128 as usize - kb!(32)
    } else {
        vram128 as usize
    }
}

#[inline(always)]
const fn region_of(addr: u32) -> u32 {
    addr >> 24
}

#[inline(always)]
const fn align16(addr: u32) -> u32 {
    addr & 0xFFFFFFFE
}

#[inline(always)]
const fn align32(addr: u32) -> u32 {
    addr & 0xFFFFFFFC
}


/// Reads a u16 from a byte array in little endian byte order.
#[inline]
pub fn read16_le(mem: &[u8], offset: usize) -> u16 {
    assert!(mem.len() > offset + 1, "16bit read out of range (offset: {}, len: {})", offset, mem.len());
    (mem[offset] as u16) | ((mem[offset + 1] as u16) <<  8)
}

/// Reads a u32 from a byte array in little endian byte order.
#[inline]
pub fn read32_le(mem: &[u8], offset: usize) -> u32 {
    assert!(mem.len() > offset + 3, "32bit read out of range (offset: {}, len: {})", offset, mem.len());
    (mem[offset] as u32) |
        ((mem[offset + 1] as u32) <<  8) |
        ((mem[offset + 2] as u32) << 16) |
        ((mem[offset + 3] as u32) << 24)
}

/// Writes a u16 into a byte array in little endian byte order.
#[inline]
pub fn write16_le(mem: &mut [u8], offset: usize, value: u16) {
    assert!(mem.len() > offset + 1, "16bit write out of range (offset: {}, len: {})", offset, mem.len());
    mem[offset] = value as u8;
    mem[offset + 1] = (value >> 8) as u8;
}

/// Writes a u32 into a byte array in little endian byte order.
#[inline]
pub fn write32_le(mem: &mut [u8], offset: usize, value: u32) {
    assert!(mem.len() > offset + 3, "32bit write out of range (offset: {}, len: {})", offset, mem.len());
    mem[offset] = value as u8;
    mem[offset + 1] = (value >>  8) as u8;
    mem[offset + 2] = (value >> 16) as u8;
    mem[offset + 3] = (value >> 24) as u8;
}

#[cfg(test)]
mod test {
    use super::*;

    // macro_rules! assert_hex_eq {
    //     ($lhs:expr, $rhs:expr) => {
    //     }
    // }

    #[test]
    fn test_mirrors() {
        let mut memory = GbaMemory::new(true);

        check_mirrors(&mut memory, REGION_EWRAM, REGION_EWRAM_LEN, 0xDEADBEEF);
        check_mirrors(&mut memory, REGION_IWRAM, REGION_IWRAM_LEN, 0xDEADBEEF);
        check_mirrors(&mut memory, REGION_PAL, REGION_PAL_LEN, 0xDEADBEEF);
        check_mirrors(&mut memory, REGION_VRAM, 1024 * 128, 0xDEADBEEF);
        check_mirrors(&mut memory, REGION_OAM, REGION_OAM_LEN, 0xDEADBEEF);

        memory.write_word(start_of_region(REGION_EWRAM), 0xDEADBEEF);
        memory.ioregs.internal_memory_control.set_external_ram_enabled(false);
        memory.write_word(start_of_region(REGION_EWRAM), 0xBEEFDEAD);
        assert_eq!(0xBEEFDEAD, memory.read_word(start_of_region(REGION_IWRAM)));
        memory.ioregs.internal_memory_control.set_external_ram_enabled(true);
        assert_eq!(0xDEADBEEF, memory.read_word(start_of_region(REGION_EWRAM)));

        fn check_mirrors(memory: &mut GbaMemory, region: u32, region_len: usize, value: u32) {
            let region_len = region_len as u32;

            let addr_start = start_of_region(region);
            let addr_middle = addr_start + (region_len / 2);
            let addr_end = addr_start + region_len - 4;

            let value_start = value;
            let value_middle = value.wrapping_mul(value);
            let value_end = value_middle.wrapping_mul(value);

            memory.write_word(addr_start, value_start);
            memory.write_word(addr_middle, value_middle);
            memory.write_word(addr_end, value_end);

            for mirror in 0..3 {
                println!("checking mirror #{} of region 0x{:02X}", mirror, region);

                let addr_mirror_start = addr_start + (region_len * mirror);
                let addr_mirror_middle = addr_mirror_start + (region_len / 2);
                let addr_mirror_end = addr_mirror_start + region_len - 4;

                assert_eq!(memory.read_word(addr_mirror_start), value_start);
                assert_eq!(memory.read_word(addr_mirror_middle), value_middle);
                assert_eq!(memory.read_word(addr_mirror_end), value_end);

                assert_eq!(memory.read_halfword(addr_mirror_start), value_start as u16);
                assert_eq!(memory.read_halfword(addr_mirror_middle), value_middle as u16);
                assert_eq!(memory.read_halfword(addr_mirror_end), value_end as u16);

                assert_eq!(memory.read_byte(addr_mirror_start), value_start as u8);
                assert_eq!(memory.read_byte(addr_mirror_middle), value_middle as u8);
                assert_eq!(memory.read_byte(addr_mirror_end), value_end as u8);
            }
        }
    }

    #[test]
    fn test_io_registers() {
        let mut memory = GbaMemory::new(true);

        memory.set_prefetch(0, 0xCECECECE); // easier to catch errors this way

        // Make sure that the internal memory control is initialized correctly:
        assert_eq!(false, memory.ioregs.internal_memory_control.ram_disabled());
        assert_eq!(true, memory.ioregs.internal_memory_control.external_ram_enabled());
        assert_eq!(2, 15 - memory.ioregs.internal_memory_control.external_ram_wait());

        // Simple IO register write:
        memory.write_halfword(0x04000008, 0xFBEA);
        assert_eq!(memory.read_halfword(0x04000008), 0xFBEA);
        assert_eq!(memory.ioregs.bg_cnt[0].inner, 0xFBEA);

        // Writing to 2 IO registers at once:
        memory.write_word(0x04000014, 0xBEE5CABE);
        assert_eq!(memory.read_halfword(0x04000014), 0xCABE);
        assert_eq!(memory.read_halfword(0x04000016), 0xBEE5);
        assert_eq!(memory.ioregs.bg_hofs[1].inner, 0xCABE);
        assert_eq!(memory.ioregs.bg_vofs[1].inner, 0xBEE5);

        // Partial Register Write:
        memory.write_byte(0x04000018, 0xDD);
        assert_eq!(memory.read_byte(0x04000018), 0xDD);
        assert_eq!(memory.ioregs.bg_hofs[2].inner as u8, 0xDD);

        // Partial Register Read
        memory.write_halfword(0x04000018, 0xEFBE);
        assert_eq!(memory.read_byte(0x04000018), 0xBE);
        assert_eq!(memory.read_byte(0x04000019), 0xEF);
    }

    #[test]
    fn test_bad_valid_reads() {
        let mut memory = GbaMemory::new(true);

        memory.set_prefetch(0x00000000, 0xCECECECE); // BIOS Prefetch
        memory.set_prefetch(0x08000000, 0xFEFEFEFE); // Normal Prefetch

        // Reading from BIOS without permission:
        memory.bios_readable = true;
        assert_ne!(memory.read_word(0), 0xCECECECE);
        memory.bios_readable = false;
        assert_eq!(memory.read_word(0), 0xCECECECE);

        // Reading from unused region:
        assert_eq!(memory.read_word(0x01000000), 0xFEFEFEFE);

        // Reading from empty cartridge:
        let addr_empty_cartridge = 0x0800FCCC;
        assert_eq!(memory.read_halfword(addr_empty_cartridge), (addr_empty_cartridge / 2) as u16);

        // Reading from unused IO port:
        //
        // Returns last prefetched instruction when the entire 32bit memory fragment is
        // Unused (eg. 0E0h) and/or Write-Only (eg.  DMA0SAD). And otherwise, returns
        // zero if the lower 16bit fragment is readable (eg.  04Ch=MOSAIC, 04Eh=NOTUSED/ZERO).
        memory.write_halfword(0x0400004C, 0xBEEF);
        assert_eq!(memory.read_halfword(0x0400004E), 0xFEFE);
        assert_eq!(memory.read_word(0x0400004C), 0);
    }

    fn start_of_region(region: u32) -> u32 {
        return region << 24
    }

    #[test]
    fn test_cycles() {
        let memory = GbaMemory::new(true);

        macro_rules! region_nonseq_test {
            ($Region:expr, $Byte:expr, $Halfword:expr, $Word:expr) => {
                assert_eq!($Byte, memory.data_access_byte_nonseq(start_of_region($Region)));
                assert_eq!($Halfword, memory.data_access_halfword_nonseq(start_of_region($Region)));
                assert_eq!($Word, memory.data_access_word_nonseq(start_of_region($Region)));
            }
        }

        region_nonseq_test!(REGION_BIOS, 1, 1, 1);
        region_nonseq_test!(REGION_IWRAM, 1, 1, 1);
        region_nonseq_test!(REGION_IOREG, 1, 1, 1);
        region_nonseq_test!(REGION_OAM, 1, 1, 1);
        region_nonseq_test!(REGION_EWRAM, 3, 3, 6);
        region_nonseq_test!(REGION_VRAM, 1, 1, 2);
        region_nonseq_test!(REGION_CART0_L, 5, 5, 8);
        region_nonseq_test!(REGION_SRAM, 5, 5, 5);

        // Address Bus Width and CPU Read/Write Access Widths
        // Shows the Bus-Width, supported read and write widths, and the clock cycles for 8/16/32bit accesses.
        //
        //   Region        Bus   Read      Write     Cycles
        //   BIOS ROM      32    8/16/32   -         1/1/1
        //   Work RAM 32K  32    8/16/32   8/16/32   1/1/1
        //   I/O           32    8/16/32   8/16/32   1/1/1
        //   OAM           32    8/16/32   16/32     1/1/1 *
        //   Work RAM 256K 16    8/16/32   8/16/32   3/3/6 **
        //   Palette RAM   16    8/16/32   16/32     1/1/2 *
        //   VRAM          16    8/16/32   16/32     1/1/2 *
        //   GamePak ROM   16    8/16/32   -         5/5/8 **/***
        //   GamePak Flash 16    8/16/32   16/32     5/5/8 **/***
        //   GamePak SRAM  8     8         8         5     **
    }
}
