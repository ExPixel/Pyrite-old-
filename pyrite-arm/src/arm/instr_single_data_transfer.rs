use super::super::{ ArmCpu, ArmMemory, registers::CpuMode };
use super::super::alu::bs::{lli, lri, ari, rri};

const LOAD: bool = true;
const STORE: bool = false;

const POST: bool = false;
const PRE:  bool = true;

const DEC: bool = false;
const INC: bool = true;

const WRITEBACK: bool = true;
const NO_WRITEBACK: bool = false;

/// This is actually the writeback bit but only during post-indexed load/store instructions.
const USER_MODE: bool = true;
const NO_USER_MODE: bool = false;

// #TODO handle data aborts by correctly jumping to the data abort exception vector.
macro_rules! arm_gen_sdt {
    ($name:ident, $transfer:expr, $transfer_type:expr, $data_size:expr, $get_offset:expr, $direction:expr, $indexing:expr, $writeback:expr, $user_mode:expr) => (
        pub fn $name(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, instr: u32) {
            cpu.arm_prefetch(memory);

            let rd = bits!(instr, 12, 15);
            let rn = bits!(instr, 16, 19);

            // switch to user mode if the "T Bit" is set.
            let original_mode = cpu.registers.read_mode();
            if $user_mode && original_mode != CpuMode::User {
                cpu.registers.write_mode(CpuMode::User);
            }

            let offset = $get_offset(cpu, instr);
            let mut addr = cpu.registers.read(rn);

            // pre-indexing
            if $indexing == PRE {
                if $direction == INC {
                    addr = addr.wrapping_add(offset);
                } else {
                    addr = addr.wrapping_sub(offset);
                }
            }

            // writeback to base register
            // post-indexing as well
            if $writeback {
                let writeback_addr = if $indexing == POST {
                    if $direction == INC {
                        addr.wrapping_add(offset)
                    } else {
                        addr.wrapping_sub(offset)
                    }
                } else {
                    addr
                };
                cpu.registers.write(rn, writeback_addr);
            }

            $transfer(cpu, memory, rd, addr);

            // Switch back to our original mode if the "T Bit" is set and we weren't originally in
            // user mode.
            if $user_mode && original_mode != CpuMode::User {
                cpu.registers.write_mode(original_mode);
            }

            if $transfer_type == LOAD {
                if rd == 15 || ($writeback == WRITEBACK && rn == 15) {
                    let dest_pc = cpu.registers.read(15);
                    cpu.arm_branch_to(dest_pc, memory);
                }
            }
        }
    );

    ($name:ident, $transfer:expr, $transfer_type:expr, $data_size:expr, $get_offset:expr, $direction:expr, $indexing:expr, $writeback:expr) => (
        arm_gen_sdt!($name, $transfer, $transfer_type, $data_size, $get_offset, $direction, $indexing, $writeback, NO_USER_MODE);
    );
}

#[inline(always)]
fn sdt_ldr(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, rd: u32, addr: u32) {
    // From the ARM7TDMI Documentation:
    //  A word load will normally use a word aligned address, however,
    //  an address offset from the word boundary will cause the data to
    //  be rotated into the register so that the addressed byte occupies bit 0-7.
    // Basically we rotate the word to the right by the number of bits that the address
    // is unaligned (offset from the word boundary).
    let value = memory.read_data_word(addr & 0xFFFFFFFC, false, &mut cpu.cycles).rotate_right(8 * (addr % 4));
    cpu.registers.write(rd, value);
}

#[inline(always)]
fn sdt_ldrb(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, rd: u32, addr: u32) {
    let value = memory.read_data_byte(addr, false, &mut cpu.cycles);
    cpu.registers.write(rd, value as u32);
}

#[inline(always)]
fn sdt_str(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, rd: u32, addr: u32) {
    let mut value = cpu.registers.read(rd);
    // If the Program Counter is used as the source register in a word store, it will be 12 bytes
    // ahead instead of 8 when read.
    if rd == 15 { value = value.wrapping_add(4); }
    memory.write_data_word(addr & 0xFFFFFFFC, value, false, &mut cpu.cycles);
}

#[inline(always)]
fn sdt_strb(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, rd: u32, addr: u32) {
    let mut value = cpu.registers.read(rd);
    // If the Program Counter is used as the source register in a byte store, it will be 12 bytes
    // ahead instead of 8 when read.
    if rd == 15 { value = value.wrapping_add(4); }
    memory.write_data_byte(addr, (value & 0xFF) as u8, false, &mut cpu.cycles);
}

#[inline(always)]
fn imm(_cpu: &mut ArmCpu, instr: u32) -> u32 {
    instr & 0xFFF
}

