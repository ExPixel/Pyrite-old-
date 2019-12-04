pub trait ArmMemory {
    /// The CPU will call this during its internal cycles.
    fn on_internal_cycles(&mut self, icycles: u32);

    fn     read_code_word(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u32;
    fn read_code_halfword(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u16;

    fn     read_data_word(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u32;
    fn read_data_halfword(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u16;
    fn     read_data_byte(&mut self, addr: u32, seq: bool, cycles: &mut u32) ->  u8;

    fn     write_data_word(&mut self, addr: u32, data: u32, seq: bool, cycles: &mut u32);
    fn write_data_halfword(&mut self, addr: u32, data: u16, seq: bool, cycles: &mut u32);
    fn     write_data_byte(&mut self, addr: u32, data:  u8, seq: bool, cycles: &mut u32);

    fn     view_word(&self, addr: u32) -> u32;
    fn view_halfword(&self, addr: u32) -> u16;
    fn     view_byte(&self, addr: u32) ->  u8;

    /// Branches will use this to get the cycles for a prefetch instead of actually doing reads.
    /// Used for optimization.
    fn code_cycles_word(&mut self, addr: u32, seq: bool) -> u32 {
        let mut cycles = 0;
        let _ = self.read_code_word(addr, seq, &mut cycles);
        return cycles;
    }

    /// Branches will use this to get the cycles for a prefetch instead of actually doing reads.
    /// Used for optimization.
    fn code_cycles_halfword(&mut self, addr: u32, seq: bool) -> u32 {
        let mut cycles = 0;
        let _ = self.read_code_halfword(addr, seq, &mut cycles);
        return cycles;
    }
}

impl ArmMemory for Vec<u8> {
    fn on_internal_cycles(&mut self, _icycles: u32) { /* NOP */ }

    fn read_code_word(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u32 {
        return self.read_data_word(addr, seq, cycles);
    }

    fn read_code_halfword(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u16 {
        return self.read_data_halfword(addr, seq, cycles);
    }

    fn read_data_word(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u32 {
        let addr = addr & 0xFFFFFFFC;
        let lo = self.read_data_halfword(addr, seq, cycles) as u32;
        let hi = self.read_data_halfword(addr + 2, seq, cycles) as u32;
        return lo | (hi << 16)
    }

    fn read_data_halfword(&mut self, addr: u32, seq: bool, cycles: &mut u32) -> u16 {
        let addr = addr & 0xFFFFFFFE;
        let lo = self.read_data_byte(addr, seq, cycles) as u16;
        let hi = self.read_data_byte(addr + 1, seq, cycles) as u16;
        return lo | (hi << 8);
    }

    fn read_data_byte(&mut self, addr: u32, seq: bool, cycles: &mut u32) ->  u8 {
        if seq { *cycles += 1 } else { *cycles += 2 }
        if let Some(data) = self.get(addr as usize) {
            *data
        } else {
            panic!("out of bounds read from 0x{:08X}", addr);
        }
    }

    fn write_data_word(&mut self, addr: u32, data: u32, seq: bool, cycles: &mut u32) {
        let addr = addr & 0xFFFFFFFC;
        self.write_data_byte(addr, data as u8, seq, cycles);
        self.write_data_byte(addr + 1, (data >> 8) as u8, seq, cycles);
        self.write_data_byte(addr + 2, (data >> 16) as u8, seq, cycles);
        self.write_data_byte(addr + 3, (data >> 24) as u8, seq, cycles);
    }

    fn write_data_halfword(&mut self, addr: u32, data: u16, seq: bool, cycles: &mut u32) {
        let addr = addr & 0xFFFFFFFE;
        self.write_data_byte(addr, data as u8, seq, cycles);
        self.write_data_byte(addr + 1, (data >> 8) as u8, seq, cycles);
    }

    fn write_data_byte(&mut self, addr: u32, data:  u8, seq: bool, cycles: &mut u32) {
        if seq { *cycles += 1 } else { *cycles += 2 }

        if let Some(dst) = self.get_mut(addr as usize) {
            *dst = data;
        } else {
            panic!("out of bounds write to 0x{:08X}", addr);
        }
    }

    fn view_word(&self, addr: u32) -> u32 {
        let addr = addr & 0xFFFFFFFC;
        let lo = self.view_halfword(addr) as u32;
        let hi = self.view_halfword(addr + 2) as u32;
        return lo | (hi << 16)
    }

    fn view_halfword(&self, addr: u32) -> u16 {
        let addr = addr & 0xFFFFFFFE;
        let lo = self.view_byte(addr) as u16;
        let hi = self.view_byte(addr + 1) as u16;
        return lo | (hi << 8);
    }

    fn view_byte(&self, addr: u32) ->  u8 {
        if let Some(data) = self.get(addr as usize) {
            *data
        } else {
            panic!("out of bounds read from 0x{:08X}", addr);
        }
    }
}
