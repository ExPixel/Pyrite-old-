use super::ArmCpu;

#[inline]
pub fn internal_multiply_cycles(mut rhs: u32, signed: bool) -> u32 {
    // if the most significant bits of RHS are set (RHS is negative), we use !RHS so that we can
    // just check if they are zero instead to handle both the positive and negative case.
    if signed && (rhs as i32) < 0 { rhs = !rhs }

    if (rhs & 0xFFFFFF00) == 0 {
        // m = 1, if bits [32:8] of the multiplier operand are all zero or all one
        1
    } else if (rhs & 0xFFFF0000) == 0 {
        // m = 2, if bits [32:16] of the multiplier operand are all zero or all one.
        2
    } else if (rhs & 0xFF000000) == 0 {
        // m = 3, if bits [32:24] of the multiplier operand are all zero or all one.
        3
    } else {
        // m = 4, in all other cases
        4
    }
}

#[inline]
pub fn arm_alu_adc(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    lhs.wrapping_add(rhs).wrapping_add(if cpu.registers.getf_c() { 1 } else { 0 })
}

#[inline]
pub fn arm_alu_sbc(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    lhs.wrapping_sub(rhs).wrapping_sub(if !cpu.registers.getf_c() { 1 } else { 0 })
}

#[inline]
pub fn arm_alu_rsc(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    rhs.wrapping_sub(lhs).wrapping_sub(if !cpu.registers.getf_c() { 1 } else { 0 })
}


#[inline]
pub fn arm_alu_adcs(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    let carry = if cpu.registers.getf_c() { 1 } else { 0 };
    let res = arm_alu_adc(cpu, lhs, rhs);
    set_adc_flags(cpu, lhs, rhs, carry, res);
    res
}

#[inline]
pub fn arm_alu_sbcs(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    let not_carry = if !cpu.registers.getf_c() { 1 } else { 0 };
    let res = arm_alu_sbc(cpu, lhs, rhs);
    set_sbc_flags(cpu, lhs, rhs, not_carry, res);
    res
}

#[inline]
pub fn arm_alu_rscs(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    let not_carry = if !cpu.registers.getf_c() { 1 } else { 0 };
    let res = arm_alu_rsc(cpu, lhs, rhs);
    set_sbc_flags(cpu, rhs, lhs, not_carry, res);
    res
}

#[inline]
pub fn arm_alu_adds(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    let res = lhs.wrapping_add(rhs);
    set_add_flags(cpu, lhs, rhs, res);
    res
}

#[inline(always)]
pub fn arm_alu_add(_cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    lhs.wrapping_add(rhs)
}

#[inline]
pub fn arm_alu_subs(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    let res = lhs.wrapping_sub(rhs);
    set_sub_flags(cpu, lhs, rhs, res);
    res
}

#[inline(always)]
pub fn arm_alu_sub(_cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    lhs.wrapping_sub(rhs)
}

#[inline]
pub fn arm_alu_rsbs(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    let res = rhs.wrapping_sub(lhs);
    set_sub_flags(cpu, rhs, lhs, res);
    res
}

#[inline(always)]
pub fn arm_alu_rsb(_cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    rhs.wrapping_sub(lhs)
}

#[inline]
pub fn arm_alu_ands(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    let res = lhs & rhs;
    set_nz_flags(cpu, res);
    res
}

#[inline(always)]
pub fn arm_alu_and(_cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    lhs & rhs
}

#[inline]
pub fn arm_alu_orrs(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    let res = lhs | rhs;
    set_nz_flags(cpu, res);
    res
}

#[inline(always)]
pub fn arm_alu_orr(_cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    lhs | rhs
}

#[inline]
pub fn arm_alu_eors(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    let res = lhs ^ rhs;
    set_nz_flags(cpu, res);
    res
}

#[inline(always)]
pub fn arm_alu_eor(_cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    lhs ^ rhs
}

#[inline]
pub fn arm_alu_bics(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    let res = lhs & !rhs;
    set_nz_flags(cpu, res);
    res
}

#[inline(always)]
pub fn arm_alu_bic(_cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    lhs & !rhs
}

