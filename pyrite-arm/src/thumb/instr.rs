use super::super::{ ArmCpu, ArmMemory };
use super::super::alu;
use super::super::clock;

#[inline(always)]
fn sdt_ldr(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, rd: u32, addr: u32) {
    // From the ARM7TDMI Documentation:
    //      A word load will normally use a word aligned address, however,
    //      an address offset from the word boundary will cause the data to
    //      be rotated into the register so that the addressed byte occupies bit 0-7.
    // From GBATek:
    //      Reads from forcibly aligned address "addr AND (NOT 3)", and then rotate
    //      the data as "ROR (addr AND 3)*8"
    let value = memory.load32(addr & 0xFFFFFFFC).rotate_right(8 * (addr % 4));
    cpu.registers.write(rd, value);
}

#[inline(always)]
fn sdt_str(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, rd: u32, addr: u32) {
    let value = cpu.registers.read(rd);

    // @NOTE commented out this part from the ARM code because PC shouldn't be reachable
    //       from the instructions that will use this :P
    // If the Program Counter is used as the source register in a word store, it will be 12 bytes
    // ahead instead of 8 when read.
    // if rd == 15 { value = value.wrapping_add(4); }

    memory.store32(addr & 0xFFFFFFFC, value);
}

macro_rules! impl_move_shifted_register {
    ($name:ident, $op:expr) => {
        pub fn $name(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
            let rd = bits!(opcode, 0, 2);
            let rs = bits!(opcode, 3, 5);
            let lhs = cpu.registers.read(rs);
            let rhs = bits!(opcode, 6, 10);
            let res = $op(cpu, lhs, rhs);
            cpu.registers.write(rd, res);

            // clock the instruction prefetch
            cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
        }
    }
}

// Shift Rs left by a 5-bit immediate value and store the result in Rd.
impl_move_shifted_register!(thumb_lsl_imm, alu::arm_alu_lli_s);
// Perform logical shift right on Rs by a 5- bit immediate value and store the result in Rd.
impl_move_shifted_register!(thumb_lsr_imm, alu::arm_alu_lri_s);
// Perform arithmetic shift right on Rs by a 5-bit immediate value and store the result in Rd.
impl_move_shifted_register!(thumb_asr_imm, alu::arm_alu_ari_s);

/// Add contents of Rn to contents of Rs. Place result in Rd.
pub fn thumb_add_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);
    let rn = bits!(opcode, 6, 8);

    let lhs = cpu.registers.read(rs);
    let rhs = cpu.registers.read(rn);
    let res = alu::arm_alu_adds(cpu, lhs, rhs);

    cpu.registers.write(rd, res);

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
}

/// Subtract contents of Rn from contents of Rs. Place result in Rd.
pub fn thumb_sub_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);
    let rn = bits!(opcode, 6, 8);

    let lhs = cpu.registers.read(rs);
    let rhs = cpu.registers.read(rn);
    let res = alu::arm_alu_subs(cpu, lhs, rhs);

    cpu.registers.write(rd, res);

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
}

/// Add 3-bit immediate value to contents of Rs. Place result in Rd.
pub fn thumb_add_imm3(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);

    let lhs = cpu.registers.read(rs);
    let rhs = bits!(opcode, 6, 8);
    let res = alu::arm_alu_adds(cpu, lhs, rhs);

    cpu.registers.write(rd, res);

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
}

/// Subtract 3-bit immediate value from contents of Rs. Place result in Rd.
pub fn thumb_sub_imm3(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);

    let lhs = cpu.registers.read(rs);
    let rhs = bits!(opcode, 6, 8);
    let res = alu::arm_alu_subs(cpu, lhs, rhs);

    cpu.registers.write(rd, res);

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
}

macro_rules! mov_compare_add_subtract_imm {
    ($name:ident, $op:expr, $rd:expr) => {
        pub fn $name(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
            let rd = $rd;
            let lhs = cpu.registers.read(rd);
            let rhs = opcode & 0xFF;
            let res = $op(cpu, lhs, rhs);
            cpu.registers.write(rd, res);

            // clock the instruction prefetch
            cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
        }
    };

    ($name:ident, $op:expr, $rd:expr, $no_write:ident) => {
        pub fn $name(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
            let rd = $rd;
            let lhs = cpu.registers.read(rd);
            let rhs = opcode & 0xFF;
            $op(cpu, lhs, rhs);

            // clock the instruction prefetch
            cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
        }
    };
}

