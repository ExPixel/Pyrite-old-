use super::{ arm, thumb, clock };
use super::registers::{ CpuMode, ArmRegisters };

pub const EXCEPTION_BASE: u32 = 0;

pub struct ArmCpu {
    /// The last instruction that was fetched.
    fetched:    u32,

    /// This last instruction that was decoded.
    decoded:    u32,

    /// Temporary cycle count used by the currently running step if
    /// there is one.
    pub(crate) cycles: u32,

    /// The total number of cycles that have elapsed.
    total_cycles: u64,

    pub registers:  ArmRegisters,

    /// Called by the CPU when an exception (trap) has occurred in the CPU.
    /// If false is returned the CPU will continue execution of the exception
    /// and jump to the exception's vector. If true is returned execution is stopped.
    on_exception: Option<ExceptionHandler>,
}

impl ArmCpu {
    pub fn new() -> ArmCpu {
        ArmCpu {
            registers: ArmRegisters::new(CpuMode::System),
            decoded: 0,
            fetched: 0,
            cycles: 0,
            total_cycles: 0,
            on_exception: None,
        }
    }

    /// Like set_pc but should only be used after the CPU has branched to
    /// some location. This instruction assumes that sometime after it
    /// and before the next execution the CPU will fetch an instruction
    /// and advance the program counter by the size of one instruction.
    ///
    /// TLDR this should only be used while already inside of the `step`
    /// function.
    #[inline]
    pub(crate) fn arm_branch_to(&mut self, pc: u32, memory: &mut dyn ArmMemory) {
        self.decoded = memory.load32(pc);
        self.registers.write(15, pc + 4);
    }

    /// See `arm_branch_to`
    #[inline]
    pub(crate) fn thumb_branch_to(&mut self, pc: u32, memory: &mut dyn ArmMemory) {
        self.decoded = memory.load16(pc) as u32;
        self.registers.write(15, pc + 2);
    }

    /// Flushes the CPU's pipeline, sets the program counter
    /// and "fetches" and "decodes" the next instruction.
    /// PC will point to the instruction that will be fetched during the next step
    /// This should only be used after the CPU has been reset and is being
    /// prepared to start execution.
    pub fn set_pc(&mut self, value: u32, memory: &mut dyn ArmMemory) {
        if self.registers.getf_t() {
            self.thumb_set_pc(value, memory);
        } else {
            self.arm_set_pc(value, memory);
        }
    }

    #[inline]
    fn arm_set_pc(&mut self, value: u32, memory: &mut dyn ArmMemory) {
        self.decoded = memory.load32(value);
        self.fetched = memory.load32(value + 4);
        self.registers.write(15, value + 8); // the next fetch will be at EXECUTING_PC + 8
    }

    #[inline]
    fn thumb_set_pc(&mut self, value: u32, memory: &mut dyn ArmMemory) {
        self.decoded = memory.load16(value) as u32;
        self.fetched = memory.load16(value + 2) as u32;
        self.registers.write(15, value + 4); // the next fetch will be at EXECUTING_PC + 4
    }

    /// Resets a CPU's registers
    pub fn reset_registers(&mut self) {
        self.registers = ArmRegisters::new(CpuMode::Supervisor);
        self.registers.setf_i();
        self.registers.setf_f();
        self.registers.clearf_t();
    }

    /// Causes the CPU to run for one step, which is USUALLY one instruction
    /// but might not always be.
    pub fn step(&mut self, memory: &mut dyn ArmMemory) -> u32 {
        self.cycles = 0;
        if self.registers.getf_t() {
            self.step_thumb(memory);
        } else {
            self.step_arm(memory);
        }
        self.total_cycles += self.cycles as u64;
        return self.cycles;
    }