#[inline]
pub fn arm_alu_movs(cpu: &mut ArmCpu, _lhs: u32, rhs: u32) -> u32 {
    set_nz_flags(cpu, rhs);
    rhs
}

#[inline(always)]
pub fn arm_alu_mov(_cpu: &mut ArmCpu, _lhs: u32, rhs: u32) -> u32 {
    rhs
}

#[inline]
pub fn arm_alu_mvns(cpu: &mut ArmCpu, _lhs: u32, rhs: u32) -> u32 {
    let res = !rhs;
    set_nz_flags(cpu, res);
    res
}

#[inline(always)]
pub fn arm_alu_mvn(_cpu: &mut ArmCpu, _lhs: u32, rhs: u32) -> u32 {
    !rhs
}

#[inline]
pub fn arm_alu_cmps(cpu: &mut ArmCpu, lhs: u32, rhs: u32) {
    let res = lhs.wrapping_sub(rhs);
    set_sub_flags(cpu, lhs, rhs, res);
}

#[inline]
pub fn arm_alu_cmns(cpu: &mut ArmCpu, lhs: u32, rhs: u32) {
    let res = lhs.wrapping_add(rhs);
    set_add_flags(cpu, lhs, rhs, res);
}

#[inline]
pub fn arm_alu_teqs(cpu: &mut ArmCpu, lhs: u32, rhs: u32) {
    set_nz_flags(cpu, lhs ^ rhs);
}

#[inline]
pub fn arm_alu_tsts(cpu: &mut ArmCpu, lhs: u32, rhs: u32) {
    set_nz_flags(cpu, lhs & rhs);
}

#[inline]
pub fn set_nz_flags64(cpu: &mut ArmCpu, res: u64) {
    cpu.registers.putf_n(((res >> 63) & 1) == 1);
    cpu.registers.putf_z(res == 0);
}

#[inline]
pub fn set_nz_flags(cpu: &mut ArmCpu, res: u32) {
    cpu.registers.putfi_n((res >> 31) & 1);
    cpu.registers.putf_z(res == 0);
}

#[inline]
pub fn set_add_flags(cpu: &mut ArmCpu, lhs: u32, rhs: u32, res: u32) {
    cpu.registers.putfi_n((res >> 31) & 1);
    cpu.registers.putf_z(res == 0);

    let (_, carry) = lhs.overflowing_add(rhs);
    let (_, overflow) = (lhs as i32).overflowing_add(rhs as i32);

    cpu.registers.putf_c(carry);
    cpu.registers.putf_v(overflow);
}

#[inline]
pub fn set_sub_flags(cpu: &mut ArmCpu, lhs: u32, rhs: u32, res: u32) {
    cpu.registers.putfi_n((res >> 31) & 1);
    cpu.registers.putf_z(res == 0);

    let (_, overflow) = (lhs as i32).overflowing_sub(rhs as i32);

    // #NOTE The concept of a borrow is not the same in ARM as it is in x86.
    //       while in x86 the borrow flag is set if lhs < rhs, in ARM
    //       if is set if lhs >= rhs.
    cpu.registers.putf_c(lhs >= rhs);

    cpu.registers.putf_v(overflow);
}

#[inline]
pub fn set_sbc_flags(cpu: &mut ArmCpu, lhs: u32, rhs: u32, not_carry: u32, res: u32) {
    cpu.registers.putfi_n((res >> 31) & 1);
    cpu.registers.putf_z(res == 0);

    // #NOTE The concept of a borrow is not the same in ARM as it is in x86.
    //       while in x86 the borrow flag is set if lhs < rhs, in ARM
    //       if is set if lhs >= rhs.
    cpu.registers.putf_c((lhs as u64) >= (rhs as u64 + not_carry as u64));

    cpu.registers.putf_v((((lhs >> 31)^rhs)&((lhs >> 31) ^ res)) != 0);
}