mov_compare_add_subtract_imm!(thumb_mov_i8_r0, alu::arm_alu_movs, 0);
mov_compare_add_subtract_imm!(thumb_mov_i8_r1, alu::arm_alu_movs, 1);
mov_compare_add_subtract_imm!(thumb_mov_i8_r2, alu::arm_alu_movs, 2);
mov_compare_add_subtract_imm!(thumb_mov_i8_r3, alu::arm_alu_movs, 3);
mov_compare_add_subtract_imm!(thumb_mov_i8_r4, alu::arm_alu_movs, 4);
mov_compare_add_subtract_imm!(thumb_mov_i8_r5, alu::arm_alu_movs, 5);
mov_compare_add_subtract_imm!(thumb_mov_i8_r6, alu::arm_alu_movs, 6);
mov_compare_add_subtract_imm!(thumb_mov_i8_r7, alu::arm_alu_movs, 7);

mov_compare_add_subtract_imm!(thumb_cmp_i8_r0, alu::arm_alu_cmps, 0, NO_WRITE);
mov_compare_add_subtract_imm!(thumb_cmp_i8_r1, alu::arm_alu_cmps, 1, NO_WRITE);
mov_compare_add_subtract_imm!(thumb_cmp_i8_r2, alu::arm_alu_cmps, 2, NO_WRITE);
mov_compare_add_subtract_imm!(thumb_cmp_i8_r3, alu::arm_alu_cmps, 3, NO_WRITE);
mov_compare_add_subtract_imm!(thumb_cmp_i8_r4, alu::arm_alu_cmps, 4, NO_WRITE);
mov_compare_add_subtract_imm!(thumb_cmp_i8_r5, alu::arm_alu_cmps, 5, NO_WRITE);
mov_compare_add_subtract_imm!(thumb_cmp_i8_r6, alu::arm_alu_cmps, 6, NO_WRITE);
mov_compare_add_subtract_imm!(thumb_cmp_i8_r7, alu::arm_alu_cmps, 7, NO_WRITE);

mov_compare_add_subtract_imm!(thumb_add_i8_r0, alu::arm_alu_adds, 0);
mov_compare_add_subtract_imm!(thumb_add_i8_r1, alu::arm_alu_adds, 1);
mov_compare_add_subtract_imm!(thumb_add_i8_r2, alu::arm_alu_adds, 2);
mov_compare_add_subtract_imm!(thumb_add_i8_r3, alu::arm_alu_adds, 3);
mov_compare_add_subtract_imm!(thumb_add_i8_r4, alu::arm_alu_adds, 4);
mov_compare_add_subtract_imm!(thumb_add_i8_r5, alu::arm_alu_adds, 5);
mov_compare_add_subtract_imm!(thumb_add_i8_r6, alu::arm_alu_adds, 6);
mov_compare_add_subtract_imm!(thumb_add_i8_r7, alu::arm_alu_adds, 7);

mov_compare_add_subtract_imm!(thumb_sub_i8_r0, alu::arm_alu_subs, 0);
mov_compare_add_subtract_imm!(thumb_sub_i8_r1, alu::arm_alu_subs, 1);
mov_compare_add_subtract_imm!(thumb_sub_i8_r2, alu::arm_alu_subs, 2);
mov_compare_add_subtract_imm!(thumb_sub_i8_r3, alu::arm_alu_subs, 3);
mov_compare_add_subtract_imm!(thumb_sub_i8_r4, alu::arm_alu_subs, 4);
mov_compare_add_subtract_imm!(thumb_sub_i8_r5, alu::arm_alu_subs, 5);
mov_compare_add_subtract_imm!(thumb_sub_i8_r6, alu::arm_alu_subs, 6);
mov_compare_add_subtract_imm!(thumb_sub_i8_r7, alu::arm_alu_subs, 7);

