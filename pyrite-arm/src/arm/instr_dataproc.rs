use super::super::alu::*;
use super::super::{ArmCpu, ArmMemory};

pub const S_SET: bool = true;
pub const S_CLR: bool = false;
pub const REG_SHIFT: bool = true;
pub const IMM_SHIFT: bool = false;

/// Creates a function for an arithmetic data processing function that writes
/// back to the destination register.
macro_rules! dataproc {
    ($name:ident, $get_operand:expr, $operation:expr, $s_flag:expr, $r_shift:expr) => {
        // #TODO possibly add some debug code here in the data processing instruction
        //       gen so that I can log an error when Rs is R15 which is not supported
        //       by these sets of instructions.
        pub fn $name(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) -> u32 {
            let mut cycles = cpu.arm_prefetch(memory);

            let rd = bits!(instr, 12, 15);
            let rn = bits!(instr, 16, 19);

            // When using R15 as operand (Rm or Rn), the returned value
            // depends on the instruction: PC+12 if I=0,R=1 (shift by register),
            // otherwise PC+8 (shift by immediate).
            let lhs = if rn == 15 && $r_shift {
                cpu.registers.read(rn) + 4
            } else {
                cpu.registers.read(rn)
            };

            // clock the register shift
            if $r_shift {
                cycles += 1;
                memory.on_internal_cycles(1);
            }

            // If S=1, Rd=R15; should not be used in user mode:
            //   CPSR = SPSR_<current mode>
            //   PC = result
            //   For example: MOVS PC,R14  ;return from SWI (PC=R14_svc, CPSR=SPSR_svc).
            if unlikely!(rd == 15 && $s_flag) {
                let rhs = $get_operand(cpu, instr);
                let res = $operation(cpu, lhs, rhs);
                let spsr = cpu.registers.read_spsr();
                cpu.registers.write_cpsr(spsr);
                cycles += cpu.branch_to(res, memory);
            } else {
                let rhs = $get_operand(cpu, instr);
                let res = $operation(cpu, lhs, rhs);
                if unlikely!(rd == 15) {
                    cycles += cpu.arm_branch_to(res & 0xFFFFFFFC, memory);
                } else {
                    cpu.registers.write(rd, res);
                }
            }

            return cycles;
        }
    };
}

/// Creates a function data processing function that does not write
/// back to the destination register. These always have the S flag set.
macro_rules! dataproc_no_write {
    ($name:ident, $get_operand:expr, $operation:expr, $r_shift:expr) => {
        // #TODO possibly add some debug code here in the data processing instruction
        //       gen so that I can log an error when Rs is R15 which is not supported
        //       by these sets of instructions.
        pub fn $name(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) -> u32 {
            let mut cycles = cpu.arm_prefetch(memory);

            let rd = bits!(instr, 12, 15);
            let rn = bits!(instr, 16, 19);
            // When using R15 as operand (Rm or Rn), the returned value
            // depends on the instruction: PC+12 if I=0,R=1 (shift by register),
            // otherwise PC+8 (shift by immediate).
            let lhs = if rn == 15 {
                cpu.registers.read(rn) + 4
            } else {
                cpu.registers.read(rn)
            };

            // clock the register shift
            if $r_shift {
                cycles += 1;
                memory.on_internal_cycles(1);
            }

            // If S=1, Rd=R15; should not be used in user mode:
            //   CPSR = SPSR_<current mode>
            //   PC = result
            //   For example: MOVS PC,R14  ;return from SWI (PC=R14_svc, CPSR=SPSR_svc).
            if unlikely!(rd == 15) {
                let rhs = $get_operand(cpu, instr);
                $operation(cpu, lhs, rhs);
                let spsr = cpu.registers.read_spsr();
                cpu.registers.write_cpsr(spsr);
            } else {
                let rhs = $get_operand(cpu, instr);
                $operation(cpu, lhs, rhs);
            }

            if rd == 15 {
                if cpu.registers.getf_t() {
                    let dest = cpu.registers.read(15) & 0xFFFFFFFE;
                    cycles += cpu.thumb_branch_to(dest, memory);
                } else {
                    let dest = cpu.registers.read(15) & 0xFFFFFFFC;
                    cycles += cpu.arm_branch_to(dest, memory);
                }
            }

            return cycles;
        }
    };
}

