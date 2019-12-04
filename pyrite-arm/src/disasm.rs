use super::ArmMemory;

const ARM_OPCODE_TABLE: [(u32, u32); 15] = [
    (0x0ffffff0, 0x012fff10), // Branch and Exchange
    (0x0fb00ff0, 0x01000090), // Single Data Swap
    (0x0fc000f0, 0x00000090), // Multiply
    (0x0e400f90, 0x00000090), // Halfword Data Transfer (register offset)
    (0x0f8000f0, 0x00800090), // Multiply Long
    (0x0e400090, 0x00400090), // Halfword Data Transfer (immediate offset)
    (0x0f000010, 0x0e000000), // Coprocessor Data Operation
    (0x0f000010, 0x0e000010), // Coprocessor Register Transfer
    (0x0e000010, 0x06000010), // Undefined
    (0x0f000000, 0x0f000000), // Software Interrupt
    (0x0e000000, 0x08000000), // Block Data Transfer
    (0x0e000000, 0x0a000000), // Branch
    (0x0e000000, 0x0c000000), // Coprocessor Data Transfer
    (0x0c000000, 0x00000000), // Data Processing / PSR Transfer
    (0x0c000000, 0x04000000), // Single Data Transfer
];

const THUMB_OPCODE_TABLE: [(u16, u16); 17] = [
    (0xff00, 0xb000), // Add Offset to Stack Pointer
    (0xff00, 0xdf00), // Software Interrupt
    (0xfc00, 0x4000), // ALU Operations
    (0xfc00, 0x4400), // Hi Register Operations / Branch Exchange
    (0xf600, 0xb400), // Push/Pop Registers
    (0xf800, 0x1800), // Add / Subtract
    (0xf800, 0x4800), // PC-relative Load
    (0xf200, 0x5000), // Load/Store with register offset
    (0xf200, 0x5200), // Load/Store Sign-Extended Byte/Halfword
    (0xf000, 0x8000), // Load/Store Halfword
    (0xf000, 0x9000), // SP-relative Load/Store
    (0xf000, 0xa000), // Load Address
    (0xf000, 0xc000), // Multiple Load/Store
    (0xf000, 0xd000), // Conditional Branch
    (0xe000, 0x0000), // Move Shifted Register
    (0xe000, 0x2000), // Move/ Compare/ Add/ Subtract Immediate
    (0xe000, 0x6000), // Load/Store with Immediate Offset
];

pub fn disassemble(thumb: bool, address: u32, memory: &dyn ArmMemory) {
    if thumb {
        let opcode = memory.view_halfword(address);
        for (select_bits, diff) in THUMB_OPCODE_TABLE.iter() {
            if ((opcode & select_bits) ^ diff) == 0 {
                unimplemented!("thumb disassembly");
            }
        }
        unimplemented!("thumb undefined");
    } else {
        let opcode = memory.view_word(address);
        for (select_bits, diff) in ARM_OPCODE_TABLE.iter() {
            if ((opcode & select_bits) ^ diff) == 0 {
                unimplemented!("arm disassembly");
            }
        }
        unimplemented!("arm undefined");
    }
}

pub fn disassemble_arm(dest: &mut String, address: u32, memory: &dyn ArmMemory) {
    use std::fmt::Write;
    let opcode = memory.view_word(address);
    for (select_bits, diff) in ARM_OPCODE_TABLE.iter() {
        if ((opcode & select_bits) ^ diff) == 0 {
            write!(dest, "undefined @ = {:08X}", memory.view_word(address)).unwrap();
            return;
        }
    }
    dest.push_str("undefined");
}

pub fn disassemble_thumb(dest: &mut String, address: u32, memory: &dyn ArmMemory) {
    use std::fmt::Write;
    let opcode = memory.view_halfword(address);
    for (select_bits, diff) in THUMB_OPCODE_TABLE.iter() {
        if ((opcode & select_bits) ^ diff) == 0 {
            write!(dest, "undefined @ = {:04X}", memory.view_halfword(address)).unwrap();
            return;
        }
    }
    dest.push_str("undefined");
}