pub fn thumb_dp_g1(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);

    let lhs = cpu.registers.read(rd);
    let rhs = cpu.registers.read(rs);

    match bits!(opcode, 6, 7) {
        0 => { let res = alu::arm_alu_ands(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        1 => { let res = alu::arm_alu_eors(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        2 => { let res = alu::arm_alu_llr_s(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        3 => { let res = alu::arm_alu_lrr_s(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        _ => unreachable!(),
    }

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
}

pub fn thumb_dp_g2(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);

    let lhs = cpu.registers.read(rd);
    let rhs = cpu.registers.read(rs);

    match bits!(opcode, 6, 7) {
        0 => { let res = alu::arm_alu_arr_s(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        1 => { let res = alu::arm_alu_adcs(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        2 => { let res = alu::arm_alu_sbcs(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        3 => { let res = alu::arm_alu_rrr_s(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        _ => unreachable!(),
    }

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
}

pub fn thumb_dp_g3(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);

    let lhs = cpu.registers.read(rd);
    let rhs = cpu.registers.read(rs);

    match bits!(opcode, 6, 7) {
        0 => { alu::arm_alu_tsts(cpu, lhs, rhs); },
        1 => { let res = alu::arm_alu_rsbs(cpu, rhs, 0); cpu.registers.write(rd, res) },
        2 => { alu::arm_alu_cmps(cpu, lhs, rhs); },
        3 => { alu::arm_alu_cmns(cpu, lhs, rhs); },
        _ => unreachable!(),
    }

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
}

pub fn thumb_dp_g4(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rs = bits!(opcode, 3, 5);

    let lhs = cpu.registers.read(rd);
    let rhs = cpu.registers.read(rs);

    match bits!(opcode, 6, 7) {
        0 => { let res = alu::arm_alu_orrs(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        1 => {
            let res = lhs.wrapping_mul(rhs);
            cpu.registers.write(rd, res);
            alu::set_nz_flags(cpu, res);
            cpu.cycles += clock::cycles_multiply(memory, false, cpu.registers.read(15), rhs, true);
        },
        2 => { let res = alu::arm_alu_bics(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        3 => { let res = alu::arm_alu_mvns(cpu, lhs, rhs); cpu.registers.write(rd, res) },
        _ => unreachable!(),
    }

    // if this was not a multiply instruction
    if bits!(opcode, 6, 7) != 1 {
        // clock the instruction prefetch
        cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
    }
}

pub fn thumb_addh(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rs_hi = bits_b!(opcode, 6);
    let rd_hi = bits_b!(opcode, 7);
    let rd = bits!(opcode, 0, 2) + (if rd_hi { 8 } else { 0 });
    let rs = bits!(opcode, 3, 5) + (if rs_hi { 8 } else { 0 });

    let lhs = cpu.registers.read(rd);
    let rhs = cpu.registers.read(rs);
    let res = lhs.wrapping_add(rhs); // this version of add does not set flags

    cpu.registers.write(rd, res);

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
    if unlikely!(rd == 15) {
        let dest = cpu.registers.read(15) & 0xFFFFFFFE;
        cpu.cycles += clock::cycles_branch_refill(memory, true, dest);
        cpu.thumb_branch_to(dest, memory);
    }
}

pub fn thumb_cmph(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rs_hi = bits_b!(opcode, 6);
    let rd_hi = bits_b!(opcode, 7);
    let rd = bits!(opcode, 0, 2) + (if rd_hi { 8 } else { 0 });
    let rs = bits!(opcode, 3, 5) + (if rs_hi { 8 } else { 0 });

    let lhs = cpu.registers.read(rd);
    let rhs = cpu.registers.read(rs);
    alu::arm_alu_cmps(cpu, lhs, rhs);

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
}

pub fn thumb_movh(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rs_hi = bits_b!(opcode, 6);
    let rd_hi = bits_b!(opcode, 7);
    let rd = bits!(opcode, 0, 2) + (if rd_hi { 8 } else { 0 });
    let rs = bits!(opcode, 3, 5) + (if rs_hi { 8 } else { 0 });
    let rhs = cpu.registers.read(rs);
    cpu.registers.write(rd, rhs);

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
    if unlikely!(rd == 15) {
        let dest = cpu.registers.read(15) & 0xFFFFFFFE;
        cpu.cycles += clock::cycles_branch_refill(memory, true, dest);
        cpu.thumb_branch_to(dest, memory);
    }
}

pub fn thumb_bx_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rs_hi = bits_b!(opcode, 6);
    let rs = bits!(opcode, 3, 5) + (if rs_hi { 8 } else { 0 });
    let mut dest = cpu.registers.read(rs);

    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));

    if (dest & 1) == 0 {
        dest &= 0xFFFFFFFC;
        cpu.registers.clearf_t();
        cpu.arm_branch_to(dest, memory);
        cpu.cycles += clock::cycles_branch_refill(memory, false, dest);
    } else {
        dest &= 0xFFFFFFFE;
        cpu.thumb_branch_to(dest, memory);
        cpu.cycles += clock::cycles_branch_refill(memory, true, dest);
    }
}

pub fn thumb_b(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let offset = sign_extend_32!((opcode & 0x7FF) << 1, 12);
    let pc = cpu.registers.read(15);
    let dest = pc.wrapping_add(offset) & 0xFFFFFFFE;
    cpu.thumb_branch_to(dest, memory);
    cpu.cycles += clock::cycles_branch(memory, true, pc, dest);
}

#[inline(always)]
fn thumb_ldr_pc(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32, rd: u32) {
    let offset = (opcode & 0xFF) << 2;
    // From ARM7TDMI Documentation:
    //      The value of the PC will be 4 bytes greater than the address of this instruction,
    //      but bit 1 of PC is forced to 0 to ensure it is word aligned.
    //  @ NOTE I also force bit 0 to 0 here because I can't think of any instruction that would
    //         allow you to get an unaligned (halfword-aligned in THUMB mode) address into PC
    //         but I might be wrong.
    let pc = cpu.registers.read(15) & 0xFFFFFFFC;
    let addr = pc.wrapping_add(offset);

    // @ NOTE I just do a raw read here instead of an sdt_ldr because the address will always
    //        be word aligned.
    let data = memory.load32(addr);
    cpu.registers.write(rd, data);

    cpu.cycles += clock::cycles_load_register(memory, true, pc, 32, addr);
}

pub fn thumb_ldr_pc_r0(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldr_pc(cpu, memory, opcode, 0) }
pub fn thumb_ldr_pc_r1(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldr_pc(cpu, memory, opcode, 1) }
pub fn thumb_ldr_pc_r2(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldr_pc(cpu, memory, opcode, 2) }
pub fn thumb_ldr_pc_r3(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldr_pc(cpu, memory, opcode, 3) }
pub fn thumb_ldr_pc_r4(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldr_pc(cpu, memory, opcode, 4) }
pub fn thumb_ldr_pc_r5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldr_pc(cpu, memory, opcode, 5) }
pub fn thumb_ldr_pc_r6(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldr_pc(cpu, memory, opcode, 6) }
pub fn thumb_ldr_pc_r7(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldr_pc(cpu, memory, opcode, 7) }

pub fn thumb_str_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let ro = bits!(opcode, 6, 8);

    let base = cpu.registers.read(rb);
    let offset = cpu.registers.read(ro);
    let addr = base.wrapping_add(offset);

    sdt_str(cpu, memory, rd, addr);

    cpu.cycles += clock::cycles_store_register(memory, true, cpu.registers.read(15), 32, addr);
}

pub fn thumb_ldr_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let ro = bits!(opcode, 6, 8);

    let base = cpu.registers.read(rb);
    let offset = cpu.registers.read(ro);
    let addr = base.wrapping_add(offset);

    sdt_ldr(cpu, memory, rd, addr);

    cpu.cycles += clock::cycles_load_register(memory, true, cpu.registers.read(15), 32, addr);
}

pub fn thumb_strb_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let ro = bits!(opcode, 6, 8);

    let base = cpu.registers.read(rb);
    let offset = cpu.registers.read(ro);
    let addr = base.wrapping_add(offset);

    let value = cpu.registers.read(rd);
    memory.store8(addr, (value & 0xFF) as u8);

    cpu.cycles += clock::cycles_store_register(memory, true, cpu.registers.read(15), 8, addr);
}

pub fn thumb_ldrb_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let ro = bits!(opcode, 6, 8);

    let base = cpu.registers.read(rb);
    let offset = cpu.registers.read(ro);
    let addr = base.wrapping_add(offset);

    let value = memory.load8(addr);
    cpu.registers.write(rd, value as u32);

    cpu.cycles += clock::cycles_load_register(memory, true, cpu.registers.read(15), 8, addr);
}

pub fn thumb_strh_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let ro = bits!(opcode, 6, 8);

    let base = cpu.registers.read(rb);
    let offset = cpu.registers.read(ro);
    let addr = base.wrapping_add(offset);

    let value = cpu.registers.read(rd) & 0xFFFF;
    memory.store16(addr, value as u16);

    cpu.cycles += clock::cycles_store_register(memory, true, cpu.registers.read(15), 16, addr);
}

pub fn thumb_ldrsb_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let ro = bits!(opcode, 6, 8);

    let base = cpu.registers.read(rb);
    let offset = cpu.registers.read(ro);
    let addr = base.wrapping_add(offset);

    let value = memory.load8(addr) as i8 as i32 as u32;
    cpu.registers.write(rd, value);

    cpu.cycles += clock::cycles_load_register(memory, true, cpu.registers.read(15), 8, addr);
}

pub fn thumb_ldrh_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let ro = bits!(opcode, 6, 8);

    let base = cpu.registers.read(rb);
    let offset = cpu.registers.read(ro);
    let addr = base.wrapping_add(offset);

    let value = memory.load16(addr) as u32;
    cpu.registers.write(rd, value);

    cpu.cycles += clock::cycles_load_register(memory, true, cpu.registers.read(15), 16, addr);
}

pub fn thumb_ldrsh_reg(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);
    let ro = bits!(opcode, 6, 8);

    let base = cpu.registers.read(rb);
    let offset = cpu.registers.read(ro);
    let addr = base.wrapping_add(offset);

    let value = memory.load16(addr) as i16 as i32 as u32;
    cpu.registers.write(rd, value);

    cpu.cycles += clock::cycles_load_register(memory, true, cpu.registers.read(15), 16, addr);
}

pub fn thumb_str_imm5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);

    let base = cpu.registers.read(rb);
    let offset = bits!(opcode, 6, 10) << 2;
    let addr = base.wrapping_add(offset);

    sdt_str(cpu, memory, rd, addr);

    cpu.cycles += clock::cycles_store_register(memory, true, cpu.registers.read(15), 32, addr);
}

pub fn thumb_ldr_imm5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);

    let base = cpu.registers.read(rb);
    let offset = bits!(opcode, 6, 10) << 2;
    let addr = base.wrapping_add(offset);

    sdt_ldr(cpu, memory, rd, addr);

    cpu.cycles += clock::cycles_load_register(memory, true, cpu.registers.read(15), 32, addr);
}

pub fn thumb_strb_imm5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);

    let base = cpu.registers.read(rb);
    let offset = bits!(opcode, 6, 10);
    let addr = base.wrapping_add(offset);

    let value = cpu.registers.read(rd);
    memory.store8(addr, (value & 0xFF) as u8);

    cpu.cycles += clock::cycles_store_register(memory, true, cpu.registers.read(15), 8, addr);
}