// instr: AND
dataproc!(arm_and_lli, bs::lli, arm_alu_and, S_CLR, IMM_SHIFT);
dataproc!(arm_and_llr, bs::llr, arm_alu_and, S_CLR, REG_SHIFT);
dataproc!(arm_and_lri, bs::lri, arm_alu_and, S_CLR, IMM_SHIFT);
dataproc!(arm_and_lrr, bs::lrr, arm_alu_and, S_CLR, REG_SHIFT);
dataproc!(arm_and_ari, bs::ari, arm_alu_and, S_CLR, IMM_SHIFT);
dataproc!(arm_and_arr, bs::arr, arm_alu_and, S_CLR, REG_SHIFT);
dataproc!(arm_and_rri, bs::rri, arm_alu_and, S_CLR, IMM_SHIFT);
dataproc!(arm_and_rrr, bs::rrr, arm_alu_and, S_CLR, REG_SHIFT);
dataproc!(arm_and_imm, bs::imm, arm_alu_and, S_CLR, IMM_SHIFT);

// instr: ANDS
dataproc!(arm_ands_lli, bs::lli_s, arm_alu_ands, S_SET, IMM_SHIFT);
dataproc!(arm_ands_llr, bs::llr_s, arm_alu_ands, S_SET, REG_SHIFT);
dataproc!(arm_ands_lri, bs::lri_s, arm_alu_ands, S_SET, IMM_SHIFT);
dataproc!(arm_ands_lrr, bs::lrr_s, arm_alu_ands, S_SET, REG_SHIFT);
dataproc!(arm_ands_ari, bs::ari_s, arm_alu_ands, S_SET, IMM_SHIFT);
dataproc!(arm_ands_arr, bs::arr_s, arm_alu_ands, S_SET, REG_SHIFT);
dataproc!(arm_ands_rri, bs::rri_s, arm_alu_ands, S_SET, IMM_SHIFT);
dataproc!(arm_ands_rrr, bs::rrr_s, arm_alu_ands, S_SET, REG_SHIFT);
dataproc!(arm_ands_imm, bs::imm, arm_alu_ands, S_SET, IMM_SHIFT);

// instr: BIC
dataproc!(arm_bic_lli, bs::lli, arm_alu_bic, S_CLR, IMM_SHIFT);
dataproc!(arm_bic_llr, bs::llr, arm_alu_bic, S_CLR, REG_SHIFT);
dataproc!(arm_bic_lri, bs::lri, arm_alu_bic, S_CLR, IMM_SHIFT);
dataproc!(arm_bic_lrr, bs::lrr, arm_alu_bic, S_CLR, REG_SHIFT);
dataproc!(arm_bic_ari, bs::ari, arm_alu_bic, S_CLR, IMM_SHIFT);
dataproc!(arm_bic_arr, bs::arr, arm_alu_bic, S_CLR, REG_SHIFT);
dataproc!(arm_bic_rri, bs::rri, arm_alu_bic, S_CLR, IMM_SHIFT);
dataproc!(arm_bic_rrr, bs::rrr, arm_alu_bic, S_CLR, REG_SHIFT);
dataproc!(arm_bic_imm, bs::imm, arm_alu_bic, S_CLR, IMM_SHIFT);

// instr: BICS
dataproc!(arm_bics_lli, bs::lli_s, arm_alu_bics, S_SET, IMM_SHIFT);
dataproc!(arm_bics_llr, bs::llr_s, arm_alu_bics, S_SET, REG_SHIFT);
dataproc!(arm_bics_lri, bs::lri_s, arm_alu_bics, S_SET, IMM_SHIFT);
dataproc!(arm_bics_lrr, bs::lrr_s, arm_alu_bics, S_SET, REG_SHIFT);
dataproc!(arm_bics_ari, bs::ari_s, arm_alu_bics, S_SET, IMM_SHIFT);
dataproc!(arm_bics_arr, bs::arr_s, arm_alu_bics, S_SET, REG_SHIFT);
dataproc!(arm_bics_rri, bs::rri_s, arm_alu_bics, S_SET, IMM_SHIFT);
dataproc!(arm_bics_rrr, bs::rrr_s, arm_alu_bics, S_SET, REG_SHIFT);
dataproc!(arm_bics_imm, bs::imm, arm_alu_bics, S_SET, IMM_SHIFT);