// Load word, Negative immediate offset
arm_gen_sdt!(arm_ldr_ofim, sdt_ldr, LOAD, 32, imm, DEC, PRE, NO_WRITEBACK);

// Load word, Positive immediate offset
arm_gen_sdt!(arm_ldr_ofip, sdt_ldr, LOAD, 32, imm, INC, PRE, NO_WRITEBACK);

// Load word, Negative arithmetic-right-shifted register offset
arm_gen_sdt!(arm_ldr_ofrmar, sdt_ldr, LOAD, 32, ari, DEC, PRE, NO_WRITEBACK);

// Load word, Negative left-shifted register offset
arm_gen_sdt!(arm_ldr_ofrmll, sdt_ldr, LOAD, 32, lli, DEC, PRE, NO_WRITEBACK);

// Load word, Negative right-shifted register offset
arm_gen_sdt!(arm_ldr_ofrmlr, sdt_ldr, LOAD, 32, lri, DEC, PRE, NO_WRITEBACK);

// Load word, Negative right-rotated register offset
arm_gen_sdt!(arm_ldr_ofrmrr, sdt_ldr, LOAD, 32, rri, DEC, PRE, NO_WRITEBACK);

// Load word, Positive arithmetic-right-shifted register offset
arm_gen_sdt!(arm_ldr_ofrpar, sdt_ldr, LOAD, 32, ari, INC, PRE, NO_WRITEBACK);

// Load word, Positive left-shifted register offset
arm_gen_sdt!(arm_ldr_ofrpll, sdt_ldr, LOAD, 32, lli, INC, PRE, NO_WRITEBACK);

// Load word, Positive right-shifted register offset
arm_gen_sdt!(arm_ldr_ofrplr, sdt_ldr, LOAD, 32, lri, INC, PRE, NO_WRITEBACK);

// Load word, Positive right-rotated register offset
arm_gen_sdt!(arm_ldr_ofrprr, sdt_ldr, LOAD, 32, rri, INC, PRE, NO_WRITEBACK);

// Load word, Immediate offset, pre-decrement
arm_gen_sdt!(arm_ldr_prim, sdt_ldr, LOAD, 32, imm, DEC, PRE, WRITEBACK);

// Load word, Immediate offset, pre-increment
arm_gen_sdt!(arm_ldr_prip, sdt_ldr, LOAD, 32, imm, INC, PRE, WRITEBACK);

// Load word, Arithmetic-right-shifted register offset, pre-decrement
arm_gen_sdt!(arm_ldr_prrmar, sdt_ldr, LOAD, 32, ari, DEC, PRE, WRITEBACK);

// Load word, Left-shifted register offset, pre-decrement
arm_gen_sdt!(arm_ldr_prrmll, sdt_ldr, LOAD, 32, lli, DEC, PRE, WRITEBACK);

// Load word, Right-shifted register offset, pre-decrement
arm_gen_sdt!(arm_ldr_prrmlr, sdt_ldr, LOAD, 32, lri, DEC, PRE, WRITEBACK);

// Load word, Right-rotated register offset, pre-decrement
arm_gen_sdt!(arm_ldr_prrmrr, sdt_ldr, LOAD, 32, rri, DEC, PRE, WRITEBACK);

// Load word, Arithmetic-right-shifted register offset, pre-increment
arm_gen_sdt!(arm_ldr_prrpar, sdt_ldr, LOAD, 32, ari, INC, PRE, WRITEBACK);

// Load word, Left-shifted register offset, pre-increment
arm_gen_sdt!(arm_ldr_prrpll, sdt_ldr, LOAD, 32, lli, INC, PRE, WRITEBACK);

// Load word, Right-shifted register offset, pre-increment
arm_gen_sdt!(arm_ldr_prrplr, sdt_ldr, LOAD, 32, lri, INC, PRE, WRITEBACK);

// Load word, Right-rotated register offset, pre-increment
arm_gen_sdt!(arm_ldr_prrprr, sdt_ldr, LOAD, 32, rri, INC, PRE, WRITEBACK);

// Load word, Immediate offset, post-decrement
arm_gen_sdt!(arm_ldr_ptim, sdt_ldr, LOAD, 32, imm, DEC, POST, WRITEBACK);

// Load word, Immediate offset, post-increment
arm_gen_sdt!(arm_ldr_ptip, sdt_ldr, LOAD, 32, imm, INC, POST, WRITEBACK);

// Load word, Arithmetic-right-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldr_ptrmar, sdt_ldr, LOAD, 32, ari, DEC, POST, WRITEBACK);

