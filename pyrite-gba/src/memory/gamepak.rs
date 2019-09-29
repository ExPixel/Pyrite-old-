use super::{align32, read16_le, read32_le};

/// GBA cartridge memory.
pub struct GamePakROM {
    data: Vec<u8>,
}

impl GamePakROM {
    /// Create a new GamePakROM using the given binary.
    pub fn new(data: Vec<u8>) -> GamePakROM {
        GamePakROM { data, }
    }

    /// Append data/code to the GamePak ROM
    pub fn append(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    /// Clear the ROM data
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get the length of the ROM
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns a reference to the internal vector used by GamePakROM
    pub fn inner(&self) -> &Vec<u8> {
        &self.data
    }

    /// Returns a mutable reference to the internal vector used by GamePakROM
    pub fn inner_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    /// Returns the value that should be read when reading from an out of bounds
    /// address.
    #[inline(always)]
    fn bad_read_value_of(addr: u32) -> u16 {
        ((addr / 2) & 0xFFFF) as u16
    }

    pub fn read_byte(&self, addr: u32) -> u8 {
        let off = addr & 0x00FFFFFF;
        if off as usize > self.data.len() {
            let bad_read = Self::bad_read_value_of(addr);
            (bad_read >> (4 * (addr & 1))) as u8
        } else {
            self.data[off as usize]
        }
    }

    pub fn read_halfword(&self, unaligned_addr: u32) -> u16 {
        let aligned_off = unaligned_addr & 0x00FFFFFE;
        if aligned_off as usize >= self.data.len() {
            Self::bad_read_value_of(unaligned_addr)
        } else {
            read16_le(&self.data, aligned_off as usize)
        }
    }

    pub fn read_word(&self, unaligned_addr: u32) -> u32 {
        let aligned_addr = align32(unaligned_addr);
        let aligned_off = unaligned_addr & 0x00FFFFFC;

        if aligned_off as usize >= self.data.len() {
            let lo = Self::bad_read_value_of(aligned_addr) as u32;
            let hi = Self::bad_read_value_of(aligned_addr + 2) as u32;
            lo | (hi << 16)
        } else {
            read32_le(&self.data, aligned_off as usize)
        }
    }

    pub fn write_byte(&mut self, _addr: u32, _value: u8) -> Result<(), &'static str> {
        // @TODO add support for GPIO and other GamePak "ROM" writes
        Err("writing to GamePak ROM not yet supported")
    }

    pub fn write_halfword(&mut self, _unaligned_addr: u32, _value: u16) -> Result<(), &'static str> {
        // @TODO add support for GPIO and other GamePak "ROM" writes
        Err("writing to GamePak ROM not yet supported")
    }

    pub fn write_word(&mut self, _unaligned_addr: u32, _value: u32) -> Result<(), &'static str> {
        // @TODO add support for GPIO and other GamePak "ROM" writes
        Err("writing to GamePak ROM not yet supported")
    }
}