// instr: ORR
dataproc!(arm_orr_lli, bs::lli, arm_alu_orr, S_CLR, IMM_SHIFT);
dataproc!(arm_orr_llr, bs::llr, arm_alu_orr, S_CLR, REG_SHIFT);
dataproc!(arm_orr_lri, bs::lri, arm_alu_orr, S_CLR, IMM_SHIFT);
dataproc!(arm_orr_lrr, bs::lrr, arm_alu_orr, S_CLR, REG_SHIFT);
dataproc!(arm_orr_ari, bs::ari, arm_alu_orr, S_CLR, IMM_SHIFT);
dataproc!(arm_orr_arr, bs::arr, arm_alu_orr, S_CLR, REG_SHIFT);
dataproc!(arm_orr_rri, bs::rri, arm_alu_orr, S_CLR, IMM_SHIFT);
dataproc!(arm_orr_rrr, bs::rrr, arm_alu_orr, S_CLR, REG_SHIFT);
dataproc!(arm_orr_imm, bs::imm, arm_alu_orr, S_CLR, IMM_SHIFT);

// instr: ORRS
dataproc!(arm_orrs_lli, bs::lli_s, arm_alu_orrs, S_SET, IMM_SHIFT);
dataproc!(arm_orrs_llr, bs::llr_s, arm_alu_orrs, S_SET, REG_SHIFT);
dataproc!(arm_orrs_lri, bs::lri_s, arm_alu_orrs, S_SET, IMM_SHIFT);
dataproc!(arm_orrs_lrr, bs::lrr_s, arm_alu_orrs, S_SET, REG_SHIFT);
dataproc!(arm_orrs_ari, bs::ari_s, arm_alu_orrs, S_SET, IMM_SHIFT);
dataproc!(arm_orrs_arr, bs::arr_s, arm_alu_orrs, S_SET, REG_SHIFT);
dataproc!(arm_orrs_rri, bs::rri_s, arm_alu_orrs, S_SET, IMM_SHIFT);
dataproc!(arm_orrs_rrr, bs::rrr_s, arm_alu_orrs, S_SET, REG_SHIFT);
dataproc!(arm_orrs_imm, bs::imm, arm_alu_orrs, S_SET, IMM_SHIFT);

// instr: EOR
dataproc!(arm_eor_lli, bs::lli, arm_alu_eor, S_CLR, IMM_SHIFT);
dataproc!(arm_eor_llr, bs::llr, arm_alu_eor, S_CLR, REG_SHIFT);
dataproc!(arm_eor_lri, bs::lri, arm_alu_eor, S_CLR, IMM_SHIFT);
dataproc!(arm_eor_lrr, bs::lrr, arm_alu_eor, S_CLR, REG_SHIFT);
dataproc!(arm_eor_ari, bs::ari, arm_alu_eor, S_CLR, IMM_SHIFT);
dataproc!(arm_eor_arr, bs::arr, arm_alu_eor, S_CLR, REG_SHIFT);
dataproc!(arm_eor_rri, bs::rri, arm_alu_eor, S_CLR, IMM_SHIFT);
dataproc!(arm_eor_rrr, bs::rrr, arm_alu_eor, S_CLR, REG_SHIFT);
dataproc!(arm_eor_imm, bs::imm, arm_alu_eor, S_CLR, IMM_SHIFT);

// instr: EORS
dataproc!(arm_eors_lli, bs::lli_s, arm_alu_eors, S_SET, IMM_SHIFT);
dataproc!(arm_eors_llr, bs::llr_s, arm_alu_eors, S_SET, REG_SHIFT);
dataproc!(arm_eors_lri, bs::lri_s, arm_alu_eors, S_SET, IMM_SHIFT);
dataproc!(arm_eors_lrr, bs::lrr_s, arm_alu_eors, S_SET, REG_SHIFT);
dataproc!(arm_eors_ari, bs::ari_s, arm_alu_eors, S_SET, IMM_SHIFT);
dataproc!(arm_eors_arr, bs::arr_s, arm_alu_eors, S_SET, REG_SHIFT);
dataproc!(arm_eors_rri, bs::rri_s, arm_alu_eors, S_SET, IMM_SHIFT);
dataproc!(arm_eors_rrr, bs::rrr_s, arm_alu_eors, S_SET, REG_SHIFT);
dataproc!(arm_eors_imm, bs::imm, arm_alu_eors, S_SET, IMM_SHIFT);

