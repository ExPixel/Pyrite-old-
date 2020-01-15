use super::super::{registers::CpuMode, ArmCpu, ArmMemory};

const LOAD: bool = true;
const STORE: bool = false;

const POST: bool = false;
const PRE: bool = true;

const DEC: bool = false;
const INC: bool = true;

const WRITEBACK: bool = true;
const NO_WRITEBACK: bool = false;

const USER_MODE: bool = true;
const NO_USER_MODE: bool = false;

macro_rules! arm_gen_bdt {
    ($name:ident, $transfer:expr, $transfer_type:expr, $direction:expr, $indexing:expr, $writeback:expr, $s_bit:expr) => {
        pub fn $name(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) -> u32 {
            let mut cycles = cpu.arm_prefetch(memory);

            let register_list = bits!(instr, 0, 15);
            let rn = bits!(instr, 16, 19);
            let reg_count = register_list.count_ones(); // the number of registers being loaded or written to.
            let base = cpu.registers.read(rn);

            // The lowest register always goes into the lowest address
            // so the starting address for writes (-4) gets set up here.
            let mut addr = if $direction == DEC {
                if $indexing == PRE {
                    base.wrapping_sub(reg_count * 4).wrapping_sub(4)
                } else {
                    base.wrapping_sub(reg_count * 4)
                }
            } else {
                if $indexing == POST {
                    base.wrapping_sub(4)
                } else {
                    // no change required for pre-increment addressing.
                    base
                }
            };

            let last_mode = cpu.registers.read_mode();
            // If the S-bit is set for an LDM instruction which doesn't include R15 in the transfer
            // list or an STM instruction, then the registers transferred are taken from the user
            // bank.
            if $s_bit && ($transfer_type == STORE || (register_list & (1 << 15)) == 0) {
                cpu.registers.write_mode(CpuMode::User);
            }

            let mut first = true;
            for reg in 0..16 {
                if (register_list & (1 << reg)) != 0 {
                    addr = addr.wrapping_add(4);
                    cycles += $transfer(cpu, memory, reg, addr, first);

                    if first {
                        first = false;

                        // From Documentation:
                        // The second cycle fetches the first word and performs base modification.
                        // So writeback must happen sometime before the second write and after address
                        // calulation.
                        //
                        // Extended:
                        // When write-back is specified, the base is written back at the end of the
                        // second cycle of the instruction. During a STM, the first register is
                        // written out at the start of the second cycle. A STM which includes
                        // storing the base, with the base as the first register to be stored, will
                        // therefore store the unchanged value, whereas with the base second or
                        // later in the transfer order, will store the modified value. A LDM will
                        // always overwrite the updated base if the base is in teh list.
                        if $writeback {
                            if $transfer_type == STORE || (register_list & (1 << rn)) == 0 {
                                let writeback_addr = if $direction == DEC {
                                    base.wrapping_sub(reg_count * 4)
                                } else {
                                    base.wrapping_add(reg_count * 4)
                                };
                                cpu.registers.write(rn, writeback_addr);
                            }
                        }
                    }
                }
            }

            if $s_bit && $transfer_type == LOAD && (register_list & (1 << 15)) != 0 {
                // if the S-bit is set in an LDM instruction and R15 is in the transfer list
                // then SPSR_<mode> is transferred to CPSR at the same time as R15 is loaded (the end
                // of the transfer).
                let spsr = cpu.registers.read_spsr();
                cpu.registers.write_cpsr(spsr);
            }

            if $s_bit && ($transfer_type == STORE || (register_list & (1 << 15)) == 0) {
                // Here, we just switch back to whatever mode we were in before the user mode
                // switch.
                cpu.registers.write_mode(last_mode);
            }

            if $transfer_type == LOAD {
                // This final internal cycle is for moving the last word into its destination
                // register.
                //
                // #TODO The ARM7TDMI documentation also mentions that this can be merged with the
                // next prefetch cycle as well to create one N cycle, but I'm not sure if the GBA does
                // that or not.
                cycles += 1;
                memory.on_internal_cycles(1);

                if (register_list & (1 << 15)) != 0 {
                    let dest_pc = cpu.registers.read(15);
                    if cpu.registers.getf_t() {
                        cycles += cpu.thumb_branch_to(dest_pc, memory);
                    } else {
                        cycles += cpu.arm_branch_to(dest_pc, memory);
                    }
                }
            }

            return cycles;
        }
    };
}