pub fn thumb_ldrb_imm5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);

    let base = cpu.registers.read(rb);
    let offset = bits!(opcode, 6, 10);
    let addr = base.wrapping_add(offset);

    let value = memory.load8(addr);
    cpu.registers.write(rd, value as u32);

    cpu.cycles += clock::cycles_load_register(memory, true, cpu.registers.read(15), 8, addr);
}

pub fn thumb_strh_imm5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);

    let base = cpu.registers.read(rb);
    let offset = bits!(opcode, 6, 10) << 1;
    let addr = base.wrapping_add(offset);

    let value = cpu.registers.read(rd) & 0xFFFF;
    memory.store16(addr, value as u16);

    cpu.cycles += clock::cycles_store_register(memory, true, cpu.registers.read(15), 16, addr);
}

pub fn thumb_ldrh_imm5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let rd = bits!(opcode, 0, 2);
    let rb = bits!(opcode, 3, 5);

    let base = cpu.registers.read(rb);
    let offset = bits!(opcode, 6, 10) << 1;
    let addr = base.wrapping_add(offset);

    let value = memory.load16(addr) as u32;
    cpu.registers.write(rd, value);

    cpu.cycles += clock::cycles_load_register(memory, true, cpu.registers.read(15), 16, addr);
}

#[inline(always)]
fn thumb_strsp(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32, rd: u32) {
    let offset = bits!(opcode, 0, 7) << 2;
    let addr = cpu.registers.read(13).wrapping_add(offset);

    sdt_str(cpu, memory, rd, addr);

    cpu.cycles += clock::cycles_store_register(memory, true, cpu.registers.read(15), 32, addr);
}