// instr: SUB
dataproc!(arm_sub_lli, bs::lli, arm_alu_sub, S_CLR, IMM_SHIFT);
dataproc!(arm_sub_llr, bs::llr, arm_alu_sub, S_CLR, REG_SHIFT);
dataproc!(arm_sub_lri, bs::lri, arm_alu_sub, S_CLR, IMM_SHIFT);
dataproc!(arm_sub_lrr, bs::lrr, arm_alu_sub, S_CLR, REG_SHIFT);
dataproc!(arm_sub_ari, bs::ari, arm_alu_sub, S_CLR, IMM_SHIFT);
dataproc!(arm_sub_arr, bs::arr, arm_alu_sub, S_CLR, REG_SHIFT);
dataproc!(arm_sub_rri, bs::rri, arm_alu_sub, S_CLR, IMM_SHIFT);
dataproc!(arm_sub_rrr, bs::rrr, arm_alu_sub, S_CLR, REG_SHIFT);
dataproc!(arm_sub_imm, bs::imm, arm_alu_sub, S_CLR, IMM_SHIFT);

// instr: SUBS
dataproc!(arm_subs_lli, bs::lli_s, arm_alu_subs, S_SET, IMM_SHIFT);
dataproc!(arm_subs_llr, bs::llr_s, arm_alu_subs, S_SET, REG_SHIFT);
dataproc!(arm_subs_lri, bs::lri_s, arm_alu_subs, S_SET, IMM_SHIFT);
dataproc!(arm_subs_lrr, bs::lrr_s, arm_alu_subs, S_SET, REG_SHIFT);
dataproc!(arm_subs_ari, bs::ari_s, arm_alu_subs, S_SET, IMM_SHIFT);
dataproc!(arm_subs_arr, bs::arr_s, arm_alu_subs, S_SET, REG_SHIFT);
dataproc!(arm_subs_rri, bs::rri_s, arm_alu_subs, S_SET, IMM_SHIFT);
dataproc!(arm_subs_rrr, bs::rrr_s, arm_alu_subs, S_SET, REG_SHIFT);
dataproc!(arm_subs_imm, bs::imm, arm_alu_subs, S_SET, IMM_SHIFT);

// instr: RSB
dataproc!(arm_rsb_lli, bs::lli, arm_alu_rsb, S_CLR, IMM_SHIFT);
dataproc!(arm_rsb_llr, bs::llr, arm_alu_rsb, S_CLR, REG_SHIFT);
dataproc!(arm_rsb_lri, bs::lri, arm_alu_rsb, S_CLR, IMM_SHIFT);
dataproc!(arm_rsb_lrr, bs::lrr, arm_alu_rsb, S_CLR, REG_SHIFT);
dataproc!(arm_rsb_ari, bs::ari, arm_alu_rsb, S_CLR, IMM_SHIFT);
dataproc!(arm_rsb_arr, bs::arr, arm_alu_rsb, S_CLR, REG_SHIFT);
dataproc!(arm_rsb_rri, bs::rri, arm_alu_rsb, S_CLR, IMM_SHIFT);
dataproc!(arm_rsb_rrr, bs::rrr, arm_alu_rsb, S_CLR, REG_SHIFT);
dataproc!(arm_rsb_imm, bs::imm, arm_alu_rsb, S_CLR, IMM_SHIFT);

// instr: RSBS
dataproc!(arm_rsbs_lli, bs::lli_s, arm_alu_rsbs, S_SET, IMM_SHIFT);
dataproc!(arm_rsbs_llr, bs::llr_s, arm_alu_rsbs, S_SET, REG_SHIFT);
dataproc!(arm_rsbs_lri, bs::lri_s, arm_alu_rsbs, S_SET, IMM_SHIFT);
dataproc!(arm_rsbs_lrr, bs::lrr_s, arm_alu_rsbs, S_SET, REG_SHIFT);
dataproc!(arm_rsbs_ari, bs::ari_s, arm_alu_rsbs, S_SET, IMM_SHIFT);
dataproc!(arm_rsbs_arr, bs::arr_s, arm_alu_rsbs, S_SET, REG_SHIFT);
dataproc!(arm_rsbs_rri, bs::rri_s, arm_alu_rsbs, S_SET, IMM_SHIFT);
dataproc!(arm_rsbs_rrr, bs::rrr_s, arm_alu_rsbs, S_SET, REG_SHIFT);
dataproc!(arm_rsbs_imm, bs::imm, arm_alu_rsbs, S_SET, IMM_SHIFT);