// Load word, Left-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldr_ptrmll, sdt_ldr, LOAD, 32, lli, DEC, POST, WRITEBACK);

// Load word, Right-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldr_ptrmlr, sdt_ldr, LOAD, 32, lri, DEC, POST, WRITEBACK);

// Load word, Right-rotated register offset, post-decrement
arm_gen_sdt!(arm_ldr_ptrmrr, sdt_ldr, LOAD, 32, rri, DEC, POST, WRITEBACK);

// Load word, Arithmetic-right-shifted register offset, post-increment
arm_gen_sdt!(arm_ldr_ptrpar, sdt_ldr, LOAD, 32, ari, INC, POST, WRITEBACK);

// Load word, Left-shifted register offset, post-increment
arm_gen_sdt!(arm_ldr_ptrpll, sdt_ldr, LOAD, 32, lli, INC, POST, WRITEBACK);

// Load word, Right-shifted register offset, post-increment
arm_gen_sdt!(arm_ldr_ptrplr, sdt_ldr, LOAD, 32, lri, INC, POST, WRITEBACK);

// Load word, Right-rotated register offset, post-increment
arm_gen_sdt!(arm_ldr_ptrprr, sdt_ldr, LOAD, 32, rri, INC, POST, WRITEBACK);

// Load byte, Negative immediate offset
arm_gen_sdt!(arm_ldrb_ofim, sdt_ldrb, LOAD, 8, imm, DEC, PRE, NO_WRITEBACK);

// Load byte, Positive immediate offset
arm_gen_sdt!(arm_ldrb_ofip, sdt_ldrb, LOAD, 8, imm, INC, PRE, NO_WRITEBACK);

// Load byte, Negative arithmetic-right-shifted register offset
arm_gen_sdt!(arm_ldrb_ofrmar, sdt_ldrb, LOAD, 8, ari, DEC, PRE, NO_WRITEBACK);

// Load byte, Negative left-shifted register offset
arm_gen_sdt!(arm_ldrb_ofrmll, sdt_ldrb, LOAD, 8, lli, DEC, PRE, NO_WRITEBACK);

// Load byte, Negative right-shifted register offset
arm_gen_sdt!(arm_ldrb_ofrmlr, sdt_ldrb, LOAD, 8, lri, DEC, PRE, NO_WRITEBACK);

// Load byte, Negative right-rotated register offset
arm_gen_sdt!(arm_ldrb_ofrmrr, sdt_ldrb, LOAD, 8, rri, DEC, PRE, NO_WRITEBACK);

// Load byte, Positive arithmetic-right-shifted register offset
arm_gen_sdt!(arm_ldrb_ofrpar, sdt_ldrb, LOAD, 8, ari, INC, PRE, NO_WRITEBACK);

// Load byte, Positive left-shifted register offset
arm_gen_sdt!(arm_ldrb_ofrpll, sdt_ldrb, LOAD, 8, lli, INC, PRE, NO_WRITEBACK);

// Load byte, Positive right-shifted register offset
arm_gen_sdt!(arm_ldrb_ofrplr, sdt_ldrb, LOAD, 8, lri, INC, PRE, NO_WRITEBACK);

// Load byte, Positive right-rotated register offset
arm_gen_sdt!(arm_ldrb_ofrprr, sdt_ldrb, LOAD, 8, rri, INC, PRE, NO_WRITEBACK);

// Load byte, Immediate offset, pre-decrement
arm_gen_sdt!(arm_ldrb_prim, sdt_ldrb, LOAD, 8, imm, DEC, PRE, WRITEBACK);

// Load byte, Immediate offset, pre-increment
arm_gen_sdt!(arm_ldrb_prip, sdt_ldrb, LOAD, 8, imm, INC, PRE, WRITEBACK);

// Load byte, Arithmetic-right-shifted register offset, pre-decrement
arm_gen_sdt!(arm_ldrb_prrmar, sdt_ldrb, LOAD, 8, ari, DEC, PRE, WRITEBACK);

// Load byte, Left-shifted register offset, pre-decrement
arm_gen_sdt!(arm_ldrb_prrmll, sdt_ldrb, LOAD, 8, lli, DEC, PRE, WRITEBACK);

// Load byte, Right-shifted register offset, pre-decrement
arm_gen_sdt!(arm_ldrb_prrmlr, sdt_ldrb, LOAD, 8, lri, DEC, PRE, WRITEBACK);

// Load byte, Right-rotated register offset, pre-decrement
arm_gen_sdt!(arm_ldrb_prrmrr, sdt_ldrb, LOAD, 8, rri, DEC, PRE, WRITEBACK);

