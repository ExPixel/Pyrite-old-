
macro_rules! set_bit   { ($v:expr, $b:expr) => ( $v |= 1  << $b ) }
macro_rules! clear_bit { ($v:expr, $b:expr) => ( $v &= !(1 << $b) ) }
macro_rules! put_bit   {
    ($v:expr, $b:expr, $bv:expr) => ( if $bv {
        set_bit!($v, $b)
    }  else {
        clear_bit!($v, $b)
    })
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u32)]
pub enum CpuMode {
    /// User mode (usr) is the usual ARM program execution state,
    /// and is used for executing most application programs.
    User = 0b10000,

    /// System mode is a priviledged user mode for the operating system.
    /// NOTE: System mode can only be entered from another priviledged mode
    /// by modifying the the mode bit of the Current Program Status Register (CPSR),
    System = 0b11111,

    /// Fast Interrupt (FIQ) mode supports a data transfer or channel process.
    FIQ = 0b10001,

    /// Interrupt (IRQ) mode is used for general-purpose interrupt handling.
    IRQ = 0b10010,

    /// Supervisor mode is a protected mode for the operating system.
    Supervisor = 0b10011,

    /// Abort mode is entered after a data or instruction prefetch Abort.
    Abort = 0b10111,

    /// Undefined mode is entered when an undefined instruction is executed.
    Undefined = 0b11011,

    /// Used to represent any mode that is not defined by the ARMv4T instruction set.
    Invalid = 0b00000,
}

impl CpuMode {
    pub fn is_priviledged(self) -> bool {
        self != CpuMode::User && self != CpuMode::Invalid
    }

    pub fn from_bits(mode_bits: u32) -> CpuMode {
        match mode_bits {
            0b10000 => CpuMode::User,
            0b11111 => CpuMode::System,
            0b10001 => CpuMode::FIQ,
            0b10010 => CpuMode::IRQ,
            0b10011 => CpuMode::Supervisor,
            0b10111 => CpuMode::Abort,
            0b11011 => CpuMode::Undefined,
            _ => CpuMode::Invalid,
        }
    }

    #[inline(always)]
    pub fn bits(self) -> u32 {
        self as u32
    }
}


#[inline]
fn check_cpu_mode(mode_bits: u32) -> CpuMode {
    let mode = CpuMode::from_bits(mode_bits);
    if mode == CpuMode::Invalid {
        log::debug!("invalid CPU mode: {:05b}", mode_bits);
    }
    return mode;
}


pub struct ArmRegisters {
    /// The currently in use general purpose registers (r0-r15).
    gp_registers: [u32; 16],

    /// Banked registers for non user modes:
    /// - 0-4:   r8_fiq - r12_fiq
    /// - 5-6:   r13_fiq & r14_fiq
    /// - 7-8:   r13_svc & r14_svc
    /// - 9-10:  r13_abt & r14_abt
    /// - 11-12: r13_irq & r14_irq
    /// - 13-14: r13_und & r14_und
    bk_registers: [u32; 15],

    /// banked Saved Program Status Registers (SPSR)
    bk_spsr: [u32; 5],

    /// Current Program Status Register
    cpsr: u32,

    /// Saved Program Status Register
    spsr: u32,

    // ## DEBUGGING
    // These keep track of the value of the program counter (minus 2 instructions) when a register
    // was changed.
    #[cfg(feature = "track_register_writes")] gp_registers_record: [u32; 16],
    #[cfg(feature = "track_register_writes")] bk_registers_record: [u32; 15],
}

impl ArmRegisters {
    pub fn new(mode: CpuMode) -> ArmRegisters {
        ArmRegisters {
            gp_registers:   [0; 16],
            bk_registers:   [0; 15],
            bk_spsr:        [0;  5],
            cpsr:           mode.bits(),
            spsr:           0,

            #[cfg(feature = "track_register_writes")] gp_registers_record:    [0; 16],
            #[cfg(feature = "track_register_writes")] bk_registers_record:    [0; 15],
        }
    }

    /// Reads and returns the value of a general purpose register.
    #[inline(always)]
    pub fn read(&self, register: u32) -> u32 {
        self.gp_registers[register as usize]
    }