// instr: ADD
dataproc!(arm_add_lli, bs::lli, arm_alu_add, S_CLR, IMM_SHIFT);
dataproc!(arm_add_llr, bs::llr, arm_alu_add, S_CLR, REG_SHIFT);
dataproc!(arm_add_lri, bs::lri, arm_alu_add, S_CLR, IMM_SHIFT);
dataproc!(arm_add_lrr, bs::lrr, arm_alu_add, S_CLR, REG_SHIFT);
dataproc!(arm_add_ari, bs::ari, arm_alu_add, S_CLR, IMM_SHIFT);
dataproc!(arm_add_arr, bs::arr, arm_alu_add, S_CLR, REG_SHIFT);
dataproc!(arm_add_rri, bs::rri, arm_alu_add, S_CLR, IMM_SHIFT);
dataproc!(arm_add_rrr, bs::rrr, arm_alu_add, S_CLR, REG_SHIFT);
dataproc!(arm_add_imm, bs::imm, arm_alu_add, S_CLR, IMM_SHIFT);

// instr: ADDS
dataproc!(arm_adds_lli, bs::lli_s, arm_alu_adds, S_SET, IMM_SHIFT);
dataproc!(arm_adds_llr, bs::llr_s, arm_alu_adds, S_SET, REG_SHIFT);
dataproc!(arm_adds_lri, bs::lri_s, arm_alu_adds, S_SET, IMM_SHIFT);
dataproc!(arm_adds_lrr, bs::lrr_s, arm_alu_adds, S_SET, REG_SHIFT);
dataproc!(arm_adds_ari, bs::ari_s, arm_alu_adds, S_SET, IMM_SHIFT);
dataproc!(arm_adds_arr, bs::arr_s, arm_alu_adds, S_SET, REG_SHIFT);
dataproc!(arm_adds_rri, bs::rri_s, arm_alu_adds, S_SET, IMM_SHIFT);
dataproc!(arm_adds_rrr, bs::rrr_s, arm_alu_adds, S_SET, REG_SHIFT);
dataproc!(arm_adds_imm, bs::imm, arm_alu_adds, S_SET, IMM_SHIFT);

// instr: ADC
dataproc!(arm_adc_lli, bs::lli, arm_alu_adc, S_CLR, IMM_SHIFT);
dataproc!(arm_adc_llr, bs::llr, arm_alu_adc, S_CLR, REG_SHIFT);
dataproc!(arm_adc_lri, bs::lri, arm_alu_adc, S_CLR, IMM_SHIFT);
dataproc!(arm_adc_lrr, bs::lrr, arm_alu_adc, S_CLR, REG_SHIFT);
dataproc!(arm_adc_ari, bs::ari, arm_alu_adc, S_CLR, IMM_SHIFT);
dataproc!(arm_adc_arr, bs::arr, arm_alu_adc, S_CLR, REG_SHIFT);
dataproc!(arm_adc_rri, bs::rri, arm_alu_adc, S_CLR, IMM_SHIFT);
dataproc!(arm_adc_rrr, bs::rrr, arm_alu_adc, S_CLR, REG_SHIFT);
dataproc!(arm_adc_imm, bs::imm, arm_alu_adc, S_CLR, IMM_SHIFT);

// instr: ADCS
dataproc!(arm_adcs_lli, bs::lli_s, arm_alu_adcs, S_SET, IMM_SHIFT);
dataproc!(arm_adcs_llr, bs::llr_s, arm_alu_adcs, S_SET, REG_SHIFT);
dataproc!(arm_adcs_lri, bs::lri_s, arm_alu_adcs, S_SET, IMM_SHIFT);
dataproc!(arm_adcs_lrr, bs::lrr_s, arm_alu_adcs, S_SET, REG_SHIFT);
dataproc!(arm_adcs_ari, bs::ari_s, arm_alu_adcs, S_SET, IMM_SHIFT);
dataproc!(arm_adcs_arr, bs::arr_s, arm_alu_adcs, S_SET, REG_SHIFT);
dataproc!(arm_adcs_rri, bs::rri_s, arm_alu_adcs, S_SET, IMM_SHIFT);
dataproc!(arm_adcs_rrr, bs::rrr_s, arm_alu_adcs, S_SET, REG_SHIFT);
dataproc!(arm_adcs_imm, bs::imm, arm_alu_adcs, S_SET, IMM_SHIFT);