pub fn thumb_strsp_r0(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_strsp(cpu, memory, opcode, 0) }
pub fn thumb_strsp_r1(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_strsp(cpu, memory, opcode, 1) }
pub fn thumb_strsp_r2(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_strsp(cpu, memory, opcode, 2) }
pub fn thumb_strsp_r3(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_strsp(cpu, memory, opcode, 3) }
pub fn thumb_strsp_r4(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_strsp(cpu, memory, opcode, 4) }
pub fn thumb_strsp_r5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_strsp(cpu, memory, opcode, 5) }
pub fn thumb_strsp_r6(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_strsp(cpu, memory, opcode, 6) }
pub fn thumb_strsp_r7(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_strsp(cpu, memory, opcode, 7) }

#[inline(always)]
fn thumb_ldrsp(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32, rd: u32) {
    let offset = bits!(opcode, 0, 7) << 2;
    let addr = cpu.registers.read(13).wrapping_add(offset);

    sdt_ldr(cpu, memory, rd, addr);

    cpu.cycles += clock::cycles_load_register(memory, true, cpu.registers.read(15), 32, addr);
}

pub fn thumb_ldrsp_r0(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldrsp(cpu, memory ,opcode, 0) }
pub fn thumb_ldrsp_r1(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldrsp(cpu, memory ,opcode, 1) }
pub fn thumb_ldrsp_r2(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldrsp(cpu, memory ,opcode, 2) }
pub fn thumb_ldrsp_r3(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldrsp(cpu, memory ,opcode, 3) }
pub fn thumb_ldrsp_r4(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldrsp(cpu, memory ,opcode, 4) }
pub fn thumb_ldrsp_r5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldrsp(cpu, memory ,opcode, 5) }
pub fn thumb_ldrsp_r6(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldrsp(cpu, memory ,opcode, 6) }
pub fn thumb_ldrsp_r7(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldrsp(cpu, memory ,opcode, 7) }