#[inline]
pub fn set_adc_flags(cpu: &mut ArmCpu, lhs: u32, rhs: u32, carry: u32, res: u32) {
    cpu.registers.putfi_n((res >> 31) & 1);
    cpu.registers.putf_z(res == 0);

    let (res_0, carry_0) = lhs.overflowing_add(rhs);
    let (_, overflow_0) = (lhs as i32).overflowing_add(rhs as i32);

    let (_, carry_1) = res_0.overflowing_add(carry);
    let (_, overflow_1) = (res_0 as i32).overflowing_add(carry as i32);

    cpu.registers.putf_c(carry_0 | carry_1);
    cpu.registers.putf_v(overflow_0 | overflow_1);
}


// ---- ARM ALU SHIFTS ----
#[inline]
pub fn arm_alu_lli(lhs: u32, rhs: u32) -> u32 {
    // LSL #0 is a special case, where the shifter carry out is the old value of the CPSR C
    // flag. The contents of Rm are used directly as the second operand.
    if rhs == 0 { lhs }
    else { lhs.arm_lsl(rhs) }
}

#[inline]
pub fn arm_alu_llr(lhs: u32, rhs: u32) -> u32 {
    // If this byte is zero, the unchanged contents of Rm will be used as the second operand,
    // and the old value of the CPSR C flag will be passed on as the shifter carry output.
    if rhs == 0 { return lhs }
    // LSL by 32 has result zero, carry out equal to bit 0 of Rm.
    // LSL by more than 32 has result zero, carry out zero.
    if rhs >= 32 { 0 }
    else { lhs.arm_lsl(rhs) }
}

#[inline]
pub fn arm_alu_lri(lhs: u32, rhs: u32) -> u32 {
    // The form of the shift field which might be expected to correspond to LSR #0 is used to encode LSR #32,
    // which has a zero result with bit 31 of Rm as the carry output.
    if rhs == 0 { 0 }
    else { lhs.arm_lsr(rhs) }
}

#[inline]
pub fn arm_alu_lrr(lhs: u32, rhs: u32) -> u32 {
    // If this byte is zero, the unchanged contents of Rm will be used as the second operand,
    // and the old value of the CPSR C flag will be passed on as the shifter carry output.
    if rhs == 0 { return lhs }
    // LSR by 32 has result zero, carry out equal to bit 31 of Rm.
    // LSR by more than 32 has result zero, carry out zero.
    if rhs >= 32 { 0 }
    else { lhs.arm_lsr(rhs) }
}

#[inline]
pub fn arm_alu_ari(lhs: u32, rhs: u32) -> u32 {
    // The form of the shift field which might be expected to give ASR #0
    // is used to encode ASR #32
    if rhs == 0 {
        if (lhs & 0x80000000) == 0 { 0x00000000 }
        else { 0xffffffff }
    } else {
        lhs.arm_asr(rhs)
    }
}

#[inline]
pub fn arm_alu_arr(lhs: u32, rhs: u32) -> u32 {
    // If this byte is zero, the unchanged contents of Rm will be used as the second operand,
    // and the old value of the CPSR C flag will be passed on as the shifter carry output.
    if rhs == 0 { return lhs }
    // ASR by 32 or more has result filled with and carry out equal to bit 31 of Rm.
    if rhs >= 32 {
        if (lhs & 0x80000000) == 0 { 0x00000000 }
        else { 0xffffffff }
    } else {
        lhs.arm_asr(rhs)
    }
}

#[inline]
pub fn arm_alu_rri(cpu: &ArmCpu, lhs: u32, rhs: u32) -> u32 {
    // The form of the shift field which might be expected to give ROR #0
    // is used to encode a special function of the barrel shifter, rotate right extended (RRX)
    if rhs == 0 { return arm_alu_rrx(cpu, lhs) }
    lhs.arm_ror(rhs)
}

#[inline]
pub fn arm_alu_rrr(lhs: u32, rhs: u32) -> u32 {
    // If this byte is zero, the unchanged contents of Rm will be used as the second operand,
    // and the old value of the CPSR C flag will be passed on as the shifter carry output.
    if rhs == 0 { return lhs }
    // ROR by 32 has result equal to Rm, carry out equal to bit 31 of Rm.
    if rhs == 32 { lhs }
    else {
        // ROR by n where n is greater than 32 will give the same result and carry out as ROR by n-32;
        // therefore repeatedly subtract 32 from n until the amount is in the range 1 to 32 and see above.
        let rhs = rhs & 31; // This might not be right?

        lhs.arm_ror(rhs)
    }
}