    /// Writes a value to a register.
    #[inline(always)]
    pub fn write(&mut self, register: u32, value: u32) {
        self.gp_registers[register as usize] = value;

        #[cfg(feature = "track_register_writes")]
        {
            let exec_addr = self.gp_registers[15] - (if self.getf_t() { 4 } else { 8 });
            self.gp_registers_record[register as usize] = exec_addr;
        }
    }

    #[cfg(feature = "track_register_writes")]
    #[inline(always)]
    pub fn register_change_location(&self, register: u32) -> u32 {
        self.gp_registers_record[register as usize]
    }

    #[cfg(not(feature = "track_register_writes"))]
    #[inline(always)]
    pub fn register_change_location(&self, _register: u32) -> u32 {
        0
    }

    pub fn write_with_mode(&mut self, tmp_mode: CpuMode, register: u32, value: u32) {
        let old_mode = self.read_mode();
        self.write_mode(tmp_mode);
        self.write(register, value);
        self.write_mode(old_mode);
    }

    /// Returns the current value of the N (Negative or Less Than) flag in the CPSR.
    #[inline(always)] pub fn getf_n(&self) -> bool { (self.cpsr & (1 << 31)) != 0 }
    /// Returns the current value of the Z (Zero) flag in the CPSR.
    #[inline(always)] pub fn getf_z(&self) -> bool { (self.cpsr & (1 << 30)) != 0 }
    /// Returns the current value of the C (Carry) flag in the CPSR.
    #[inline(always)] pub fn getf_c(&self) -> bool { (self.cpsr & (1 << 29)) != 0 }
    /// Returns the current value of the V (Overflow) flag in the CPSR.
    #[inline(always)] pub fn getf_v(&self) -> bool { (self.cpsr & (1 << 28)) != 0 }
    /// Returns the current value of the I (IRQ Disable) flag in the CPSR.
    #[inline(always)] pub fn getf_i(&self) -> bool { (self.cpsr & (1 <<  7)) != 0 }
    /// Returns the current value of the F (FIQ Disable) flag in the CPSR.
    #[inline(always)] pub fn getf_f(&self) -> bool { (self.cpsr & (1 <<  6)) != 0 }
    /// Returns the current value of the T (State/Thumb) flag in the CPSR.
    #[inline(always)] pub fn getf_t(&self) -> bool { (self.cpsr & (1 <<  5)) != 0 }

    /// Sets the N (Negative or Less Than) flag in the CPSR.
    #[inline(always)] pub fn setf_n(&mut self) { set_bit!(self.cpsr, 31); }
    /// Sets the Z (Zero) flag in the CPSR.
    #[inline(always)] pub fn setf_z(&mut self) { set_bit!(self.cpsr, 30); }
    /// Sets the C (Carry) flag in the CPSR.
    #[inline(always)] pub fn setf_c(&mut self) { set_bit!(self.cpsr, 29); }
    /// Sets the V (Overflow) flag in the CPSR.
    #[inline(always)] pub fn setf_v(&mut self) { set_bit!(self.cpsr, 28); }
    /// Sets the I (IRQ Disable) flag in the CPSR.
    #[inline(always)] pub fn setf_i(&mut self) { set_bit!(self.cpsr,  7); }
    /// Sets the F (FIQ Disable) flag in the CPSR.
    #[inline(always)] pub fn setf_f(&mut self) { set_bit!(self.cpsr,  6); }
    /// Sets the T (State/Thumb) flag in the CPSR.
    #[inline(always)] pub fn setf_t(&mut self) { set_bit!(self.cpsr,  5); }

    /// Clears the N (Negative or Less Than) flag in the CPSR.
    #[inline(always)] pub fn clearf_n(&mut self) { clear_bit!(self.cpsr, 31); }
    /// Clears the Z (Zero) flag in the CPSR.
    #[inline(always)] pub fn clearf_z(&mut self) { clear_bit!(self.cpsr, 30); }
    /// Clears the C (Carry) flag in the CPSR.
    #[inline(always)] pub fn clearf_c(&mut self) { clear_bit!(self.cpsr, 29); }
    /// Clears the V (Overflow) flag in the CPSR.
    #[inline(always)] pub fn clearf_v(&mut self) { clear_bit!(self.cpsr, 28); }
    /// Clears the I (IRQ Disable) flag in the CPSR.
    #[inline(always)] pub fn clearf_i(&mut self) { clear_bit!(self.cpsr,  7); }
    /// Clears the F (FIQ Disable) flag in the CPSR.
    #[inline(always)] pub fn clearf_f(&mut self) { clear_bit!(self.cpsr,  6); }
    /// Clears the T (State/Thumb) flag in the CPSR.
    #[inline(always)] pub fn clearf_t(&mut self) { clear_bit!(self.cpsr,  5); }