// Load byte, Arithmetic-right-shifted register offset, pre-increment
arm_gen_sdt!(arm_ldrb_prrpar, sdt_ldrb, LOAD, 8, ari, INC, PRE, WRITEBACK);

// Load byte, Left-shifted register offset, pre-increment
arm_gen_sdt!(arm_ldrb_prrpll, sdt_ldrb, LOAD, 8, lli, INC, PRE, WRITEBACK);

// Load byte, Right-shifted register offset, pre-increment
arm_gen_sdt!(arm_ldrb_prrplr, sdt_ldrb, LOAD, 8, lri, INC, PRE, WRITEBACK);

// Load byte, Right-rotated register offset, pre-increment
arm_gen_sdt!(arm_ldrb_prrprr, sdt_ldrb, LOAD, 8, rri, INC, PRE, WRITEBACK);

// Load byte, Immediate offset, post-decrement
arm_gen_sdt!(arm_ldrb_ptim, sdt_ldrb, LOAD, 8, imm, DEC, POST, WRITEBACK);

// Load byte, Immediate offset, post-increment
arm_gen_sdt!(arm_ldrb_ptip, sdt_ldrb, LOAD, 8, imm, INC, POST, WRITEBACK);

// Load byte, Arithmetic-right-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldrb_ptrmar, sdt_ldrb, LOAD, 8, ari, DEC, POST, WRITEBACK);

// Load byte, Left-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldrb_ptrmll, sdt_ldrb, LOAD, 8, lli, DEC, POST, WRITEBACK);

// Load byte, Right-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldrb_ptrmlr, sdt_ldrb, LOAD, 8, lri, DEC, POST, WRITEBACK);

// Load byte, Right-rotated register offset, post-decrement
arm_gen_sdt!(arm_ldrb_ptrmrr, sdt_ldrb, LOAD, 8, rri, DEC, POST, WRITEBACK);

// Load byte, Arithmetic-right-shifted register offset, post-increment
arm_gen_sdt!(arm_ldrb_ptrpar, sdt_ldrb, LOAD, 8, ari, INC, POST, WRITEBACK);

// Load byte, Left-shifted register offset, post-increment
arm_gen_sdt!(arm_ldrb_ptrpll, sdt_ldrb, LOAD, 8, lli, INC, POST, WRITEBACK);

// Load byte, Right-shifted register offset, post-increment
arm_gen_sdt!(arm_ldrb_ptrplr, sdt_ldrb, LOAD, 8, lri, INC, POST, WRITEBACK);

// Load byte, Right-rotated register offset, post-increment
arm_gen_sdt!(arm_ldrb_ptrprr, sdt_ldrb, LOAD, 8, rri, INC, POST, WRITEBACK);

// Load byte into user-mode register, Immediate offset, post-decrement
arm_gen_sdt!(arm_ldrbt_ptim, sdt_ldrb, LOAD, 8, imm, DEC, POST, WRITEBACK, USER_MODE);

// Load byte into user-mode register, Immediate offset, post-increment
arm_gen_sdt!(arm_ldrbt_ptip, sdt_ldrb, LOAD, 8, imm, INC, POST, WRITEBACK, USER_MODE);

// Load byte into user-mode register, Arithmetic-right-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldrbt_ptrmar, sdt_ldrb, LOAD, 8, ari, DEC, POST, WRITEBACK, USER_MODE);

// Load byte into user-mode register, Left-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldrbt_ptrmll, sdt_ldrb, LOAD, 8, lli, DEC, POST, WRITEBACK, USER_MODE);

// Load byte into user-mode register, Right-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldrbt_ptrmlr, sdt_ldrb, LOAD, 8, lri, DEC, POST, WRITEBACK, USER_MODE);

// Load byte into user-mode register, Right-rotated register offset, post-decrement
arm_gen_sdt!(arm_ldrbt_ptrmrr, sdt_ldrb, LOAD, 8, rri, DEC, POST, WRITEBACK, USER_MODE);

// Load byte into user-mode register, Arithmetic-right-shifted register offset, post-increment
arm_gen_sdt!(arm_ldrbt_ptrpar, sdt_ldrb, LOAD, 8, ari, INC, POST, WRITEBACK, USER_MODE);

// Load byte into user-mode register, Left-shifted register offset, post-increment
arm_gen_sdt!(arm_ldrbt_ptrpll, sdt_ldrb, LOAD, 8, lli, INC, POST, WRITEBACK, USER_MODE);

// Load byte into user-mode register, Right-shifted register offset, post-increment
arm_gen_sdt!(arm_ldrbt_ptrplr, sdt_ldrb, LOAD, 8, lri, INC, POST, WRITEBACK, USER_MODE);