// instr: SBC
dataproc!(arm_sbc_lli, bs::lli, arm_alu_sbc, S_CLR, IMM_SHIFT);
dataproc!(arm_sbc_llr, bs::llr, arm_alu_sbc, S_CLR, REG_SHIFT);
dataproc!(arm_sbc_lri, bs::lri, arm_alu_sbc, S_CLR, IMM_SHIFT);
dataproc!(arm_sbc_lrr, bs::lrr, arm_alu_sbc, S_CLR, REG_SHIFT);
dataproc!(arm_sbc_ari, bs::ari, arm_alu_sbc, S_CLR, IMM_SHIFT);
dataproc!(arm_sbc_arr, bs::arr, arm_alu_sbc, S_CLR, REG_SHIFT);
dataproc!(arm_sbc_rri, bs::rri, arm_alu_sbc, S_CLR, IMM_SHIFT);
dataproc!(arm_sbc_rrr, bs::rrr, arm_alu_sbc, S_CLR, REG_SHIFT);
dataproc!(arm_sbc_imm, bs::imm, arm_alu_sbc, S_CLR, IMM_SHIFT);

// instr: SBCS
dataproc!(arm_sbcs_lli, bs::lli_s, arm_alu_sbcs, S_SET, IMM_SHIFT);
dataproc!(arm_sbcs_llr, bs::llr_s, arm_alu_sbcs, S_SET, REG_SHIFT);
dataproc!(arm_sbcs_lri, bs::lri_s, arm_alu_sbcs, S_SET, IMM_SHIFT);
dataproc!(arm_sbcs_lrr, bs::lrr_s, arm_alu_sbcs, S_SET, REG_SHIFT);
dataproc!(arm_sbcs_ari, bs::ari_s, arm_alu_sbcs, S_SET, IMM_SHIFT);
dataproc!(arm_sbcs_arr, bs::arr_s, arm_alu_sbcs, S_SET, REG_SHIFT);
dataproc!(arm_sbcs_rri, bs::rri_s, arm_alu_sbcs, S_SET, IMM_SHIFT);
dataproc!(arm_sbcs_rrr, bs::rrr_s, arm_alu_sbcs, S_SET, REG_SHIFT);
dataproc!(arm_sbcs_imm, bs::imm, arm_alu_sbcs, S_SET, IMM_SHIFT);

// instr: RSC
dataproc!(arm_rsc_lli, bs::lli, arm_alu_rsc, S_CLR, IMM_SHIFT);
dataproc!(arm_rsc_llr, bs::llr, arm_alu_rsc, S_CLR, REG_SHIFT);
dataproc!(arm_rsc_lri, bs::lri, arm_alu_rsc, S_CLR, IMM_SHIFT);
dataproc!(arm_rsc_lrr, bs::lrr, arm_alu_rsc, S_CLR, REG_SHIFT);
dataproc!(arm_rsc_ari, bs::ari, arm_alu_rsc, S_CLR, IMM_SHIFT);
dataproc!(arm_rsc_arr, bs::arr, arm_alu_rsc, S_CLR, REG_SHIFT);
dataproc!(arm_rsc_rri, bs::rri, arm_alu_rsc, S_CLR, IMM_SHIFT);
dataproc!(arm_rsc_rrr, bs::rrr, arm_alu_rsc, S_CLR, REG_SHIFT);
dataproc!(arm_rsc_imm, bs::imm, arm_alu_rsc, S_CLR, IMM_SHIFT);

// instr: RSCS
dataproc!(arm_rscs_lli, bs::lli_s, arm_alu_rscs, S_SET, IMM_SHIFT);
dataproc!(arm_rscs_llr, bs::llr_s, arm_alu_rscs, S_SET, REG_SHIFT);
dataproc!(arm_rscs_lri, bs::lri_s, arm_alu_rscs, S_SET, IMM_SHIFT);
dataproc!(arm_rscs_lrr, bs::lrr_s, arm_alu_rscs, S_SET, REG_SHIFT);
dataproc!(arm_rscs_ari, bs::ari_s, arm_alu_rscs, S_SET, IMM_SHIFT);
dataproc!(arm_rscs_arr, bs::arr_s, arm_alu_rscs, S_SET, REG_SHIFT);
dataproc!(arm_rscs_rri, bs::rri_s, arm_alu_rscs, S_SET, IMM_SHIFT);
dataproc!(arm_rscs_rrr, bs::rrr_s, arm_alu_rscs, S_SET, REG_SHIFT);
dataproc!(arm_rscs_imm, bs::imm, arm_alu_rscs, S_SET, IMM_SHIFT);