#[must_use]
#[inline(always)]
fn store_word(
    cpu: &mut ArmCpu,
    memory: &mut dyn ArmMemory,
    reg: u32,
    addr: u32,
    first_transfer: bool,
) -> u32 {
    let mut cycles = 0;
    let mut value = cpu.registers.read(reg);
    // Whenever R15 is stored to memory the stored value is the address of the STM instruction plus 12.
    if reg == 15 {
        value += 4;
    }
    memory.write_data_word(addr, value, !first_transfer, &mut cycles);
    return cycles;
}

#[must_use]
#[inline(always)]
fn load_word(
    cpu: &mut ArmCpu,
    memory: &mut dyn ArmMemory,
    reg: u32,
    addr: u32,
    first_transfer: bool,
) -> u32 {
    let mut cycles = 0;
    let value = memory.read_data_word(addr, !first_transfer, &mut cycles);
    cpu.registers.write(reg, value);
    return cycles;
}

// Load multiple words, decrement after
arm_gen_bdt!(
    arm_ldmda,
    load_word,
    LOAD,
    DEC,
    POST,
    NO_WRITEBACK,
    NO_USER_MODE
);

// Load multiple words, decrement after, Use user-mode registers
arm_gen_bdt!(
    arm_ldmda_u,
    load_word,
    LOAD,
    DEC,
    POST,
    NO_WRITEBACK,
    USER_MODE
);

// Load multiple words, decrement after, Use user-mode registers, with write back
arm_gen_bdt!(
    arm_ldmda_uw,
    load_word,
    LOAD,
    DEC,
    POST,
    WRITEBACK,
    USER_MODE
);

// Load multiple words, decrement after, Write back
arm_gen_bdt!(
    arm_ldmda_w,
    load_word,
    LOAD,
    DEC,
    POST,
    WRITEBACK,
    NO_USER_MODE
);

// Load multiple words, decrement before
arm_gen_bdt!(
    arm_ldmdb,
    load_word,
    LOAD,
    DEC,
    PRE,
    NO_WRITEBACK,
    NO_USER_MODE
);

// Load multiple words, decrement before, Use user-mode registers
arm_gen_bdt!(
    arm_ldmdb_u,
    load_word,
    LOAD,
    DEC,
    PRE,
    NO_WRITEBACK,
    USER_MODE
);

// Load multiple words, decrement before, Use user-mode registers, with write back
arm_gen_bdt!(
    arm_ldmdb_uw,
    load_word,
    LOAD,
    DEC,
    PRE,
    WRITEBACK,
    USER_MODE
);

// Load multiple words, decrement before, Write back
arm_gen_bdt!(
    arm_ldmdb_w,
    load_word,
    LOAD,
    DEC,
    PRE,
    WRITEBACK,
    NO_USER_MODE
);

// Load multiple words, increment after
arm_gen_bdt!(
    arm_ldmia,
    load_word,
    LOAD,
    INC,
    POST,
    NO_WRITEBACK,
    NO_USER_MODE
);

// Load multiple words, increment after, Use user-mode registers
arm_gen_bdt!(
    arm_ldmia_u,
    load_word,
    LOAD,
    INC,
    POST,
    NO_WRITEBACK,
    USER_MODE
);

// Load multiple words, increment after, Use user-mode registers, with write back
arm_gen_bdt!(
    arm_ldmia_uw,
    load_word,
    LOAD,
    INC,
    POST,
    WRITEBACK,
    USER_MODE
);

// Load multiple words, increment after, Write back
arm_gen_bdt!(
    arm_ldmia_w,
    load_word,
    LOAD,
    INC,
    POST,
    WRITEBACK,
    NO_USER_MODE
);

// Load multiple words, increment before
arm_gen_bdt!(
    arm_ldmib,
    load_word,
    LOAD,
    INC,
    PRE,
    NO_WRITEBACK,
    NO_USER_MODE
);