// Load byte into user-mode register, Right-rotated register offset, post-increment
arm_gen_sdt!(arm_ldrbt_ptrprr, sdt_ldrb, LOAD, 8, rri, INC, POST, WRITEBACK, USER_MODE);

// Load word into user-mode register, Immediate offset, post-decrement
arm_gen_sdt!(arm_ldrt_ptim, sdt_ldr, LOAD, 32, imm, DEC, POST, WRITEBACK, USER_MODE);

// Load word into user-mode register, Immediate offset, post-increment
arm_gen_sdt!(arm_ldrt_ptip, sdt_ldr, LOAD, 32, imm, INC, POST, WRITEBACK, USER_MODE);

// Load word into user-mode register, Arithmetic-right-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldrt_ptrmar, sdt_ldr, LOAD, 32, ari, DEC, POST, WRITEBACK, USER_MODE);

// Load word into user-mode register, Left-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldrt_ptrmll, sdt_ldr, LOAD, 32, lli, DEC, POST, WRITEBACK, USER_MODE);

// Load word into user-mode register, Right-shifted register offset, post-decrement
arm_gen_sdt!(arm_ldrt_ptrmlr, sdt_ldr, LOAD, 32, lri, DEC, POST, WRITEBACK, USER_MODE);

// Load word into user-mode register, Right-rotated register offset, post-decrement
arm_gen_sdt!(arm_ldrt_ptrmrr, sdt_ldr, LOAD, 32, rri, DEC, POST, WRITEBACK, USER_MODE);

// Load word into user-mode register, Arithmetic-right-shifted register offset, post-increment
arm_gen_sdt!(arm_ldrt_ptrpar, sdt_ldr, LOAD, 32, ari, INC, POST, WRITEBACK, USER_MODE);

// Load word into user-mode register, Left-shifted register offset, post-increment
arm_gen_sdt!(arm_ldrt_ptrpll, sdt_ldr, LOAD, 32, lli, INC, POST, WRITEBACK, USER_MODE);

// Load word into user-mode register, Right-shifted register offset, post-increment
arm_gen_sdt!(arm_ldrt_ptrplr, sdt_ldr, LOAD, 32, lri, INC, POST, WRITEBACK, USER_MODE);

// Load word into user-mode register, Right-rotated register offset, post-increment
arm_gen_sdt!(arm_ldrt_ptrprr, sdt_ldr, LOAD, 32, rri, INC, POST, WRITEBACK, USER_MODE);

// Store word, Negative immediate offset
arm_gen_sdt!(arm_str_ofim, sdt_str, STORE, 32, imm, DEC, PRE, NO_WRITEBACK);

// Store word, Positive immediate offset
arm_gen_sdt!(arm_str_ofip, sdt_str, STORE, 32, imm, INC, PRE, NO_WRITEBACK);

// Store word, Negative arithmetic-right-shifted register offset
arm_gen_sdt!(arm_str_ofrmar, sdt_str, STORE, 32, ari, DEC, PRE, NO_WRITEBACK);

// Store word, Negative left-shifted register offset
arm_gen_sdt!(arm_str_ofrmll, sdt_str, STORE, 32, lli, DEC, PRE, NO_WRITEBACK);

// Store word, Negative right-shifted register offset
arm_gen_sdt!(arm_str_ofrmlr, sdt_str, STORE, 32, lri, DEC, PRE, NO_WRITEBACK);

// Store word, Negative right-rotated register offset
arm_gen_sdt!(arm_str_ofrmrr, sdt_str, STORE, 32, rri, DEC, PRE, NO_WRITEBACK);

// Store word, Positive arithmetic-right-shifted register offset
arm_gen_sdt!(arm_str_ofrpar, sdt_str, STORE, 32, ari, INC, PRE, NO_WRITEBACK);

// Store word, Positive left-shifted register offset
arm_gen_sdt!(arm_str_ofrpll, sdt_str, STORE, 32, lli, INC, PRE, NO_WRITEBACK);

// Store word, Positive right-shifted register offset
arm_gen_sdt!(arm_str_ofrplr, sdt_str, STORE, 32, lri, INC, PRE, NO_WRITEBACK);

// Store word, Positive right-rotated register offset
arm_gen_sdt!(arm_str_ofrprr, sdt_str, STORE, 32, rri, INC, PRE, NO_WRITEBACK);

// Store word, Immediate offset, pre-decrement
arm_gen_sdt!(arm_str_prim, sdt_str, STORE, 32, imm, DEC, PRE, WRITEBACK);

// Store word, Immediate offset, pre-increment
arm_gen_sdt!(arm_str_prip, sdt_str, STORE, 32, imm, INC, PRE, WRITEBACK);