#[inline]
pub fn arm_alu_rrx(cpu: &ArmCpu, lhs: u32) -> u32 {
    let carry = if cpu.registers.getf_c() { 1 } else { 0 };
    lhs.arm_rrx(carry)
}

// ---- ARM ALU SHIFTS + FLAGS ----
#[inline]
pub fn arm_alu_lli_s(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    // LSL #0 is a special case, where the shifter carry out is the old value of the CPSR C
    // flag. The contents of Rm are used directly as the second operand.
    if rhs == 0 { lhs }
    else {
        cpu.registers.putfi_c((lhs >> (32 - rhs)) & 1);
        lhs.arm_lsl(rhs)
    }
}

#[inline]
pub fn arm_alu_llr_s(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    // If this byte is zero, the unchanged contents of Rm will be used as the second operand,
    // and the old value of the CPSR C flag will be passed on as the shifter carry output.
    if rhs == 0 { return lhs }
    // LSL by 32 has result zero, carry out equal to bit 0 of Rm.
    // LSL by more than 32 has result zero, carry out zero.
    if rhs == 32 { cpu.registers.putfi_c(lhs & 1); 0 }
    else if rhs > 32 { cpu.registers.clearf_c(); 0 }
    else {
        cpu.registers.putfi_c((lhs >> (32 - rhs)) & 1);
        lhs.arm_lsl(rhs)
    }
}

#[inline]
pub fn arm_alu_lri_s(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    // The form of the shift field which might be expected to correspond to LSR #0 is used to encode LSR #32,
    // which has a zero result with bit 31 of Rm as the carry output.
    // Logical shift right zero is redundant as it is the same as logical shift left zero,
    // so the assembler will convert LSR #0 (and ASR #0 and ROR #0) into LSL #0, and allow LSR #32 to be specified.
    if rhs == 0 {
        cpu.registers.putfi_c(lhs & 0x80000000);
        0
    } else {
        cpu.registers.putfi_c((lhs >> (rhs - 1)) & 1);
        lhs.arm_lsr(rhs)
    }
}

#[inline]
pub fn arm_alu_lrr_s(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    // If this byte is zero, the unchanged contents of Rm will be used as the second operand,
    // and the old value of the CPSR C flag will be passed on as the shifter carry output.
    if rhs == 0 { return lhs }
    // LSR by 32 has result zero, carry out equal to bit 31 of Rm.
    if rhs == 32 { cpu.registers.putfi_c(lhs & 0x80000000); 0 }
    // LSR by more than 32 has result zero, carry out zero.
    else if rhs > 32 { cpu.registers.clearf_c(); 0 }
    else {
        cpu.registers.putfi_c((lhs >> (rhs - 1)) & 1);
        lhs.arm_lsr(rhs)
    }
}

#[inline]
pub fn arm_alu_ari_s(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    // The form of the shift field which might be expected to give ASR #0 is used to encode ASR #32.
    // Bit 31 of Rm is again used as the carry output, and each bit of operand 2 is also equal to bit 31 of Rm.
    // The result is therefore all ones or all zeros, according to the value of bit 31 of Rm.
    if rhs == 0 {
        cpu.registers.putfi_c(lhs & 0x80000000);
        if (lhs & 0x80000000) == 0 { 0x00000000 }
        else { 0xffffffff }
    } else {
        cpu.registers.putfi_c((lhs >> (rhs - 1)) & 1);
        lhs.arm_asr(rhs)
    }
}