#[inline(always)]
fn thumb_addpc(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32, rd: u32) {
    let offset = (opcode & 0xFF) << 2;
    // From ARM7TDMI Documentation:
    //      When the PC is used as the source register, bit 1 of the PC is always read as 0.
    //      The value of the PC will be 4 bytes greater than the address of the instruction
    //      before bit 1 is forced to 0.
    let pc = cpu.registers.read(15) & 0xFFFFFFFD;
    cpu.registers.write(rd, pc.wrapping_add(offset));

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, pc);
}

#[inline(always)]
fn thumb_addsp(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32, rd: u32) {
    let offset = (opcode & 0xFF) << 2;
    let sp = cpu.registers.read(13);
    cpu.registers.write(rd, sp.wrapping_add(offset));

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
}

pub fn thumb_addpc_r0(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addpc(cpu, memory, opcode, 0) }
pub fn thumb_addpc_r1(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addpc(cpu, memory, opcode, 1) }
pub fn thumb_addpc_r2(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addpc(cpu, memory, opcode, 2) }
pub fn thumb_addpc_r3(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addpc(cpu, memory, opcode, 3) }
pub fn thumb_addpc_r4(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addpc(cpu, memory, opcode, 4) }
pub fn thumb_addpc_r5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addpc(cpu, memory, opcode, 5) }
pub fn thumb_addpc_r6(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addpc(cpu, memory, opcode, 6) }
pub fn thumb_addpc_r7(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addpc(cpu, memory, opcode, 7) }

pub fn thumb_addsp_r0(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addsp(cpu, memory, opcode, 0) }
pub fn thumb_addsp_r1(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addsp(cpu, memory, opcode, 1) }
pub fn thumb_addsp_r2(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addsp(cpu, memory, opcode, 2) }
pub fn thumb_addsp_r3(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addsp(cpu, memory, opcode, 3) }
pub fn thumb_addsp_r4(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addsp(cpu, memory, opcode, 4) }
pub fn thumb_addsp_r5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addsp(cpu, memory, opcode, 5) }
pub fn thumb_addsp_r6(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addsp(cpu, memory, opcode, 6) }
pub fn thumb_addsp_r7(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_addsp(cpu, memory, opcode, 7) }

pub fn thumb_addsp_imm7(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let mut offset = bits!(opcode, 0, 6) << 2;
    if bits_b!(opcode, 7) {
        offset = -(offset as i32) as u32
    }
    let sp = cpu.registers.read(13);
    cpu.registers.write(13, sp.wrapping_add(offset));

    // clock the instruction prefetch
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
}

pub fn thumb_push(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let register_list = opcode & 0xFF;
    let reg_count = register_list.count_ones();
    let base = cpu.registers.read(13);

    // the lowest register always goes into the lowest address so we precalculate the lowest
    // address (minus 4) here:
    let mut addr = base.wrapping_sub(reg_count * 4).wrapping_sub(4);

    // writeback the value to the base register (R13)
    cpu.registers.write(13, base.wrapping_sub(reg_count * 4));

    // count prefetch cycles
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));

    // transfer
    let mut first = true;
    for reg in 0..8 {
        if (register_list & (1 << reg)) != 0 {
            addr = addr.wrapping_add(4);

            let value = cpu.registers.read(reg);
            memory.store32(addr, value);

            if first {
                cpu.cycles += memory.data_access_nonseq32(addr);
                first = false;
            } else {
                cpu.cycles += memory.data_access_seq32(addr);
            }
        }
    }
}