// Store word, Arithmetic-right-shifted register offset, pre-decrement
arm_gen_sdt!(arm_str_prrmar, sdt_str, STORE, 32, ari, DEC, PRE, WRITEBACK);

// Store word, Left-shifted register offset, pre-decrement
arm_gen_sdt!(arm_str_prrmll, sdt_str, STORE, 32, lli, DEC, PRE, WRITEBACK);

// Store word, Right-shifted register offset, pre-decrement
arm_gen_sdt!(arm_str_prrmlr, sdt_str, STORE, 32, lri, DEC, PRE, WRITEBACK);

// Store word, Right-rotated register offset, pre-decrement
arm_gen_sdt!(arm_str_prrmrr, sdt_str, STORE, 32, rri, DEC, PRE, WRITEBACK);

// Store word, Arithmetic-right-shifted register offset, pre-increment
arm_gen_sdt!(arm_str_prrpar, sdt_str, STORE, 32, ari, INC, PRE, WRITEBACK);

// Store word, Left-shifted register offset, pre-increment
arm_gen_sdt!(arm_str_prrpll, sdt_str, STORE, 32, lli, INC, PRE, WRITEBACK);

// Store word, Right-shifted register offset, pre-increment
arm_gen_sdt!(arm_str_prrplr, sdt_str, STORE, 32, lri, INC, PRE, WRITEBACK);

// Store word, Right-rotated register offset, pre-increment
arm_gen_sdt!(arm_str_prrprr, sdt_str, STORE, 32, rri, INC, PRE, WRITEBACK);

// Store word, Immediate offset, post-decrement
arm_gen_sdt!(arm_str_ptim, sdt_str, STORE, 32, imm, DEC, POST, WRITEBACK);

// Store word, Immediate offset, post-increment
arm_gen_sdt!(arm_str_ptip, sdt_str, STORE, 32, imm, INC, POST, WRITEBACK);

// Store word, Arithmetic-right-shifted register offset, post-decrement
arm_gen_sdt!(arm_str_ptrmar, sdt_str, STORE, 32, ari, DEC, POST, WRITEBACK);

// Store word, Left-shifted register offset, post-decrement
arm_gen_sdt!(arm_str_ptrmll, sdt_str, STORE, 32, lli, DEC, POST, WRITEBACK);

// Store word, Right-shifted register offset, post-decrement
arm_gen_sdt!(arm_str_ptrmlr, sdt_str, STORE, 32, lri, DEC, POST, WRITEBACK);

// Store word, Right-rotated register offset, post-decrement
arm_gen_sdt!(arm_str_ptrmrr, sdt_str, STORE, 32, rri, DEC, POST, WRITEBACK);

// Store word, Arithmetic-right-shifted register offset, post-increment
arm_gen_sdt!(arm_str_ptrpar, sdt_str, STORE, 32, ari, INC, POST, WRITEBACK);

// Store word, Left-shifted register offset, post-increment
arm_gen_sdt!(arm_str_ptrpll, sdt_str, STORE, 32, lli, INC, POST, WRITEBACK);

// Store word, Right-shifted register offset, post-increment
arm_gen_sdt!(arm_str_ptrplr, sdt_str, STORE, 32, lri, INC, POST, WRITEBACK);

// Store word, Right-rotated register offset, post-increment
arm_gen_sdt!(arm_str_ptrprr, sdt_str, STORE, 32, rri, INC, POST, WRITEBACK);

// Store byte, Negative immediate offset
arm_gen_sdt!(arm_strb_ofim, sdt_strb, STORE, 8, imm, DEC, PRE, NO_WRITEBACK);

// Store byte, Positive immediate offset
arm_gen_sdt!(arm_strb_ofip, sdt_strb, STORE, 8, imm, INC, PRE, NO_WRITEBACK);

// Store byte, Negative arithmetic-right-shifted register offset
arm_gen_sdt!(arm_strb_ofrmar, sdt_strb, STORE, 8, ari, DEC, PRE, NO_WRITEBACK);

// Store byte, Negative left-shifted register offset
arm_gen_sdt!(arm_strb_ofrmll, sdt_strb, STORE, 8, lli, DEC, PRE, NO_WRITEBACK);

// Store byte, Negative right-shifted register offset
arm_gen_sdt!(arm_strb_ofrmlr, sdt_strb, STORE, 8, lri, DEC, PRE, NO_WRITEBACK);

// Store byte, Negative right-rotated register offset
arm_gen_sdt!(arm_strb_ofrmrr, sdt_strb, STORE, 8, rri, DEC, PRE, NO_WRITEBACK);