#[inline]
pub fn arm_alu_arr_s(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    // If this byte is zero, the unchanged contents of Rm will be used as the second operand,
    // and the old value of the CPSR C flag will be passed on as the shifter carry output.
    if rhs == 0 { return lhs }
    // ASR by 32 or more has result filled with and carry out equal to bit 31 of Rm.
    if rhs >= 32 {
        cpu.registers.putfi_c(lhs & 0x80000000);
        if (lhs & 0x80000000) == 0 { 0x00000000 }
        else { 0xffffffff }
    } else {
        cpu.registers.putfi_c((lhs >> (rhs - 1)) & 1);
        lhs.arm_asr(rhs)
    }
}

#[inline]
pub fn arm_alu_rri_s(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    // The form of the shift field which might be expected to give ROR #0
    // is used to encode a special function of the barrel shifter, rotate right extended (RRX)
    if rhs == 0 { return arm_alu_rrx_s(cpu, lhs) }
    cpu.registers.putfi_c((lhs >> (rhs - 1)) & 1);
    lhs.arm_ror(rhs)
}

#[inline]
pub fn arm_alu_rrr_s(cpu: &mut ArmCpu, lhs: u32, rhs: u32) -> u32 {
    // If this byte is zero, the unchanged contents of Rm will be used as the second operand,
    // and the old value of the CPSR C flag will be passed on as the shifter carry output.
    if rhs == 0 { return lhs }
    // ROR by 32 has result equal to Rm, carry out equal to bit 31 of Rm.
    if rhs == 32 { cpu.registers.putfi_c(lhs & 0x80000000); lhs }
    else {
        // ROR by n where n is greater than 32 will give the same result and carry out as ROR by n-32;
        // therefore repeatedly subtract 32 from n until the amount is in the range 1 to 32 and see above.
        let rhs = rhs & 31; // This might not be right?

        cpu.registers.putfi_c((lhs >> (rhs - 1)) & 1);
        lhs.arm_ror(rhs)
    }
}

#[inline]
pub fn arm_alu_rrx_s(cpu: &mut ArmCpu, lhs: u32) -> u32 {
    let carry = if cpu.registers.getf_c() { 1 } else { 0 };
    cpu.registers.putfi_c(lhs & 1);
    lhs.arm_rrx(carry)
}

/// Clearer versions of shifts
pub trait ArmShifts {
    /// Logical Shift Left
    #[inline(always)]
    fn arm_lsl(self, shift: Self) -> Self;

    /// Logical Shift Right
    #[inline(always)]
    fn arm_lsr(self, shift: Self) -> Self;

    /// Arithmetic Shift Right
    #[inline(always)]
    fn arm_asr(self, shift: Self) -> Self;

    /// Rotate Right
    #[inline(always)]
    fn arm_ror(self, shift: Self) -> Self;

    /// Rotate Right Extended
    #[inline(always)]
    fn arm_rrx(self, carry: Self) -> Self;
}

impl ArmShifts for u32 {
    /// Logical Shift Left
    #[inline(always)]
    fn arm_lsl(self, shift: u32) -> u32 { self << shift }

    /// Logical Shift Right
    #[inline(always)]
    fn arm_lsr(self, shift: u32) -> u32 { self >> shift }

    /// Arithmetic Shift Right
    #[inline(always)]
    fn arm_asr(self, shift: u32) -> u32 { ((self as i32) >> shift) as u32 }

    /// Rotate Right
    #[inline(always)]
    fn arm_ror(self, shift: u32) -> u32 {
        // // This does become an ROR instruction at the end. :P
        // // rorl %cl, %edi
        // (self << (32 - shift)) | (self >> shift)
        self.rotate_right(shift)
    }

    /// Rotate Right Extended
    #[inline(always)]
    fn arm_rrx(self, carry: u32) -> u32 {
        (self >> 1) | (carry << 31)
    }
}

/// Barrel shifter operations used for getting the value of the
/// second operand from a data processing instruction.
pub mod bs {
    use super::*;