    /// Changes the N (Negative or Less Than) flag in the CPSR to a given value.
    #[inline(always)] pub fn putf_n(&mut self, val: bool) { put_bit!(self.cpsr, 31, val); }
    /// Changes the Z (Zero) flag in the CPSR to a given value.
    #[inline(always)] pub fn putf_z(&mut self, val: bool) { put_bit!(self.cpsr, 30, val); }
    /// Changes the C (Carry) flag in the CPSR to a given value.
    #[inline(always)] pub fn putf_c(&mut self, val: bool) { put_bit!(self.cpsr, 29, val); }
    /// Changes the V (Overflow) flag in the CPSR to a given value.
    #[inline(always)] pub fn putf_v(&mut self, val: bool) { put_bit!(self.cpsr, 28, val); }
    /// Changes the I (IRQ Disable) flag in the CPSR to a given value.
    #[inline(always)] pub fn putf_i(&mut self, val: bool) { put_bit!(self.cpsr,  7, val); }
    /// Changes the F (FIQ Disable) flag in the CPSR to a given value.
    #[inline(always)] pub fn putf_f(&mut self, val: bool) { put_bit!(self.cpsr,  6, val); }
    /// Changes the T (State/Thumb) flag in the CPSR to a given value.
    #[inline(always)] pub fn putf_t(&mut self, val: bool) { put_bit!(self.cpsr,  5, val); }

    /// Changes the N (Negative or Less Than) flag in the CPSR to a given value.
    #[inline(always)] pub fn putfi_n<I: Into<u32>>(&mut self, val: I) { self.putf_n(val.into() != 0) }
    /// Changes the Z (Zero) flag in the CPSR to a given value.
    #[inline(always)] pub fn putfi_z<I: Into<u32>>(&mut self, val: I) { self.putf_z(val.into() != 0) }
    /// Changes the C (Carry) flag in the CPSR to a given value.
    #[inline(always)] pub fn putfi_c<I: Into<u32>>(&mut self, val: I) { self.putf_c(val.into() != 0) }
    /// Changes the V (Overflow) flag in the CPSR to a given value.
    #[inline(always)] pub fn putfi_v<I: Into<u32>>(&mut self, val: I) { self.putf_v(val.into() != 0) }
    /// Changes the I (IRQ Disable) flag in the CPSR to a given value.
    #[inline(always)] pub fn putfi_i<I: Into<u32>>(&mut self, val: I) { self.putf_i(val.into() != 0) }
    /// Changes the F (FIQ Disable) flag in the CPSR to a given value.
    #[inline(always)] pub fn putfi_f<I: Into<u32>>(&mut self, val: I) { self.putf_f(val.into() != 0) }
    /// Changes the T (State/Thumb) flag in the CPSR to a given value.
    #[inline(always)] pub fn putfi_t<I: Into<u32>>(&mut self, val: I) { self.putf_t(val.into() != 0) }

    /// Sets the mode of the CPU. This will also change the mode bits in the CPSR register
    /// and properly swap register values to their corresponding banked values for the new mode.
    pub fn write_mode(&mut self, new_mode: CpuMode) {
        let old_mode = self.read_mode();
        self.on_mode_switch(old_mode, new_mode);
        self.cpsr = (self.cpsr & 0xFFFFFFE0) | new_mode.bits();
    }

    /// Sets the mode bits of the CPSR register. This will also change the mode of the CPU
    /// and properly swap register values to their corresponding banked values for the new mode.
    pub fn write_mode_bits(&mut self, mode_bits: u32) {
        let old_mode = self.read_mode();
        let new_mode = check_cpu_mode(mode_bits);
        self.on_mode_switch(old_mode, new_mode);
        self.cpsr = (self.cpsr & 0xFFFFFFE0) | mode_bits;
    }

    /// Returns the current mode of the CPU.
    #[inline(always)]
    pub fn read_mode(&mut self) -> CpuMode {
        check_cpu_mode(self.cpsr & 0x1F)
    }

