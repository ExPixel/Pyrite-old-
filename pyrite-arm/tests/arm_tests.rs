mod util;
use pyrite_arm::{ArmCpu, ArmMemory};
use util::run_cpu;

pub const MAX_PROGRAM_SIZE: u32 = 0x1000;

fn set_memory_bytes(memory: &mut dyn ArmMemory, mut addr: u32, src: &[u8]) {
    for src_byte in src.iter() {
        memory.write_data_byte(addr, *src_byte, false, &mut 0);
        addr += 1;
    }
}

fn get_memory_bytes(memory: &mut dyn ArmMemory, mut addr: u32, dst: &mut [u8]) {
    for dst_byte in dst.iter_mut() {
        *dst_byte = memory.read_data_byte(addr, false, &mut 0);
        addr += 1;
    }
}

#[test]
pub fn test_division() {
    static DIVISION_BIN: &[u8] = include_bytes!("../data/bin/arm_division.bin");

    let mut cpu = ArmCpu::new();
    let mut mem = DIVISION_BIN.to_vec();
    mem.resize(0x1000, 0xCE);

    // some guard values for these registers which are supported
    // to be saved and restored.
    cpu.registers.write(3, 0xDEAD);
    cpu.registers.write(4, 0xBEEF);

    {
        let mut divide = |dividend: u32, divisor: u32| -> (u32, u32) {
            let _ = cpu.set_pc(0, &mut mem); // reset the program counter
            cpu.registers.write(0, dividend);
            cpu.registers.write(1, divisor);
            while let Some(_signal) = run_cpu(&mut cpu, &mut mem) { /* IGNORE SIGNAL */ }
            return (cpu.registers.read(0), cpu.registers.read(1));
        };

        let dividend = 84837567;
        let divisor = 127;
        let result = dividend / divisor;
        let remainder = dividend % divisor;
        assert_eq!(divide(dividend, divisor), (result, remainder));
    }

    // make sure that guard values are correct
    assert_eq!(cpu.registers.read(3), 0xDEAD);
    assert_eq!(cpu.registers.read(4), 0xBEEF);
}

#[test]
pub fn test_chacha20() {
    let expected = [
        0x10, 0xf1, 0xe7, 0xe4, 0xd1, 0x3b, 0x59, 0x15, 0x50, 0x0f, 0xdd, 0x1f, 0xa3, 0x20, 0x71,
        0xc4, 0xc7, 0xd1, 0xf4, 0xc7, 0x33, 0xc0, 0x68, 0x03, 0x04, 0x22, 0xaa, 0x9a, 0xc3, 0xd4,
        0x6c, 0x4e, 0xd2, 0x82, 0x64, 0x46, 0x07, 0x9f, 0xaa, 0x09, 0x14, 0xc2, 0xd7, 0x05, 0xd9,
        0x8b, 0x02, 0xa2, 0xb5, 0x12, 0x9c, 0xd1, 0xde, 0x16, 0x4e, 0xb9, 0xcb, 0xd0, 0x83, 0xe8,
        0xa2, 0x50, 0x3c, 0x4e,
    ];

    let test_key = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
        0x1e, 0x1f,
    ];

    let test_nonce = [
        0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x4A, 0x00, 0x00, 0x00, 0x00,
    ];

    let mut dest = [0u8; 64];

    chacha20(1, &test_key, &test_nonce, &mut dest);
    assert_eq!(&expected[0..], &dest[0..]);

    fn chacha20(iterations: u32, key: &[u8], nonce: &[u8], dest: &mut [u8]) {
        static CHACHA20_BIN: &[u8] = include_bytes!("../data/bin/chacha20.bin");

        let mut cpu = ArmCpu::new();
        let mut mem = CHACHA20_BIN.to_vec();
        mem.resize(0x1000, 0xCE);
        let _ = cpu.set_pc(0, &mut mem); // reset the program counter

        while let Some((signal_type, signal_value)) = run_cpu(&mut cpu, &mut mem) {
            match signal_type {
                0 => {
                    let key_addr = signal_value;
                    set_memory_bytes(&mut mem, key_addr, &key[0..32]);
                }
                1 => {
                    let nonce_addr = signal_value;
                    set_memory_bytes(&mut mem, nonce_addr, &nonce[0..12]);
                }
                2 => {
                    cpu.registers.write(0, iterations);
                }
                3 => {
                    let dest_addr = signal_value;
                    get_memory_bytes(&mut mem, dest_addr, dest);
                }
                _ => panic!(
                    "unrecognized signal: [{}](0x{:08X})",
                    signal_type, signal_value
                ),
            }
        }
    }
}

