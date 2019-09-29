//! Stubs for ARM _instructions that have yet to be implemented.

use super::super::{ ArmCpu, ArmMemory };

// Perform coprocessor data operation
pub fn arm_cdp(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Load coprocessor data from memory, Negative offset
pub fn arm_ldc_ofm(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Load coprocessor data from memory, Positive offset
pub fn arm_ldc_ofp(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Load coprocessor data from memory, Pre-decrement
pub fn arm_ldc_prm(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Load coprocessor data from memory, Pre-increment
pub fn arm_ldc_prp(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Load coprocessor data from memory, Post-decrement
pub fn arm_ldc_ptm(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Load coprocessor data from memory, Post-increment
pub fn arm_ldc_ptp(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Load coprocessor data from memory, Unindexed, bits 7-0 available for copro use
pub fn arm_ldc_unm(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Load coprocessor data from memory, Unindexed, bits 7-0 available for copro use
pub fn arm_ldc_unp(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Write coprocessor register from ARM register
pub fn arm_mcr(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Read coprocessor register to ARM register
pub fn arm_mrc(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Store coprocessor data to memory, Negative offset
pub fn arm_stc_ofm(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Store coprocessor data to memory, Positive offset
pub fn arm_stc_ofp(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Store coprocessor data to memory, Pre-decrement
pub fn arm_stc_prm(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Store coprocessor data to memory, Pre-increment
pub fn arm_stc_prp(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Store coprocessor data to memory, Post-decrement
pub fn arm_stc_ptm(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Store coprocessor data to memory, Post-increment
pub fn arm_stc_ptp(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Store coprocessor data to memory, Unindexed, bits 7-0 available for copro use
pub fn arm_stc_unm(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

// Store coprocessor data to memory, Unindexed, bits 7-0 available for copro use
pub fn arm_stc_unp(_cpu: &mut ArmCpu, _memory: &mut dyn ArmMemory, _instr: u32) {
    gba_error!("coprocessor instructions not implemented"); // #COPROCESSOR_FN
}