    fn step_arm(&mut self, memory: &mut dyn ArmMemory) {
        let opcode = self.decoded;
        self.decoded = self.fetched;

        let opcode_row = bits!(opcode, 20, 27);
        let opcode_col = bits!(opcode,  4,  7);
        let opcode_idx = (opcode_row * 16) + opcode_col;

        if opcode_idx < 4096 {
            if check_condition((opcode >> 28) & 0xF, &self.registers) {
                let arm_fn = arm::ARM_OPCODE_TABLE[opcode_idx as usize];
                arm_fn(self, memory, opcode);
            } else {
                self.cycles += clock::cycles_prefetch(memory, false, self.registers.read(15));
            }

            let pc = self.registers.read(15);
            if self.registers.getf_t() {
                self.fetched = memory.load16(pc) as u32;
                self.registers.write(15, pc + 2);
            } else {
                self.fetched = memory.load32(pc);
                self.registers.write(15, pc + 4);
            }
        } else {
            unreachable!(
                "decoded ARM opcode was out of range. (idx: {}, row: {}, col: {})",
                opcode_idx, opcode_row, opcode_col
            );
        }
    }

    fn step_thumb(&mut self, memory: &mut dyn ArmMemory) {
        let opcode = self.decoded;
        self.decoded = self.fetched;

        let opcode_row = bits!(opcode, 12, 15);
        let opcode_col = bits!(opcode, 8,  11);
        let opcode_idx = (opcode_row * 16) + opcode_col;

        if opcode_idx < 256 {
            let thumb_fn = thumb::THUMB_OPCODE_TABLE[opcode_idx as usize];
            thumb_fn(self, memory, opcode);

            let pc = self.registers.read(15);
            if self.registers.getf_t() {
                self.fetched = memory.load16(pc) as u32;
                self.registers.write(15, pc + 2);
            } else {
                self.fetched = memory.load32(pc);
                self.registers.write(15, pc + 4);
            }
        } else {
            unreachable!(
                "decoded THUMB opcode was out of range. (idx: {}, row: {}, col: {})",
                opcode_idx, opcode_row, opcode_col
           );
        }
    }

    /// Sets the new exception handler. This will return the old exception handler.
    pub fn set_exception_handler(&mut self, on_exception: ExceptionHandler) -> Option<ExceptionHandler> {
        let old_handler = self.on_exception.take();
        self.on_exception = Some(on_exception);
        return old_handler;
    }

    /// Removes this CPU's exception handler and returns it (if there is one).
    pub fn remove_exception_handler(&mut self) -> Option<ExceptionHandler> {
        self.on_exception.take()
    }

    /// Actions performed by CPU when entering an exception
    ///   - R14_<new mode>=PC+nn   ;save old PC, ie. return address
    ///   - SPSR_<new mode>=CPSR   ;save old flags
    ///   - CPSR new T,M bits      ;set to T=0 (ARM state), and M4-0=new mode
    ///   - CPSR new I bit         ;IRQs disabled (I=1), done by ALL exceptions
    ///   - CPSR new F bit         ;FIQs disabled (F=1), done by Reset and FIQ only
    ///   - PC=exception_vector
    pub(crate) fn handle_exception(&mut self, exception: CpuException, memory: &mut dyn ArmMemory) -> bool {
        let exception_info = exception.info();

        if (exception_info.disable & 0b01) != 0 && self.registers.getf_i() { return false; }
        if (exception_info.disable & 0b10) != 0 && self.registers.getf_f() { return false; }

        // we temporarily remove the handler while processing an exception
        // because we don't want possible reentrant into the handler and
        // because it makes the borrow checker happy.
        if let Some(mut handler) = self.on_exception.take() {
            let exception_addr = self.registers.read(15) - (if self.registers.getf_t() { 4 } else { 8 });
            let consumed = handler(self, memory, exception, exception_addr);
            self.on_exception = Some(handler); // RETURN THE SLAB
            if consumed {
                return true;
            }
        }

        let mut pc = self.registers.read(15);
        if self.registers.getf_t() {
            // exception vectors assume an ARM style PC pipelining offset of +8 so we make the
            // THUMB PC which is only off by 4 look like one :P
            pc += 4; 
        }

        self.registers.write(14, pc);                               // Save the return address.
        let cpsr = self.registers.read_cpsr();
        self.registers.write_mode(exception_info.mode_on_entry);    // Set the entry mode.
        self.registers.write_spsr(cpsr);                            // Set the CPSR of the old mode to the SPSR of the new mode.
        self.registers.clearf_t();                                  // Go into ARM mode.

        if let Some(i) = exception_info.i_flag {
            self.registers.putf_i(i);                               // IRQ disable (done by all modes)
        }

        if let Some(f) = exception_info.f_flag {
            self.registers.putf_f(f);                               // FIQ disable (done by RESET and FIQ only)
        }

        let exception_vector = EXCEPTION_BASE + exception_info.offset;
        self.arm_branch_to(exception_vector, memory);               // PC = exception_vector
        self.cycles += clock::cycles_branch_refill(memory, false, exception_vector);
        return true;
    }
}

