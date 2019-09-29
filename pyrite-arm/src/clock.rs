// use super::Cpu;
use super::cpu::ArmMemory;

// pub const ILEN_A: u32 = 4;
// pub const ILEN_T: u32 = 2;

/// Returns the number of cycles required for a branch or branch with link instruction.
#[inline(always)]
pub fn cycles_branch(memory: &dyn ArmMemory, thumb: bool, pc: u32, dest: u32) -> u32 {
    if thumb {
        // during the first cycle the branch calculates the destination address
        // while performing a prefetch from the current PC.
        memory.code_access_seq16(pc) +
        // during the second cycle a fetch is performed from the branch destination and the return
        // address is stored in register r14 if the link bit is set.
        memory.code_access_nonseq16(dest) +

        // the third cycle performs a fetch from the destination+L, refilling the instruction
        // pipeline.
        memory.code_access_seq16(dest + 2)
    } else {
        memory.code_access_seq32(pc) +
        memory.code_access_nonseq32(dest) +
        memory.code_access_seq32(dest + 4)
    }
}

/// Returns the number of cycles required for the first instruction in a branch and link
/// instruction in THUMB mode.
#[inline(always)]
pub fn cycles_thumb_bl_setup(memory: &dyn ArmMemory, pc: u32) -> u32 {
    // during the first cycle the branch with link instruction in THUMB mode acts like a single
    // data operation and adds the PC to the upper part of the offset and stores the result in
    // R14(LR).
    memory.code_access_seq16(pc)
}

/// Returns the number of cycles required for the second instruction in a branch and link
/// instruction in THUMB mode.
#[inline(always)]
pub fn cycles_thumb_bl_jump(memory: &dyn ArmMemory, pc: u32, dest: u32) -> u32 {
    // the first cycle of the second instruction is calculates the final branch destination whilst
    // performing a prefetch from the current PC.
    memory.code_access_seq16(pc) +
    // the second and third cycles performs a fetch from the destination address and a prefetch in
    // order to refill the instruction pipeline.
    memory.code_access_nonseq16(dest) +
    memory.code_access_seq16(dest + 2)
}

/// Returns the number of cycles required to do a prefetch from a given location.
#[inline(always)]
pub fn cycles_prefetch(memory: &dyn ArmMemory, thumb: bool, pc: u32) -> u32 {
    if thumb {
        memory.code_access_seq16(pc)
    } else {
        memory.code_access_seq32(pc)
    }
}

/// Returns the number of cycles to refill the instruction pipeline after a branch to
/// a given location.
#[inline(always)]
pub fn cycles_branch_refill(memory: &dyn ArmMemory, thumb: bool, dest: u32) -> u32 {
    if thumb {
        memory.code_access_nonseq16(dest) +
            memory.code_access_seq16(dest + 2)
    } else {
        memory.code_access_nonseq32(dest) +
            memory.code_access_seq32(dest + 4)
    }
}

/// Returns the number of cycles required for a register shift in a data operation.
#[inline(always)]
pub fn cycles_dataop_regshift(_memory: &dyn ArmMemory) -> u32 {
    // #TODO handle gba memory stalls (and prefetch)
    1
}

/// Returns the number of I cycles required to complete a multiply operation
/// using the given multiplier operand.
#[inline(always)]
fn cycles_multiply_m(mut rs: u32, signed: bool) -> u32 {
    // if the most significant bits of rs are set (rs is negative), we use the
    // not of rs so that we can just check if they are zero instead to handle
    // both the positive and negative case.
    if signed && (rs as i32) < 0 { rs = !rs }

   if (rs & 0xFFFFFF00) == 0 {
        // m = 1, if bits [32:8] of the multiplier operand are all zero or all one.
        1
    } else if (rs & 0xFFFF0000) == 0 {
        // m = 2, if bits [32:16] of the multiplier operand are all zero or all one.
        2
    } else if (rs & 0xFF000000) == 0 {
        // m = 3, if bits [32:24] of the multiplier operand are all zero or all one.
        3
    } else {
        // m = 4, in all other cases.
        4
    }
}

/// Returns the number of cycles required for a multiply.
#[inline(always)]
pub fn cycles_multiply(memory: &dyn ArmMemory, thumb: bool, pc: u32, rs: u32, signed: bool) -> u32 {
    // #TODO handle gba memory stalls (and prefetch)
    let m = cycles_multiply_m(rs, signed);
    m + (if thumb { memory.code_access_seq16(pc) } else { memory.code_access_seq32(pc) })
}

/// Returns the number of cycles required for a multiply and accumulate.
#[inline(always)]
pub fn cycles_multiply_acc(memory: &dyn ArmMemory, thumb: bool, pc: u32, rs: u32, signed: bool) -> u32 {
    // #TODO handle gba memory stalls (and prefetch)
    let m = cycles_multiply_m(rs, signed);
    1 + m + (if thumb { memory.code_access_seq16(pc) } else { memory.code_access_seq32(pc) })
}

#[inline(always)]
pub fn cycles_load_register(memory: &dyn ArmMemory, thumb: bool, pc: u32, size: u32, addr: u32) -> u32 {
    // #TODO handle gba memory stalls (and prefetch)
    let prefetch = if thumb {
        memory.code_access_seq16(pc)
    } else {
        memory.code_access_seq32(pc)
    };

    let access = match size {
        32 => memory.data_access_nonseq32(addr),
        16 => memory.data_access_nonseq16(addr),
         8 => memory.data_access_nonseq8(addr),
         _ => unreachable!(),
    };

    return 1 + prefetch + access;
}

// #TODO rename this to cycles_instr_str or something that communicates that this will count the
// cycles for the entire str instruction. Do the same for the cycles_load_register function as
// well.

#[inline(always)]
pub fn cycles_store_register(memory: &dyn ArmMemory, thumb: bool, pc: u32, size: u32, addr: u32) -> u32 {
    // #TODO handle gba memory stalls (and prefetch)
    let prefetch = if thumb {
        memory.code_access_seq16(pc)
    } else {
        memory.code_access_seq32(pc)
    };

    let access = match size {
        32 => memory.data_access_nonseq32(addr),
        16 => memory.data_access_nonseq16(addr),
         8 => memory.data_access_nonseq8(addr),
         _ => unreachable!(),
    };

    return 1 + prefetch + access;
}

#[inline(always)]
pub fn cycles_load_register_pc(memory: &dyn ArmMemory, pc: u32, size: u32, addr: u32, dest_pc: u32) -> u32 {
    // #TODO handle gba memory stalls (and prefetch)
    let prefetch = memory.code_access_seq32(pc);

    let access = match size {
        32 => memory.data_access_nonseq32(addr),
        16 => memory.data_access_nonseq16(addr),
         8 => memory.data_access_nonseq8(addr),
         _ => unreachable!(),
    };

    return 1 + prefetch + access + cycles_branch_refill(memory, false, dest_pc);
}

#[inline(always)]
pub fn internal(_memory: &dyn ArmMemory, internal_cycles: u32) -> u32 {
    // #TODO implement GBA memory stalls (and prefetch)
    // This function won't be doing much until the above is implemented.
    internal_cycles
}