// instr: MOV
dataproc!(arm_mov_lli, bs::lli, arm_alu_mov, S_CLR, IMM_SHIFT);
dataproc!(arm_mov_llr, bs::llr, arm_alu_mov, S_CLR, REG_SHIFT);
dataproc!(arm_mov_lri, bs::lri, arm_alu_mov, S_CLR, IMM_SHIFT);
dataproc!(arm_mov_lrr, bs::lrr, arm_alu_mov, S_CLR, REG_SHIFT);
dataproc!(arm_mov_ari, bs::ari, arm_alu_mov, S_CLR, IMM_SHIFT);
dataproc!(arm_mov_arr, bs::arr, arm_alu_mov, S_CLR, REG_SHIFT);
dataproc!(arm_mov_rri, bs::rri, arm_alu_mov, S_CLR, IMM_SHIFT);
dataproc!(arm_mov_rrr, bs::rrr, arm_alu_mov, S_CLR, REG_SHIFT);
dataproc!(arm_mov_imm, bs::imm, arm_alu_mov, S_CLR, IMM_SHIFT);

// instr: MOVS
dataproc!(arm_movs_lli, bs::lli_s, arm_alu_movs, S_SET, IMM_SHIFT);
dataproc!(arm_movs_llr, bs::llr_s, arm_alu_movs, S_SET, REG_SHIFT);
dataproc!(arm_movs_lri, bs::lri_s, arm_alu_movs, S_SET, IMM_SHIFT);
dataproc!(arm_movs_lrr, bs::lrr_s, arm_alu_movs, S_SET, REG_SHIFT);
dataproc!(arm_movs_ari, bs::ari_s, arm_alu_movs, S_SET, IMM_SHIFT);
dataproc!(arm_movs_arr, bs::arr_s, arm_alu_movs, S_SET, REG_SHIFT);
dataproc!(arm_movs_rri, bs::rri_s, arm_alu_movs, S_SET, IMM_SHIFT);
dataproc!(arm_movs_rrr, bs::rrr_s, arm_alu_movs, S_SET, REG_SHIFT);
dataproc!(arm_movs_imm, bs::imm, arm_alu_movs, S_SET, IMM_SHIFT);

// instr: MVN
dataproc!(arm_mvn_lli, bs::lli, arm_alu_mvn, S_CLR, IMM_SHIFT);
dataproc!(arm_mvn_llr, bs::llr, arm_alu_mvn, S_CLR, REG_SHIFT);
dataproc!(arm_mvn_lri, bs::lri, arm_alu_mvn, S_CLR, IMM_SHIFT);
dataproc!(arm_mvn_lrr, bs::lrr, arm_alu_mvn, S_CLR, REG_SHIFT);
dataproc!(arm_mvn_ari, bs::ari, arm_alu_mvn, S_CLR, IMM_SHIFT);
dataproc!(arm_mvn_arr, bs::arr, arm_alu_mvn, S_CLR, REG_SHIFT);
dataproc!(arm_mvn_rri, bs::rri, arm_alu_mvn, S_CLR, IMM_SHIFT);
dataproc!(arm_mvn_rrr, bs::rrr, arm_alu_mvn, S_CLR, REG_SHIFT);
dataproc!(arm_mvn_imm, bs::imm, arm_alu_mvn, S_CLR, IMM_SHIFT);

// instr: MVNS
dataproc!(arm_mvns_lli, bs::lli_s, arm_alu_mvns, S_SET, IMM_SHIFT);
dataproc!(arm_mvns_llr, bs::llr_s, arm_alu_mvns, S_SET, REG_SHIFT);
dataproc!(arm_mvns_lri, bs::lri_s, arm_alu_mvns, S_SET, IMM_SHIFT);
dataproc!(arm_mvns_lrr, bs::lrr_s, arm_alu_mvns, S_SET, REG_SHIFT);
dataproc!(arm_mvns_ari, bs::ari_s, arm_alu_mvns, S_SET, IMM_SHIFT);
dataproc!(arm_mvns_arr, bs::arr_s, arm_alu_mvns, S_SET, REG_SHIFT);
dataproc!(arm_mvns_rri, bs::rri_s, arm_alu_mvns, S_SET, IMM_SHIFT);
dataproc!(arm_mvns_rrr, bs::rrr_s, arm_alu_mvns, S_SET, REG_SHIFT);
dataproc!(arm_mvns_imm, bs::imm, arm_alu_mvns, S_SET, IMM_SHIFT);