/// Returns true if an instruction should run based
/// the given condition code and cpsr.
#[inline]
pub(crate) fn check_condition(cond: u32, regs: &ArmRegisters) -> bool {
    match cond {
        // EQ   | Z set                         |   equal
        0b0000 => regs.getf_z(),
        // NE   | Z clear                       | not equal
        0b0001 => !regs.getf_z(),
        // CS   | C set                         | unsigned higher or same
        0b0010 => regs.getf_c(),
        // CC   | C clear                       | unsigned lower
        0b0011 => !regs.getf_c(),
        // MI   | N set                         | negative
        0b0100 => regs.getf_n(),
        // PL   | N clear                       | positive or zero
        0b0101 => !regs.getf_n(),
        // VS   | V set                         | overflow
        0b0110 => regs.getf_v(),
        // VC   | V clear                       | no overflow
        0b0111 => !regs.getf_v(),
        // HI   | C set and Z clear             | unsigned higher
        0b1000 => regs.getf_c() & !regs.getf_z(),
        // LS   | C clear or Z set              | unsigned lower or same
        0b1001 => !regs.getf_c() & regs.getf_z(),
        // GE   | N equals V                    | greater or equal
        0b1010 => regs.getf_n() == regs.getf_v(),
        // LT   | N not equal to V              | less than
        0b1011 => regs.getf_n() != regs.getf_v(),
        // GT   | Z clear AND (N equals V)      | greater than
        0b1100 => !regs.getf_z() & (regs.getf_n() == regs.getf_v()),
        // LE   | Z set OR (N not equal to V)   | less than or equal
        0b1101 => regs.getf_z() || (regs.getf_n() != regs.getf_v()),
        // AL   | (ignored)                     | always
        0b1110 => true,
        _      => panic!("bad condition code: 0x{:08X} ({:04b})", cond, cond),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CpuException {
    Reset,
    Undefined,
    SWI,
    PrefetchAbort,
    DataAbort,
    IRQ,
    FIQ,
    AddressExceeds26Bit,
}

impl CpuException {
    pub fn info(self) -> CpuExceptionInfo {
        match self {
            CpuException::Reset => EXCEPTION_RESET,
            CpuException::Undefined => EXCEPTION_UNDEFINED,
            CpuException::SWI => EXCEPTION_SWI,
            CpuException::PrefetchAbort => EXCEPTION_PREFETCH_ABORT,
            CpuException::DataAbort => EXCEPTION_DATA_ABORT,
            CpuException::IRQ => EXCEPTION_IRQ,
            CpuException::FIQ => EXCEPTION_FIQ,
            CpuException::AddressExceeds26Bit => EXCEPTION_ADDRESS_EXCEEDS_26BIT,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CpuExceptionInfo {
    mode_on_entry: CpuMode,
    i_flag: Option<bool>,
    f_flag: Option<bool>,
    offset: u32,

    /// Disable if 2 bits used as a mask for the I and F flags to check if
    /// a particular exception is disabled. bit 0 = I and bit 1 = F.
    disable: u8,

    /// Because of the way the CPU emulator is designed this doesn't really matter at the moment.
    #[allow(dead_code)]
    priority: u8,
    // TODO(LOW): handle exception priorities as some point.
}

impl CpuExceptionInfo {
    pub const fn new(priority: u8, mode_on_entry: CpuMode, i_flag: Option<bool>, f_flag: Option<bool>, offset: u32, disable: u8) -> CpuExceptionInfo {
        CpuExceptionInfo { priority, mode_on_entry, i_flag, f_flag, offset, disable }
    }
}

// The following are the exception vectors in memory. That is, when an exception arises, CPU is switched into ARM state, and the program counter (PC) is loaded by the respective address.
//   Address  Prio  Exception                  Mode on Entry      Interrupt Flags
//   BASE+00h 1     Reset                      Supervisor (_svc)  I=1, F=1
//   BASE+04h 7     Undefined Instruction      Undefined  (_und)  I=1, F=unchanged
//   BASE+08h 6     Software Interrupt (SWI)   Supervisor (_svc)  I=1, F=unchanged
//   BASE+0Ch 5     Prefetch Abort             Abort      (_abt)  I=1, F=unchanged
//   BASE+10h 2     Data Abort                 Abort      (_abt)  I=1, F=unchanged
//   BASE+14h ??    Address Exceeds 26bit      Supervisor (_svc)  I=1, F=unchanged
//   BASE+18h 4     Normal Interrupt (IRQ)     IRQ        (_irq)  I=1, F=unchanged
//   BASE+1Ch 3     Fast Interrupt (FIQ)       FIQ        (_fiq)  I=1, F=1
pub const EXCEPTION_RESET: CpuExceptionInfo             = CpuExceptionInfo::new(1, CpuMode::Supervisor, Some(true), Some(true), 0x00, 0b00);
pub const EXCEPTION_UNDEFINED: CpuExceptionInfo         = CpuExceptionInfo::new(7, CpuMode::Undefined,  Some(true), None,       0x04, 0b00);
pub const EXCEPTION_SWI: CpuExceptionInfo               = CpuExceptionInfo::new(6, CpuMode::Supervisor, Some(true), None,       0x08, 0b00);
pub const EXCEPTION_PREFETCH_ABORT: CpuExceptionInfo    = CpuExceptionInfo::new(5, CpuMode::Abort,      Some(true), None,       0x0C, 0b00);
pub const EXCEPTION_DATA_ABORT: CpuExceptionInfo        = CpuExceptionInfo::new(2, CpuMode::Abort,      Some(true), None,       0x10, 0b00);
pub const EXCEPTION_IRQ: CpuExceptionInfo               = CpuExceptionInfo::new(4, CpuMode::IRQ,        Some(true), None,       0x18, 0b01);
pub const EXCEPTION_FIQ: CpuExceptionInfo               = CpuExceptionInfo::new(3, CpuMode::FIQ,        Some(true), Some(true), 0x1C, 0b10);

// #TODO I don't actually know the priority for the 26bit address overflow exception.
pub const EXCEPTION_ADDRESS_EXCEEDS_26BIT: CpuExceptionInfo = CpuExceptionInfo::new(8, CpuMode::Supervisor, Some(true), None, 0x14, 0b00);


pub trait ArmMemory {
    // NOTE: reading from certain addresses (e.g. IO) can change their state,
    // so loads still require a mutable `ArmMemory`.
    fn load8(&mut self, addr: u32) -> u8;
    fn store8(&mut self, addr: u32, value: u8);

    fn load16(&mut self, addr: u32) -> u16;
    fn store16(&mut self, addr: u32, value: u16);

    fn load32(&mut self, addr: u32) -> u32;
    fn store32(&mut self, addr: u32, value: u32);

    fn code_access_seq8(&self, addr: u32) -> u32;
    fn data_access_seq8(&self, addr: u32) -> u32;

    fn code_access_nonseq8(&self, addr: u32) -> u32;
    fn data_access_nonseq8(&self, addr: u32) -> u32;

    fn code_access_seq16(&self, addr: u32) -> u32;
    fn data_access_seq16(&self, addr: u32) -> u32;

    fn code_access_nonseq16(&self, addr: u32) -> u32;
    fn data_access_nonseq16(&self, addr: u32) -> u32;

    fn code_access_seq32(&self, addr: u32) -> u32;
    fn data_access_seq32(&self, addr: u32) -> u32;

    fn code_access_nonseq32(&self, addr: u32) -> u32;
    fn data_access_nonseq32(&self, addr: u32) -> u32;
}

pub type ExceptionHandler = Box<dyn FnMut(&mut ArmCpu, &mut dyn ArmMemory, CpuException, u32) -> bool>;