pub fn thumb_push_lr(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let register_list = opcode & 0xFF;
    let reg_count = register_list.count_ones() + 1; // add one for LR
    let base = cpu.registers.read(13);

    // the lowest register always goes into the lowest address so we precalculate the lowest
    // address (minus 4) here:
    let mut addr = base.wrapping_sub(reg_count * 4).wrapping_sub(4);

    // writeback the value to the base register (R13)
    cpu.registers.write(13, base.wrapping_sub(reg_count * 4));

    // count prefetch cycles
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));

    // transfer
    let mut first = true;
    for reg in 0..8 {
        if (register_list & (1 << reg)) != 0 {
            addr = addr.wrapping_add(4);

            let value = cpu.registers.read(reg);
            memory.store32(addr, value);

            if first {
                cpu.cycles += memory.data_access_nonseq32(addr);
                first = false;
            } else {
                cpu.cycles += memory.data_access_seq32(addr);
            }
        }
    }

    // transfer LR
    addr = addr.wrapping_add(4);
    let value = cpu.registers.read(14);
    memory.store32(addr, value);
    if first {
        cpu.cycles += memory.data_access_nonseq32(addr);
    } else {
        cpu.cycles += memory.data_access_seq32(addr);
    }
}

pub fn thumb_pop(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let register_list = opcode & 0xFF;
    let reg_count = register_list.count_ones();
    let base = cpu.registers.read(13);

    // the lowest register always goes into the lowest address so we precalculate the lowest
    // address (minus 4) here:
    let mut addr = base.wrapping_sub(4);

    // writeback the ending address to the base register (R13)
    cpu.registers.write(13, base.wrapping_add(reg_count * 4));

    // count prefetch cycles
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));

    // transfer
    let mut first = true;
    for reg in 0..8 {
        if (register_list & (1 << reg)) != 0 {
            addr = addr.wrapping_add(4);

            let value = memory.load32(addr);
            cpu.registers.write(reg, value);

            if first {
                cpu.cycles += memory.data_access_nonseq32(addr);
                first = false;
            } else {
                cpu.cycles += memory.data_access_seq32(addr);
            }
        }
    }
}

pub fn thumb_pop_pc(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let register_list = opcode & 0xFF;
    let reg_count = register_list.count_ones() + 1; // count the PC in the register list
    let base = cpu.registers.read(13);

    // the lowest register always goes into the lowest address so we precalculate the lowest
    // address (minus 4) here:
    let mut addr = base.wrapping_sub(4);

    // writeback the value to the base register (R13)
    cpu.registers.write(13, base.wrapping_add(reg_count * 4));

    // count prefetch cycles
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));

    // transfer
    let mut first = true;
    for reg in 0..8 {
        if (register_list & (1 << reg)) != 0 {
            addr = addr.wrapping_add(4);

            let value = memory.load32(addr);
            cpu.registers.write(reg, value);

            if first {
                cpu.cycles += memory.data_access_nonseq32(addr);
                first = false;
            } else {
                cpu.cycles += memory.data_access_seq32(addr);
            }
        }
    }

    // transfer PC
    addr = addr.wrapping_add(4);
    let value = memory.load32(addr);
    let dest = value & 0xFFFFFFFE;
    cpu.thumb_branch_to(dest, memory);
    if first {
        cpu.cycles += memory.data_access_nonseq32(addr);
    } else {
        cpu.cycles += memory.data_access_seq32(addr);
    }
}

fn thumb_stmia(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32, rb: u32) {
    let register_list = opcode & 0xFF;
    let reg_count = register_list.count_ones();
    let base = cpu.registers.read(rb);

    // the lowest register always goes into the lowest address so we precalculate the lowest
    let mut addr = base.wrapping_sub(4);

    // count prefetch cycles
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));

    // transfer
    let mut first = true;
    for reg in 0..8 {
        if (register_list & (1 << reg)) != 0 {
            addr = addr.wrapping_add(4);

            let value = cpu.registers.read(reg);
            memory.store32(addr, value);

            if first {
                cpu.cycles += memory.data_access_nonseq32(addr);
                first = false;

                // @NOTE see ARM block data transfer instruction documentation for why this is
                //       here.
                // writeback the ending address to the base register
                cpu.registers.write(rb, base.wrapping_add(reg_count * 4));
            } else {
                cpu.cycles += memory.data_access_seq32(addr);
            }
        }
    }
}

