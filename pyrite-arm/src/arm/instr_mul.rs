use super::super::{ ArmCpu, ArmMemory };
use super::super::alu::{ set_nz_flags, set_nz_flags64, internal_multiply_cycles };

#[inline]
fn get_mulinstr_regs(instr: u32) -> (u32, u32, u32, u32) {
    let rm = bits!(instr,  0,  3);
    let rs = bits!(instr,  8, 11);
    let rn = bits!(instr, 12, 15);
    let rd = bits!(instr, 16, 19);
    (rm, rs, rn, rd)
}

#[inline]
fn get_long_mulinstr_regs(instr: u32) -> (u32, u32, u32, u32) {
    let rm = bits!(instr, 0, 3);
    let rs = bits!(instr, 8, 11);
    let rd_lo = bits!(instr, 12, 15);
    let rd_hi = bits!(instr, 16, 19);
    (rm, rs, rd_lo, rd_hi)
}

/// Multiply and accumulate registers
pub fn arm_mla(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, rn, rd) = get_mulinstr_regs(instr);
    let lhs = cpu.registers.read(rm);
    let rhs = cpu.registers.read(rs);
    let acc = cpu.registers.read(rn);
    let res = lhs.wrapping_mul(rhs).wrapping_add(acc);
    cpu.registers.write(rd, res);

    let icycles = 1 + internal_multiply_cycles(rhs, false);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Multiply and accumulate registers, setting flags
pub fn arm_mlas(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, rn, rd) = get_mulinstr_regs(instr);
    let lhs = cpu.registers.read(rm);
    let rhs = cpu.registers.read(rs);
    let acc = cpu.registers.read(rn);
    let res = lhs.wrapping_mul(rhs).wrapping_add(acc);
    cpu.registers.write(rd, res);
    set_nz_flags(cpu, res);

    let icycles = 1 + internal_multiply_cycles(rhs, false);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Multiply registers
pub fn arm_mul(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, _rn, rd) = get_mulinstr_regs(instr);
    let lhs = cpu.registers.read(rm);
    let rhs = cpu.registers.read(rs);
    let res = lhs.wrapping_mul(rhs);
    cpu.registers.write(rd, res);

    let icycles = internal_multiply_cycles(rhs, false);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Multiply registers, setting flags
pub fn arm_muls(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, _rn, rd) = get_mulinstr_regs(instr);
    let lhs = cpu.registers.read(rm);
    let rhs = cpu.registers.read(rs);
    let res = lhs.wrapping_mul(rhs);
    cpu.registers.write(rd, res);
    set_nz_flags(cpu, res);

    let icycles = internal_multiply_cycles(rhs, false);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Signed long multiply and accumulate
pub fn arm_smlal(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, rd_lo, rd_hi) = get_long_mulinstr_regs(instr);
    let rsv = cpu.registers.read(rs);
    let lhs = cpu.registers.read(rm) as i32 as i64; // sign-extended
    let rhs = rsv as i32 as i64; // sign-extended
    let alo = cpu.registers.read(rd_lo) as u32 as i64; // zero-extended
    let ahi = cpu.registers.read(rd_hi) as u32 as i64; // zero-extended
    let acc = (ahi << 32) | alo;
    let res = lhs.wrapping_mul(rhs).wrapping_add(acc);
    let res_lo = (res & 0xFFFFFFFF) as u32;
    let res_hi = ((res >> 32) & 0xFFFFFFFF) as u32;
    cpu.registers.write(rd_lo, res_lo);
    cpu.registers.write(rd_hi, res_hi);

    let icycles = 1 + internal_multiply_cycles(rsv, true);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Signed long multiply and accumulate, setting flags
pub fn arm_smlals(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, rd_lo, rd_hi) = get_long_mulinstr_regs(instr);
    let rsv = cpu.registers.read(rs);
    let lhs = cpu.registers.read(rm) as i32 as i64; // sign-extended
    let rhs = rsv as i32 as i64; // sign-extended
    let alo = cpu.registers.read(rd_lo) as u32 as i64; // zero-extended
    let ahi = cpu.registers.read(rd_hi) as u32 as i64; // zero-extended
    let acc = (ahi << 32) | alo;
    let res = lhs.wrapping_mul(rhs).wrapping_add(acc);
    let res_lo = (res & 0xFFFFFFFF) as u32;
    let res_hi = ((res >> 32) & 0xFFFFFFFF) as u32;
    cpu.registers.write(rd_lo, res_lo);
    cpu.registers.write(rd_hi, res_hi);
    set_nz_flags64(cpu, res as u64);

    let icycles = 1 + internal_multiply_cycles(rsv, true);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Signed long multiply (32x32 to 64)