    /// Returns the current mode bits of the CPSR register (lowest 5bits) will all other bits set to 0.
    #[inline(always)]
    pub fn read_mode_bits(&mut self) -> u32 { self.cpsr & 0x1F }

    /// Returns the value of the CPSR register.
    #[inline(always)]
    pub fn read_cpsr(&self) -> u32 { self.cpsr }

    /// Sets the value of the CPSR. If the mode bits are changed
    /// The mode of the CPU will be changed accordingly and banked registers will be loaded.
    pub fn write_cpsr(&mut self, value: u32) {
        let old_mode_bits = self.read_mode_bits();
        self.cpsr = value;
        let new_mode_bits = self.read_mode_bits();

        if old_mode_bits != new_mode_bits {
            let old_mode = CpuMode::from_bits(old_mode_bits);
            let new_mode = check_cpu_mode(new_mode_bits);
            self.on_mode_switch(old_mode, new_mode);
        }
    }

    // #TODO(LOW): might want to make this panic or show a warning in debug mode
    //             when it is called and the CPU is in User or System mode.
    /// Reads the value of the Saved Program Status Register (SPSR)
    /// for the current mode. This will return a garbage value for the User and
    /// System modes.
    #[inline(always)]
    pub fn read_spsr(&self) -> u32 { self.spsr }

    // #TODO(LOW): might want to make this panic or show a warning in debug mode
    //             when it is called and the CPU is in User or System mode.
    /// Writes to the Saved Program Status Register (SPSR)
    /// for the current mode. In this emulation all modes have an SPSRs but the System
    /// and User mode SPSRs are not saved on a mode switch.
    #[inline(always)]
    pub fn write_spsr(&mut self, value: u32) {
        self.spsr = value;
    }