// Load multiple words, increment before, Use user-mode registers
arm_gen_bdt!(
    arm_ldmib_u,
    load_word,
    LOAD,
    INC,
    PRE,
    NO_WRITEBACK,
    USER_MODE
);

// Load multiple words, increment before, Use user-mode registers, with write back
arm_gen_bdt!(
    arm_ldmib_uw,
    load_word,
    LOAD,
    INC,
    PRE,
    WRITEBACK,
    USER_MODE
);

// Load multiple words, increment before, Write back
arm_gen_bdt!(
    arm_ldmib_w,
    load_word,
    LOAD,
    INC,
    PRE,
    WRITEBACK,
    NO_USER_MODE
);

// Store multiple words, decrement after
arm_gen_bdt!(
    arm_stmda,
    store_word,
    STORE,
    DEC,
    POST,
    NO_WRITEBACK,
    NO_USER_MODE
);

// Store multiple words, decrement after, Use user-mode registers
arm_gen_bdt!(
    arm_stmda_u,
    store_word,
    STORE,
    DEC,
    POST,
    NO_WRITEBACK,
    USER_MODE
);

// Store multiple words, decrement after, Use user-mode registers, with write back
arm_gen_bdt!(
    arm_stmda_uw,
    store_word,
    STORE,
    DEC,
    POST,
    WRITEBACK,
    USER_MODE
);

// Store multiple words, decrement after, Write back
arm_gen_bdt!(
    arm_stmda_w,
    store_word,
    STORE,
    DEC,
    POST,
    WRITEBACK,
    NO_USER_MODE
);

// Store multiple words, decrement before
arm_gen_bdt!(
    arm_stmdb,
    store_word,
    STORE,
    DEC,
    PRE,
    NO_WRITEBACK,
    NO_USER_MODE
);

// Store multiple words, decrement before, Use user-mode registers
arm_gen_bdt!(
    arm_stmdb_u,
    store_word,
    STORE,
    DEC,
    PRE,
    NO_WRITEBACK,
    USER_MODE
);

// Store multiple words, decrement before, Use user-mode registers, with write back
arm_gen_bdt!(
    arm_stmdb_uw,
    store_word,
    STORE,
    DEC,
    PRE,
    WRITEBACK,
    USER_MODE
);

// Store multiple words, decrement before, Write back
arm_gen_bdt!(
    arm_stmdb_w,
    store_word,
    STORE,
    DEC,
    PRE,
    WRITEBACK,
    NO_USER_MODE
);

// Store multiple words, increment after
arm_gen_bdt!(
    arm_stmia,
    store_word,
    STORE,
    INC,
    POST,
    NO_WRITEBACK,
    NO_USER_MODE
);

// Store multiple words, increment after, Use user-mode registers
arm_gen_bdt!(
    arm_stmia_u,
    store_word,
    STORE,
    INC,
    POST,
    NO_WRITEBACK,
    USER_MODE
);

// Store multiple words, increment after, Use user-mode registers, with write back
arm_gen_bdt!(
    arm_stmia_uw,
    store_word,
    STORE,
    INC,
    POST,
    WRITEBACK,
    USER_MODE
);

// Store multiple words, increment after, Write back
arm_gen_bdt!(
    arm_stmia_w,
    store_word,
    STORE,
    INC,
    POST,
    WRITEBACK,
    NO_USER_MODE
);

// Store multiple words, increment before
arm_gen_bdt!(
    arm_stmib,
    store_word,
    STORE,
    INC,
    PRE,
    NO_WRITEBACK,
    NO_USER_MODE
);

// Store multiple words, increment before, Use user-mode registers
arm_gen_bdt!(
    arm_stmib_u,
    store_word,
    STORE,
    INC,
    PRE,
    NO_WRITEBACK,
    USER_MODE
);

// Store multiple words, increment before, Use user-mode registers, with write back
arm_gen_bdt!(
    arm_stmib_uw,
    store_word,
    STORE,
    INC,
    PRE,
    WRITEBACK,
    USER_MODE
);

// Store multiple words, increment before, Write back
arm_gen_bdt!(
    arm_stmib_w,
    store_word,
    STORE,
    INC,
    PRE,
    WRITEBACK,
    NO_USER_MODE
);