pub fn arm_smull(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, rd_lo, rd_hi) = get_long_mulinstr_regs(instr);
    let rsv = cpu.registers.read(rs);
    let lhs = cpu.registers.read(rm) as i32 as i64;
    let rhs = rsv as i32 as i64;
    let res = lhs.wrapping_mul(rhs);
    let res_lo = (res & 0xFFFFFFFF) as u32;
    let res_hi = ((res >> 32) & 0xFFFFFFFF) as u32;
    cpu.registers.write(rd_lo, res_lo);
    cpu.registers.write(rd_hi, res_hi);

    let icycles = internal_multiply_cycles(rsv, true);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Signed long multiply, setting flags
pub fn arm_smulls(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, rd_lo, rd_hi) = get_long_mulinstr_regs(instr);
    let rsv = cpu.registers.read(rs);
    let lhs = cpu.registers.read(rm) as i32 as i64;
    let rhs = rsv as i32 as i64;
    let res = lhs.wrapping_mul(rhs);
    let res_lo = (res & 0xFFFFFFFF) as u32;
    let res_hi = ((res >> 32) & 0xFFFFFFFF) as u32;
    cpu.registers.write(rd_lo, res_lo);
    cpu.registers.write(rd_hi, res_hi);
    set_nz_flags64(cpu, res as u64);

    let icycles = internal_multiply_cycles(rsv, true);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Unsigned long multiply and accumulate
pub fn arm_umlal(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, rd_lo, rd_hi) = get_long_mulinstr_regs(instr);
    let rsv = cpu.registers.read(rs);
    let lhs = cpu.registers.read(rm) as u64;
    let rhs = rsv as u64;
    let alo = cpu.registers.read(rd_lo) as u64;
    let ahi = cpu.registers.read(rd_hi) as u64;
    let acc = (ahi << 32) | alo;
    let res = lhs.wrapping_mul(rhs).wrapping_add(acc);
    let res_lo = (res & 0xFFFFFFFF) as u32;
    let res_hi = ((res >> 32) & 0xFFFFFFFF) as u32;
    cpu.registers.write(rd_lo, res_lo);
    cpu.registers.write(rd_hi, res_hi);

    let icycles = 1 + internal_multiply_cycles(rsv, false);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Unsigned long multiply and accumulate, setting flags
pub fn arm_umlals(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, rd_lo, rd_hi) = get_long_mulinstr_regs(instr);
    let rsv = cpu.registers.read(rs);
    let lhs = cpu.registers.read(rm) as u64;
    let rhs = rsv as u64;
    let alo = cpu.registers.read(rd_lo) as u64;
    let ahi = cpu.registers.read(rd_hi) as u64;
    let acc = (ahi << 32) | alo;
    let res = lhs.wrapping_mul(rhs).wrapping_add(acc);
    let res_lo = (res & 0xFFFFFFFF) as u32;
    let res_hi = ((res >> 32) & 0xFFFFFFFF) as u32;
    cpu.registers.write(rd_lo, res_lo);
    cpu.registers.write(rd_hi, res_hi);
    set_nz_flags64(cpu, res);

    let icycles = 1 + internal_multiply_cycles(rsv, false);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Unsigned long multiply (32x32 to 64)
pub fn arm_umull(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, rd_lo, rd_hi) = get_long_mulinstr_regs(instr);
    let rsv = cpu.registers.read(rs);
    let lhs = cpu.registers.read(rm) as u64;
    let rhs = rsv as u64;
    let res = lhs.wrapping_mul(rhs);
    let res_lo = (res & 0xFFFFFFFF) as u32;
    let res_hi = ((res >> 32) & 0xFFFFFFFF) as u32;
    cpu.registers.write(rd_lo, res_lo);
    cpu.registers.write(rd_hi, res_hi);

    let icycles = internal_multiply_cycles(rsv, false);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

/// Unsigned long multiply, setting flags
pub fn arm_umulls(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
    cpu.arm_prefetch(memory);

    let (rm, rs, rd_lo, rd_hi) = get_long_mulinstr_regs(instr);
    let rsv = cpu.registers.read(rs);
    let lhs = cpu.registers.read(rm) as u64;
    let rhs = rsv as u64;
    let res = lhs.wrapping_mul(rhs);
    let res_lo = (res & 0xFFFFFFFF) as u32;
    let res_hi = ((res >> 32) & 0xFFFFFFFF) as u32;
    cpu.registers.write(rd_lo, res_lo);
    cpu.registers.write(rd_hi, res_hi);
    set_nz_flags64(cpu, res);

    let icycles = internal_multiply_cycles(rsv, false);
    cpu.cycles += icycles;
    memory.on_internal_cycles(icycles);
}