fn thumb_ldmia(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32, rb: u32) {
    let register_list = opcode & 0xFF;
    let reg_count = register_list.count_ones();
    let base = cpu.registers.read(rb);

    // the lowest register always goes into the lowest address so we precalculate the lowest
    let mut addr = base.wrapping_sub(4);

    // writeback the ending address to the base register
    cpu.registers.write(rb, base.wrapping_add(reg_count * 4));

    // count prefetch cycles
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));

    // transfer
    let mut first = true;
    for reg in 0..8 {
        if (register_list & (1 << reg)) != 0 {
            addr = addr.wrapping_add(4);

            let value = memory.load32(addr);
            cpu.registers.write(reg, value);

            if first {
                cpu.cycles += memory.data_access_nonseq32(addr);
                first = false;
            } else {
                cpu.cycles += memory.data_access_seq32(addr);
            }
        }
    }
}

pub fn thumb_stmia_r0(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_stmia(cpu, memory, opcode, 0) }
pub fn thumb_stmia_r1(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_stmia(cpu, memory, opcode, 1) }
pub fn thumb_stmia_r2(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_stmia(cpu, memory, opcode, 2) }
pub fn thumb_stmia_r3(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_stmia(cpu, memory, opcode, 3) }
pub fn thumb_stmia_r4(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_stmia(cpu, memory, opcode, 4) }
pub fn thumb_stmia_r5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_stmia(cpu, memory, opcode, 5) }
pub fn thumb_stmia_r6(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_stmia(cpu, memory, opcode, 6) }
pub fn thumb_stmia_r7(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_stmia(cpu, memory, opcode, 7) }

pub fn thumb_ldmia_r0(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldmia(cpu, memory, opcode, 0) }
pub fn thumb_ldmia_r1(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldmia(cpu, memory, opcode, 1) }
pub fn thumb_ldmia_r2(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldmia(cpu, memory, opcode, 2) }
pub fn thumb_ldmia_r3(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldmia(cpu, memory, opcode, 3) }
pub fn thumb_ldmia_r4(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldmia(cpu, memory, opcode, 4) }
pub fn thumb_ldmia_r5(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldmia(cpu, memory, opcode, 5) }
pub fn thumb_ldmia_r6(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldmia(cpu, memory, opcode, 6) }
pub fn thumb_ldmia_r7(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_ldmia(cpu, memory, opcode, 7) }

/// Conditional Branch
#[inline(always)]
fn thumb_b_cond(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32, cond: u32) {
    use super::super::cpu::check_condition;

    if check_condition(cond, &cpu.registers) {
        let offset = sign_extend_32!((opcode & 0xFF) << 1, 9);
        let pc = cpu.registers.read(15);
        let dest = pc.wrapping_add(offset) & 0xFFFFFFFE;
        cpu.thumb_branch_to(dest, memory);
        cpu.cycles += clock::cycles_branch(memory, true, pc, dest);
    } else {
        // count prefetch cycles
        cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
    }
}

pub fn thumb_beq(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b0000) }
pub fn thumb_bne(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b0001) }
pub fn thumb_bcs(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b0010) }
pub fn thumb_bcc(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b0011) }
pub fn thumb_bmi(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b0100) }
pub fn thumb_bpl(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b0101) }
pub fn thumb_bvs(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b0110) }
pub fn thumb_bvc(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b0111) }
pub fn thumb_bhi(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b1000) }
pub fn thumb_bls(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b1001) }
pub fn thumb_bge(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b1010) }
pub fn thumb_blt(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b1011) }
pub fn thumb_bgt(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b1100) }
pub fn thumb_ble(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) { thumb_b_cond(cpu, memory, opcode, 0b1101) }

pub fn thumb_bl_setup(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let pc = cpu.registers.read(15);
    let off = sign_extend_32!((opcode & 0x7FF) << 12, 23);
    let setup = pc.wrapping_add(off);
    cpu.registers.write(14, setup);

    cpu.cycles += clock::cycles_thumb_bl_setup(memory, pc);
}

pub fn thumb_bl_off(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, opcode: u32) {
    let pc = cpu.registers.read(15);
    let lr = cpu.registers.read(14);
    let off = (opcode & 0x7FF) << 1;
    let dest = lr.wrapping_add(off) & 0xFFFFFFFE;
    cpu.registers.write(14, (pc.wrapping_sub(2)) | 1);
    cpu.thumb_branch_to(dest, memory);

    cpu.cycles += clock::cycles_thumb_bl_jump(memory, pc, dest);
}

pub fn thumb_swi(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, _opcode: u32) {
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
    cpu.handle_exception(super::super::cpu::CpuException::SWI, memory);
}

pub fn thumb_undefined(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory ,_opcode: u32) {
    cpu.cycles += clock::cycles_prefetch(memory, true, cpu.registers.read(15));
    cpu.handle_exception(super::super::cpu::CpuException::Undefined, memory);
}