    macro_rules! define_shift_by_reg {
        ($func_name:ident, $func_no_s:ident, $func_name_s:ident, $func_s:ident) => (
            pub fn $func_name(cpu: &mut ArmCpu, instr: u32) -> u32 {
                let rm = bits!(instr, 0,  3);
                let rs = bits!(instr, 8, 11);
                // When using R15 as operand (Rm or Rn), the returned value
                // depends on the instruction: PC+12 if I=0,R=1 (shift by register),
                // otherwise PC+8 (shift by immediate).
                let rm_v = if rm == 15 {
                    cpu.registers.read(rm) + 4
                } else {
                    cpu.registers.read(rm)
                };
                let rs_v = cpu.registers.read(rs) & 0xff; // we only use the lower 8bits of rs
                return $func_no_s(rm_v, rs_v);
            }

            pub fn $func_name_s(cpu: &mut ArmCpu, instr: u32) -> u32 {
                let rm = bits!(instr, 0, 3);
                let rs = bits!(instr, 8, 11);

                // When using R15 as operand (Rm or Rn), the returned value
                // depends on the instruction: PC+12 if I=0,R=1 (shift by register),
                // otherwise PC+8 (shift by immediate).
                let rm_v = if rm == 15 {
                    cpu.registers.read(rm) + 4
                } else {
                    cpu.registers.read(rm)
                };
                let rs_v = cpu.registers.read(rs) & 0xff; // we only use the lower 8bits of rs

                return $func_s(cpu, rm_v, rs_v);
            }
        )
    }

    define_shift_by_reg!(llr, arm_alu_llr, llr_s, arm_alu_llr_s);
    define_shift_by_reg!(lrr, arm_alu_lrr, lrr_s, arm_alu_lrr_s);
    define_shift_by_reg!(arr, arm_alu_arr, arr_s, arm_alu_arr_s);
    define_shift_by_reg!(rrr, arm_alu_rrr, rrr_s, arm_alu_rrr_s);


    macro_rules! define_shift_by_imm {
        ($func_name:ident, $func_no_s:ident, $func_name_s:ident, $func_s:ident) => (
            pub fn $func_name(cpu: &mut ArmCpu, instr: u32) -> u32 {
                let rm  = bits!(instr, 0,  3);
                let imm = bits!(instr, 7, 11);
                let rm_v = cpu.registers.read(rm);
                return $func_no_s(rm_v, imm);
            }

            pub fn $func_name_s(cpu: &mut ArmCpu, instr: u32) -> u32 {
                let rm  = bits!(instr, 0,  3);
                let imm = bits!(instr, 7, 11);
                let rm_v = cpu.registers.read(rm);
                return $func_s(cpu, rm_v, imm);
            }
        );

        // This is dumb by I need it for rri :(
        ($special:expr, $func_name:ident, $func_no_s:ident, $func_name_s:ident, $func_s:ident) => (
            pub fn $func_name(cpu: &mut ArmCpu, instr: u32) -> u32 {
                let rm  = bits!(instr, 0,  3);
                let imm = bits!(instr, 7, 11);
                let rm_v = cpu.registers.read(rm);
                return $func_no_s(cpu, rm_v, imm);
            }

            pub fn $func_name_s(cpu: &mut ArmCpu, instr: u32) -> u32 {
                let rm  = bits!(instr, 0,  3);
                let imm = bits!(instr, 7, 11);
                let rm_v = cpu.registers.read(rm);
                return $func_s(cpu, rm_v, imm);
            }
        );
    }

    define_shift_by_imm!(lli, arm_alu_lli, lli_s, arm_alu_lli_s);
    define_shift_by_imm!(lri, arm_alu_lri, lri_s, arm_alu_lri_s);
    define_shift_by_imm!(ari, arm_alu_ari, ari_s, arm_alu_ari_s);
    define_shift_by_imm!("SPECIAL",
                         rri, arm_alu_rri, rri_s, arm_alu_rri_s);

    /// Rotate right by immediate
    pub fn imm(_cpu: &mut ArmCpu, instr: u32) -> u32 {
        let imm = bits!(instr, 0, 7);
        let rot = bits!(instr, 8, 11);
        return imm.arm_ror(rot * 2);
    }

    /// Rotate right by immediate
    pub fn imm_nc(instr: u32) -> u32 {
        let imm = bits!(instr, 0, 7);
        let rot = bits!(instr, 8, 11);
        return imm.arm_ror(rot * 2);
    }
}