#[test]
pub fn test_chacha20_thumb() {
    let expected = [
        0x10, 0xf1, 0xe7, 0xe4, 0xd1, 0x3b, 0x59, 0x15, 0x50, 0x0f, 0xdd, 0x1f, 0xa3, 0x20, 0x71,
        0xc4, 0xc7, 0xd1, 0xf4, 0xc7, 0x33, 0xc0, 0x68, 0x03, 0x04, 0x22, 0xaa, 0x9a, 0xc3, 0xd4,
        0x6c, 0x4e, 0xd2, 0x82, 0x64, 0x46, 0x07, 0x9f, 0xaa, 0x09, 0x14, 0xc2, 0xd7, 0x05, 0xd9,
        0x8b, 0x02, 0xa2, 0xb5, 0x12, 0x9c, 0xd1, 0xde, 0x16, 0x4e, 0xb9, 0xcb, 0xd0, 0x83, 0xe8,
        0xa2, 0x50, 0x3c, 0x4e,
    ];

    let test_key = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
        0x1e, 0x1f,
    ];

    let test_nonce = [
        0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x4A, 0x00, 0x00, 0x00, 0x00,
    ];

    let mut dest = [0u8; 64];

    chacha20(1, &test_key, &test_nonce, &mut dest);
    assert_eq!(&expected[0..], &dest[0..]);

    fn chacha20(iterations: u32, key: &[u8], nonce: &[u8], dest: &mut [u8]) {
        static CHACHA20_BIN_THUMB: &[u8] = include_bytes!("../data/bin/chacha20_thumb.bin");

        let mut cpu = ArmCpu::new();
        let mut mem = CHACHA20_BIN_THUMB.to_vec();
        mem.resize(0x1000, 0xCE);
        let _ = cpu.set_pc(0, &mut mem); // reset the program counter

        while let Some((signal_type, signal_value)) = run_cpu(&mut cpu, &mut mem) {
            match signal_type {
                0 => {
                    let key_addr = signal_value;
                    println!("key_addr: 0x{:08X}", key_addr);
                    set_memory_bytes(&mut mem, key_addr, &key[0..32]);
                }
                1 => {
                    let nonce_addr = signal_value;
                    set_memory_bytes(&mut mem, nonce_addr, &nonce[0..12]);
                }
                2 => {
                    cpu.registers.write(0, iterations);
                }
                3 => {
                    let dest_addr = signal_value;
                    get_memory_bytes(&mut mem, dest_addr, dest);
                }
                _ => panic!(
                    "unrecognized signal: [{}](0x{:08X})",
                    signal_type, signal_value
                ),
            }
        }
    }
}

// @ NOTE I uncomment these two VERY inefficient functions
//        and use them while I am debugging and for nothing else.
// fn to_hex(data: &[u8]) -> String {
//     let mut out = String::new();
//     let mut first = true;
//     for b in data.iter() {
//         if first { first = false; } else { out.push(':'); }
//         out.push_str(&format!("{:02X}", *b));
//     }
//     return out;
// }
// fn to_int_arr_str(data: &[u8]) -> String {
//     let mut out = String::new();
//     let mut first = true;
//     for bytes in data.chunks(4) {
//         if first { first = false; } else { out.push(' '); }
//         let int = (bytes[0] as u32) |
//             ((bytes[1] as u32) << 8)|
//             ((bytes[2] as u32) << 16)|
//             ((bytes[3] as u32) << 24);
//         out.push_str(&format!("{:08X}", int));
//     }
//     return out;
// }