// instr: CMPS
dataproc_no_write!(arm_cmps_lli, bs::lli_s, arm_alu_cmps, IMM_SHIFT);
dataproc_no_write!(arm_cmps_llr, bs::llr_s, arm_alu_cmps, REG_SHIFT);
dataproc_no_write!(arm_cmps_lri, bs::lri_s, arm_alu_cmps, IMM_SHIFT);
dataproc_no_write!(arm_cmps_lrr, bs::lrr_s, arm_alu_cmps, REG_SHIFT);
dataproc_no_write!(arm_cmps_ari, bs::ari_s, arm_alu_cmps, IMM_SHIFT);
dataproc_no_write!(arm_cmps_arr, bs::arr_s, arm_alu_cmps, REG_SHIFT);
dataproc_no_write!(arm_cmps_rri, bs::rri_s, arm_alu_cmps, IMM_SHIFT);
dataproc_no_write!(arm_cmps_rrr, bs::rrr_s, arm_alu_cmps, REG_SHIFT);
dataproc_no_write!(arm_cmps_imm, bs::imm, arm_alu_cmps, IMM_SHIFT);

// instr: CMNS
dataproc_no_write!(arm_cmns_lli, bs::lli_s, arm_alu_cmns, IMM_SHIFT);
dataproc_no_write!(arm_cmns_llr, bs::llr_s, arm_alu_cmns, REG_SHIFT);
dataproc_no_write!(arm_cmns_lri, bs::lri_s, arm_alu_cmns, IMM_SHIFT);
dataproc_no_write!(arm_cmns_lrr, bs::lrr_s, arm_alu_cmns, REG_SHIFT);
dataproc_no_write!(arm_cmns_ari, bs::ari_s, arm_alu_cmns, IMM_SHIFT);
dataproc_no_write!(arm_cmns_arr, bs::arr_s, arm_alu_cmns, REG_SHIFT);
dataproc_no_write!(arm_cmns_rri, bs::rri_s, arm_alu_cmns, IMM_SHIFT);
dataproc_no_write!(arm_cmns_rrr, bs::rrr_s, arm_alu_cmns, REG_SHIFT);
dataproc_no_write!(arm_cmns_imm, bs::imm, arm_alu_cmns, IMM_SHIFT);

// instr: TEQS
dataproc_no_write!(arm_teqs_lli, bs::lli_s, arm_alu_teqs, IMM_SHIFT);
dataproc_no_write!(arm_teqs_llr, bs::llr_s, arm_alu_teqs, REG_SHIFT);
dataproc_no_write!(arm_teqs_lri, bs::lri_s, arm_alu_teqs, IMM_SHIFT);
dataproc_no_write!(arm_teqs_lrr, bs::lrr_s, arm_alu_teqs, REG_SHIFT);
dataproc_no_write!(arm_teqs_ari, bs::ari_s, arm_alu_teqs, IMM_SHIFT);
dataproc_no_write!(arm_teqs_arr, bs::arr_s, arm_alu_teqs, REG_SHIFT);
dataproc_no_write!(arm_teqs_rri, bs::rri_s, arm_alu_teqs, IMM_SHIFT);
dataproc_no_write!(arm_teqs_rrr, bs::rrr_s, arm_alu_teqs, REG_SHIFT);
dataproc_no_write!(arm_teqs_imm, bs::imm, arm_alu_teqs, IMM_SHIFT);

// instr: TSTS
dataproc_no_write!(arm_tsts_lli, bs::lli_s, arm_alu_tsts, IMM_SHIFT);
dataproc_no_write!(arm_tsts_llr, bs::llr_s, arm_alu_tsts, true);
dataproc_no_write!(arm_tsts_lri, bs::lri_s, arm_alu_tsts, IMM_SHIFT);
dataproc_no_write!(arm_tsts_lrr, bs::lrr_s, arm_alu_tsts, true);
dataproc_no_write!(arm_tsts_ari, bs::ari_s, arm_alu_tsts, IMM_SHIFT);
dataproc_no_write!(arm_tsts_arr, bs::arr_s, arm_alu_tsts, true);
dataproc_no_write!(arm_tsts_rri, bs::rri_s, arm_alu_tsts, IMM_SHIFT);
dataproc_no_write!(arm_tsts_rrr, bs::rrr_s, arm_alu_tsts, true);
dataproc_no_write!(arm_tsts_imm, bs::imm, arm_alu_tsts, IMM_SHIFT);