// Store byte, Positive arithmetic-right-shifted register offset
arm_gen_sdt!(arm_strb_ofrpar, sdt_strb, STORE, 8, ari, INC, PRE, NO_WRITEBACK);

// Store byte, Positive left-shifted register offset
arm_gen_sdt!(arm_strb_ofrpll, sdt_strb, STORE, 8, lli, INC, PRE, NO_WRITEBACK);

// Store byte, Positive right-shifted register offset
arm_gen_sdt!(arm_strb_ofrplr, sdt_strb, STORE, 8, lri, INC, PRE, NO_WRITEBACK);

// Store byte, Positive right-rotated register offset
arm_gen_sdt!(arm_strb_ofrprr, sdt_strb, STORE, 8, rri, INC, PRE, NO_WRITEBACK);

// Store byte, Immediate offset, pre-decrement
arm_gen_sdt!(arm_strb_prim, sdt_strb, STORE, 8, imm, DEC, PRE, WRITEBACK);

// Store byte, Immediate offset, pre-increment
arm_gen_sdt!(arm_strb_prip, sdt_strb, STORE, 8, imm, INC, PRE, WRITEBACK);

// Store byte, Arithmetic-right-shifted register offset, pre-decrement
arm_gen_sdt!(arm_strb_prrmar, sdt_strb, STORE, 8, ari, DEC, PRE, WRITEBACK);

// Store byte, Left-shifted register offset, pre-decrement
arm_gen_sdt!(arm_strb_prrmll, sdt_strb, STORE, 8, lli, DEC, PRE, WRITEBACK);

// Store byte, Right-shifted register offset, pre-decrement
arm_gen_sdt!(arm_strb_prrmlr, sdt_strb, STORE, 8, lri, DEC, PRE, WRITEBACK);

// Store byte, Right-rotated register offset, pre-decrement
arm_gen_sdt!(arm_strb_prrmrr, sdt_strb, STORE, 8, rri, DEC, PRE, WRITEBACK);

// Store byte, Arithmetic-right-shifted register offset, pre-increment
arm_gen_sdt!(arm_strb_prrpar, sdt_strb, STORE, 8, ari, INC, PRE, WRITEBACK);

// Store byte, Left-shifted register offset, pre-increment
arm_gen_sdt!(arm_strb_prrpll, sdt_strb, STORE, 8, lli, INC, PRE, WRITEBACK);

// Store byte, Right-shifted register offset, pre-increment
arm_gen_sdt!(arm_strb_prrplr, sdt_strb, STORE, 8, lri, INC, PRE, WRITEBACK);

// Store byte, Right-rotated register offset, pre-increment
arm_gen_sdt!(arm_strb_prrprr, sdt_strb, STORE, 8, rri, INC, PRE, WRITEBACK);

// Store byte, Immediate offset, post-decrement
arm_gen_sdt!(arm_strb_ptim, sdt_strb, STORE, 8, imm, DEC, POST, WRITEBACK);

// Store byte, Immediate offset, post-increment
arm_gen_sdt!(arm_strb_ptip, sdt_strb, STORE, 8, imm, INC, POST, WRITEBACK);

// Store byte, Arithmetic-right-shifted register offset, post-decrement
arm_gen_sdt!(arm_strb_ptrmar, sdt_strb, STORE, 8, ari, DEC, POST, WRITEBACK);

// Store byte, Left-shifted register offset, post-decrement
arm_gen_sdt!(arm_strb_ptrmll, sdt_strb, STORE, 8, lli, DEC, POST, WRITEBACK);

// Store byte, Right-shifted register offset, post-decrement
arm_gen_sdt!(arm_strb_ptrmlr, sdt_strb, STORE, 8, lri, DEC, POST, WRITEBACK);

// Store byte, Right-rotated register offset, post-decrement
arm_gen_sdt!(arm_strb_ptrmrr, sdt_strb, STORE, 8, rri, DEC, POST, WRITEBACK);

// Store byte, Arithmetic-right-shifted register offset, post-increment
arm_gen_sdt!(arm_strb_ptrpar, sdt_strb, STORE, 8, ari, INC, POST, WRITEBACK);

// Store byte, Left-shifted register offset, post-increment
arm_gen_sdt!(arm_strb_ptrpll, sdt_strb, STORE, 8, lli, INC, POST, WRITEBACK);

// Store byte, Right-shifted register offset, post-increment
arm_gen_sdt!(arm_strb_ptrplr, sdt_strb, STORE, 8, lri, INC, POST, WRITEBACK);

