use super::{ arm, thumb };
use super::registers::{ CpuMode, ArmRegisters };
use super::memory::ArmMemory;

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

    /// Exception that should be handled on the next call to step instead of running the next
    /// instruction.
    pending_exception: Option<CpuException>,

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
            pending_exception: None,
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
        let next_pc = pc.wrapping_add(4);
        self.registers.write(15, next_pc);
        self.decoded = memory.read_code_word(pc, false, &mut self.cycles);
        self.fetched = memory.read_code_word(next_pc, true, &mut self.cycles);
    }

    /// See `arm_branch_to`
    #[inline]
    pub(crate) fn thumb_branch_to(&mut self, pc: u32, memory: &mut dyn ArmMemory) {
        let next_pc = pc.wrapping_add(2);
        self.registers.write(15, next_pc);
        self.decoded = memory.read_code_halfword(pc, false, &mut self.cycles) as u32;
        self.fetched = memory.read_code_halfword(next_pc, true, &mut self.cycles) as u32;
    }

    /// Like `arm_branch_to` and `thumb_branch_to` but this will select which one to call for you
    /// based on the current mode and will also automatically align.
    #[inline]
    pub(crate) fn branch_to(&mut self, pc: u32, memory: &mut dyn ArmMemory) {
        if self.registers.getf_t() {
            self.thumb_branch_to(pc & 0xFFFFFFFE, memory);
        } else {
            self.arm_branch_to(pc & 0xFFFFFFFC, memory);
        }
    }

    /// Flushes the CPU's pipeline, sets the program counter
    /// and "fetches" and "decodes" the next instruction.
    pub fn set_pc(&mut self, value: u32, memory: &mut dyn ArmMemory) {
        if self.registers.getf_t() {
            self.thumb_set_pc(value, memory);
        } else {
            self.arm_set_pc(value, memory);
        }
    }

    // @TODO remove this
    #[inline]
    fn arm_set_pc(&mut self, value: u32, memory: &mut dyn ArmMemory) {
        self.arm_branch_to(value, memory);
    }

    // @TODO remove this
    #[inline]
    fn thumb_set_pc(&mut self, value: u32, memory: &mut dyn ArmMemory) {
        self.thumb_branch_to(value, memory);
    }

    pub fn arm_prefetch(&mut self, memory: &mut dyn ArmMemory) {
        let next_pc = self.registers.read(15).wrapping_add(4);
        self.registers.write(15, next_pc);
        self.fetched = memory.read_code_word(next_pc, true, &mut self.cycles);
    }

    /// Gets the cycles for a prefetch without actually fetching the memory.
    /// Used by branch instructions so that we aren't doing useless reads.
    #[inline(always)]
    pub fn arm_prefetch_cycles(&mut self, memory: &mut dyn ArmMemory) {
        let next_pc = self.registers.read(15).wrapping_add(4);
        self.registers.write(15, next_pc);
        self.cycles += memory.code_cycles_word(next_pc, true);
    }

    pub fn thumb_prefetch(&mut self, memory: &mut dyn ArmMemory) {
        let next_pc = self.registers.read(15).wrapping_add(2);
        self.registers.write(15, next_pc);
        self.fetched = memory.read_code_halfword(next_pc, true, &mut self.cycles) as u32;
    }

    /// Gets the cycles for a prefetch without actually fetching the memory.
    /// Used by branch instructions so that we aren't doing useless reads.
    #[inline(always)]
    pub fn thumb_prefetch_cycles(&mut self, memory: &mut dyn ArmMemory) {
        let next_pc = self.registers.read(15).wrapping_add(2);
        self.registers.write(15, next_pc);
        self.cycles += memory.code_cycles_halfword(next_pc, true);
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
    #[inline]
    pub fn step(&mut self, memory: &mut dyn ArmMemory) -> u32 {
        self.cycles = 0;
        if let Some(exception) = self.pending_exception.take() {
            let instr_size = if self.registers.getf_t() { 2 } else { 4 };
            self.handle_exception(exception, memory, self.registers.read(15).wrapping_sub(instr_size));
        } else {
            if self.registers.getf_t() {
                self.step_thumb(memory);
            } else {
                self.step_arm(memory);
            }
        }
        self.total_cycles += self.cycles as u64;
        return self.cycles;
    }

    pub fn set_pending_exception(&mut self, exception: CpuException) {
        if let Some(current) = self.pending_exception {
            if exception.info().priority >= current.info().priority {
                return;
            }
        }
        self.pending_exception = Some(exception);
    }

    #[inline]
    fn step_arm(&mut self, memory: &mut dyn ArmMemory) {
        let opcode = self.decoded;
        self.decoded = self.fetched;

        let opcode_row = bits!(opcode, 20, 27);
        let opcode_col = bits!(opcode,  4,  7);
        let opcode_idx = (opcode_row * 16) + opcode_col;

        if check_condition((opcode >> 28) & 0xF, &self.registers) {
            let arm_fn = arm::ARM_OPCODE_TABLE[opcode_idx as usize];
            arm_fn(self, memory, opcode);
        } else {
            self.arm_prefetch(memory);
        }
    }

    #[inline]
    fn step_thumb(&mut self, memory: &mut dyn ArmMemory) {
        let opcode = self.decoded;
        self.decoded = self.fetched;

        let opcode_row = bits!(opcode, 12, 15);
        let opcode_col = bits!(opcode, 8,  11);
        let opcode_idx = (opcode_row * 16) + opcode_col;

        let thumb_fn = thumb::THUMB_OPCODE_TABLE[opcode_idx as usize];
        thumb_fn(self, memory, opcode);
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
    pub(crate) fn handle_exception(&mut self, exception: CpuException, memory: &mut dyn ArmMemory, next_instr_address: u32) -> bool {
        let exception_info = exception.info();

        if (exception_info.disable & 0b01) != 0 && self.registers.getf_i() { return false; }
        if (exception_info.disable & 0b10) != 0 && self.registers.getf_f() { return false; }

        let exception_addr = next_instr_address.wrapping_sub(if self.registers.getf_t() { 2 } else { 4 });

        // we temporarily remove the handler while processing an exception
        // because we don't want possible reentrant into the handler and
        // because it makes the borrow checker happy.
        if let Some(mut handler) = self.on_exception.take() {
            let consumed = handler(self, memory, exception, exception_addr);
            self.on_exception = Some(handler); // RETURN THE SLAB
            if consumed {
                return true;
            }
        }

        let cpsr = self.registers.read_cpsr();
        self.registers.write_mode(exception_info.mode_on_entry);    // Set the entry mode.
        self.registers.write_spsr(cpsr);                            // Set the CPSR of the old mode to the SPSR of the new mode.
        self.registers.write(14, next_instr_address.wrapping_add(exception_info.pc_adjust)); // Save the return address.
        self.registers.clearf_t();                                  // Go into ARM mode.

        self.registers.putf_i(true);                               // IRQ disable (done by all modes)

        if let Some(f) = exception_info.f_flag {
            self.registers.putf_f(f);                               // FIQ disable (done by RESET and FIQ only)
        }

        let exception_vector = EXCEPTION_BASE + exception_info.offset;
        self.arm_branch_to(exception_vector, memory);               // PC = exception_vector
        return true;
    }
}

/// Returns true if an instruction should run based
/// the given condition code and cpsr.
#[inline]
pub(crate) fn check_condition(cond: u32, regs: &ArmRegisters) -> bool {
    match cond {
        0x0 => regs.getf_z(),                   // 0:   EQ     Z=1           equal (zero) (same)
        0x1 => !regs.getf_z(),                  // 1:   NE     Z=0           not equal (nonzero) (not same)
        0x2 => regs.getf_c(),                   // 2:   CS/HS  C=1           unsigned higher or same (carry set)
        0x3 => !regs.getf_c(),                  // 3:   CC/LO  C=0           unsigned lower (carry cleared)
        0x4 => regs.getf_n(),                   // 4:   MI     N=1           negative (minus)
        0x5 => !regs.getf_n(),                  // 5:   PL     N=0           positive or zero (plus)
        0x6 => regs.getf_v(),                   // 6:   VS     V=1           overflow (V set)
        0x7 => !regs.getf_v(),                  // 7:   VC     V=0           no overflow (V cleared)
        0x8 => regs.getf_c() & !regs.getf_z(),  // 8:   HI     C=1 and Z=0   unsigned higher
        0x9 => !regs.getf_c() | regs.getf_z(),  // 9:   LS     C=0 or Z=1    unsigned lower or same
        0xA => regs.getf_n() == regs.getf_v(),  // A:   GE     N=V           greater or equal
        0xB => regs.getf_n() != regs.getf_v(),  // B:   LT     N<>V          less than
        0xC => {                                // C:   GT     Z=0 and N=V   greater than
            !regs.getf_z() & (regs.getf_n() == regs.getf_v())
        },   
        0xD => {                                // D:   LE     Z=1 or N<>V   less or equal
            regs.getf_z() | (regs.getf_n() != regs.getf_v())
        },    
        0xE => true,                            // E:   AL     -             always (the "AL" suffix can be omitted)
        0xF => false,                           // F:   NV     -             never (ARMv1,v2 only) (Reserved ARMv3 and up)
        // :(
        _   => panic!("bad condition code: 0x{:08X} ({:04b})", cond, cond),
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
    pub fn name(self) -> &'static str {
        match self {
            CpuException::Reset => "Reset",
            CpuException::Undefined => "Undefined",
            CpuException::SWI => "SWI",
            CpuException::PrefetchAbort => "Prefetch Abort",
            CpuException::DataAbort => "Data Abort",
            CpuException::IRQ => "IRQ",
            CpuException::FIQ => "FIQ",
            CpuException::AddressExceeds26Bit => "Address Exceeds 26 bit",
        }
    }

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
    f_flag: Option<bool>,
    pc_adjust:  u32,
    offset:     u32,

    /// Disable if 2 bits used as a mask for the I and F flags to check if
    /// a particular exception is disabled. bit 0 = I and bit 1 = F.
    disable: u8,

    /// Lower number means higher priority.
    priority: u8,
}

impl CpuExceptionInfo {
    pub const fn new(priority: u8, mode_on_entry: CpuMode, f_flag: Option<bool>, pc_adjust: u32, offset: u32, disable: u8) -> CpuExceptionInfo {
        CpuExceptionInfo { priority, mode_on_entry, f_flag, pc_adjust, offset, disable }
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
pub const EXCEPTION_RESET: CpuExceptionInfo             = CpuExceptionInfo::new(1, CpuMode::Supervisor, Some(true), 0, 0x00, 0b00);
pub const EXCEPTION_UNDEFINED: CpuExceptionInfo         = CpuExceptionInfo::new(7, CpuMode::Undefined,  None,       0, 0x04, 0b00);
pub const EXCEPTION_SWI: CpuExceptionInfo               = CpuExceptionInfo::new(6, CpuMode::Supervisor, None,       0, 0x08, 0b00);
pub const EXCEPTION_PREFETCH_ABORT: CpuExceptionInfo    = CpuExceptionInfo::new(5, CpuMode::Abort,      None,       4, 0x0C, 0b00);
pub const EXCEPTION_DATA_ABORT: CpuExceptionInfo        = CpuExceptionInfo::new(2, CpuMode::Abort,      None,       4, 0x10, 0b00);
pub const EXCEPTION_IRQ: CpuExceptionInfo               = CpuExceptionInfo::new(4, CpuMode::IRQ,        None,       4, 0x18, 0b01);
pub const EXCEPTION_FIQ: CpuExceptionInfo               = CpuExceptionInfo::new(3, CpuMode::FIQ,        Some(true), 4, 0x1C, 0b10);

// #TODO I don't actually know the priority for the 26bit address overflow exception.
pub const EXCEPTION_ADDRESS_EXCEEDS_26BIT: CpuExceptionInfo = CpuExceptionInfo::new(8, CpuMode::Supervisor, None, 4, 0x14, 0b00);

pub type ExceptionHandler = Box<dyn FnMut(&mut ArmCpu, &mut dyn ArmMemory, CpuException, u32) -> bool>;