    /// Called during a mode switch to switch the general purpose registers
    /// and the spsr to their proper banked versions.
    fn on_mode_switch(&mut self, old_mode: CpuMode, new_mode: CpuMode) {
        use std::mem::swap;

        #[cfg(not(feature = "track_register_writes"))]
        macro_rules! swap_reg {
            (gp=$gp_reg:expr, bk=$bk_reg:expr) => {
                swap(&mut self.gp_registers[$gp_reg], &mut self.bk_registers[$bk_reg]);
            }
        }

        #[cfg(feature = "track_register_writes")]
        macro_rules! swap_reg {
            (gp=$gp_reg:expr, bk=$bk_reg:expr) => {
                swap(&mut self.gp_registers[$gp_reg], &mut self.bk_registers[$bk_reg]);
                swap(&mut self.gp_registers_record[$gp_reg], &mut self.bk_registers_record[$bk_reg]);
            }
        }

        if old_mode == new_mode {
            /* NOP */
            return
        }

        if old_mode != CpuMode::User && old_mode != CpuMode::System {
            // if the old mode isn't user or system (which are our default modes)
            // change to system mode:
            match old_mode {
                CpuMode::FIQ => {
                    swap_reg!(gp= 8, bk=0);
                    swap_reg!(gp= 9, bk=1);
                    swap_reg!(gp=10, bk=2);
                    swap_reg!(gp=11, bk=3);
                    swap_reg!(gp=12, bk=4);
                    swap_reg!(gp=13, bk=5);
                    swap_reg!(gp=14, bk=6);
                    self.bk_spsr[0] = self.spsr;
                },

                CpuMode::Supervisor    => {
                    swap_reg!(gp=13, bk=7);
                    swap_reg!(gp=14, bk=8);
                    self.bk_spsr[1] = self.spsr;
                },

                CpuMode::Abort         => {
                    swap_reg!(gp=13, bk= 9);
                    swap_reg!(gp=14, bk=10);
                    self.bk_spsr[2] = self.spsr;
                },

                CpuMode::IRQ           => {
                    swap_reg!(gp=13, bk=11);
                    swap_reg!(gp=14, bk=12);
                    self.bk_spsr[3] = self.spsr;
                },

                CpuMode::Undefined     => {
                    swap_reg!(gp=13, bk=13);
                    swap_reg!(gp=14, bk=14);
                    self.bk_spsr[4] = self.spsr;
                },

                CpuMode::User | CpuMode::System => { /* NOP */ },

                _ => unreachable!("bad old cpu mode in on_mode_switch: {old_mode}"),
            }
        }

        // now we can continue on as if we're switching from system mode.

        match new_mode {
            CpuMode::FIQ => {
                swap_reg!(gp= 8, bk=0);
                swap_reg!(gp= 9, bk=1);
                swap_reg!(gp=10, bk=2);
                swap_reg!(gp=11, bk=3);
                swap_reg!(gp=12, bk=4);
                swap_reg!(gp=13, bk=5);
                swap_reg!(gp=14, bk=6);
                self.spsr = self.bk_spsr[0];
            },

            CpuMode::Supervisor    => {
                swap_reg!(gp=13, bk=7);
                swap_reg!(gp=14, bk=8);
                self.spsr = self.bk_spsr[1];
            },

            CpuMode::Abort         => {
                swap_reg!(gp=13, bk= 9);
                swap_reg!(gp=14, bk=10);
                self.spsr = self.bk_spsr[2];
            },

            CpuMode::IRQ     => {
                swap_reg!(gp=13, bk=11);
                swap_reg!(gp=14, bk=12);
                self.spsr = self.bk_spsr[3];
            },

            CpuMode::Undefined     => {
                swap_reg!(gp=13, bk=13);
                swap_reg!(gp=14, bk=14);
                self.spsr = self.bk_spsr[4];
            },

            CpuMode::User | CpuMode::System => { /* NOP */ },


            _ => unreachable!("bad new cpu mode in on_mode_switch: {new_mode}"),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Tests register read/write functionality
    /// as well as mode and bank switches.
    #[test]
    fn register_rw_and_bank_switching() {
        let mut registers = ArmRegisters::new(CpuMode::System);

        // first write some random values to banked and unbanked register locations:
        registers.write( 0, 428983);
        registers.write( 4, 736834);
        registers.write( 9, 234);
        registers.write(12, 15);
        registers.write(14, 3373);

        // first we change the mode via set_mode
        registers.write_mode(CpuMode::FIQ);
        assert_eq!(registers.read_mode(), CpuMode::FIQ);
        // then check that unbanked registers still have correct values
        assert_eq!(registers.read(0), 428983);
        assert_eq!(registers.read(4), 736834);
        // then we write some random values to the banked registers in this new mode
        registers.write(12, 4787);
        registers.write( 9, 1397);
        registers.write(14, 33387);
        // write to the spsr as well
        registers.write_spsr(897987);

        // we change the mode again via set_mode_bits this time
        registers.write_mode_bits(CpuMode::Abort as u32);
        assert_eq!(registers.read_mode(), CpuMode::Abort);
        // then check the unbanked registers again (this time 9 is also unbanked.)
        assert_eq!(registers.read(0), 428983);
        assert_eq!(registers.read(4), 736834);
        assert_eq!(registers.read(9), 234);
        // write to the banked registers in this new mode
        registers.write(13, 6846);
        registers.write(14, 761357);
        // write to the spsr as well
        registers.write_spsr(555);

        // we change the mode back to user mode (which should have the same registers as system mode)
        // but this time we do it by writing to the cpsr directly.
        let mut cpsr = registers.read_cpsr();
        cpsr = (cpsr & !0x1F) | CpuMode::User as u32;
        registers.write_cpsr(cpsr);
        assert_eq!(registers.read_mode(), CpuMode::User);
        // then we check all of the original registers:
        assert_eq!(registers.read( 0), 428983);
        assert_eq!(registers.read( 4), 736834);
        assert_eq!(registers.read( 9), 234);
        assert_eq!(registers.read(14), 3373);

        // now we go back to the used privilidged modes and make sure their banked registers
        // are correct.
        registers.write_mode(CpuMode::FIQ);
        assert_eq!(registers.read_mode(), CpuMode::FIQ);
        assert_eq!(registers.read( 0), 428983);
        assert_eq!(registers.read( 4), 736834);
        assert_eq!(registers.read( 9), 1397);
        assert_eq!(registers.read(12), 4787);
        assert_eq!(registers.read(14), 33387);
        assert_eq!(registers.read_spsr(), 897987);

        registers.write_mode(CpuMode::Abort);
        assert_eq!(registers.read_mode(), CpuMode::Abort);
        assert_eq!(registers.read( 0), 428983);
        assert_eq!(registers.read( 4), 736834);
        assert_eq!(registers.read( 9), 234);
        assert_eq!(registers.read(12), 15);
        assert_eq!(registers.read(14), 761357);
        assert_eq!(registers.read_spsr(), 555);
    }
}