// Store byte, Right-rotated register offset, post-increment
arm_gen_sdt!(arm_strb_ptrprr, sdt_strb, STORE, 8, rri, INC, POST, WRITEBACK);

// Store byte from user-mode register, Immediate offset, post-decrement
arm_gen_sdt!(arm_strbt_ptim, sdt_strb, STORE, 8, imm, DEC, POST, WRITEBACK, USER_MODE);

// Store byte from user-mode register, Immediate offset, post-increment
arm_gen_sdt!(arm_strbt_ptip, sdt_strb, STORE, 8, imm, INC, POST, WRITEBACK, USER_MODE);

// Store byte from user-mode register, Arithmetic-right-shifted register offset, post-decrement
arm_gen_sdt!(arm_strbt_ptrmar, sdt_strb, STORE, 8, ari, DEC, POST, WRITEBACK, USER_MODE);

// Store byte from user-mode register, Left-shifted register offset, post-decrement
arm_gen_sdt!(arm_strbt_ptrmll, sdt_strb, STORE, 8, lli, DEC, POST, WRITEBACK, USER_MODE);

// Store byte from user-mode register, Right-shifted register offset, post-decrement
arm_gen_sdt!(arm_strbt_ptrmlr, sdt_strb, STORE, 8, lri, DEC, POST, WRITEBACK, USER_MODE);

// Store byte from user-mode register, Right-rotated register offset, post-decrement
arm_gen_sdt!(arm_strbt_ptrmrr, sdt_strb, STORE, 8, rri, DEC, POST, WRITEBACK, USER_MODE);

// Store byte from user-mode register, Arithmetic-right-shifted register offset, post-increment
arm_gen_sdt!(arm_strbt_ptrpar, sdt_strb, STORE, 8, ari, INC, POST, WRITEBACK, USER_MODE);

// Store byte from user-mode register, Left-shifted register offset, post-increment
arm_gen_sdt!(arm_strbt_ptrpll, sdt_strb, STORE, 8, lli, INC, POST, WRITEBACK, USER_MODE);

// Store byte from user-mode register, Right-shifted register offset, post-increment
arm_gen_sdt!(arm_strbt_ptrplr, sdt_strb, STORE, 8, lri, INC, POST, WRITEBACK, USER_MODE);

// Store byte from user-mode register, Right-rotated register offset, post-increment
arm_gen_sdt!(arm_strbt_ptrprr, sdt_strb, STORE, 8, rri, INC, POST, WRITEBACK, USER_MODE);

// Store word from user-mode register, Immediate offset, post-decrement
arm_gen_sdt!(arm_strt_ptim, sdt_str, STORE, 32, imm, DEC, POST, WRITEBACK, USER_MODE);

// Store word from user-mode register, Immediate offset, post-increment
arm_gen_sdt!(arm_strt_ptip, sdt_str, STORE, 32, imm, INC, POST, WRITEBACK, USER_MODE);

// Store word from user-mode register, Arithmetic-right-shifted register offset, post-decrement
arm_gen_sdt!(arm_strt_ptrmar, sdt_str, STORE, 32, ari, DEC, POST, WRITEBACK, USER_MODE);

// Store word from user-mode register, Left-shifted register offset, post-decrement
arm_gen_sdt!(arm_strt_ptrmll, sdt_str, STORE, 32, lli, DEC, POST, WRITEBACK, USER_MODE);

// Store word from user-mode register, Right-shifted register offset, post-decrement
arm_gen_sdt!(arm_strt_ptrmlr, sdt_str, STORE, 32, lri, DEC, POST, WRITEBACK, USER_MODE);

// Store word from user-mode register, Right-rotated register offset, post-decrement
arm_gen_sdt!(arm_strt_ptrmrr, sdt_str, STORE, 32, rri, DEC, POST, WRITEBACK, USER_MODE);

// Store word from user-mode register, Arithmetic-right-shifted register offset, post-increment
arm_gen_sdt!(arm_strt_ptrpar, sdt_str, STORE, 32, ari, INC, POST, WRITEBACK, USER_MODE);

// Store word from user-mode register, Left-shifted register offset, post-increment
arm_gen_sdt!(arm_strt_ptrpll, sdt_str, STORE, 32, lli, INC, POST, WRITEBACK, USER_MODE);

// Store word from user-mode register, Right-shifted register offset, post-increment
arm_gen_sdt!(arm_strt_ptrplr, sdt_str, STORE, 32, lri, INC, POST, WRITEBACK, USER_MODE);

// Store word from user-mode register, Right-rotated register offset, post-increment
arm_gen_sdt!(arm_strt_ptrprr, sdt_str, STORE, 32, rri, INC, POST, WRITEBACK, USER_MODE);

