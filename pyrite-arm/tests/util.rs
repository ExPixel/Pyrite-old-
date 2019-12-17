use pyrite_arm::{ArmCpu, ArmMemory};
use pyrite_common::Shared;

pub fn read_swi_comment(memory: &mut dyn ArmMemory, addr: u32, thumb: bool) -> u32 {
    if thumb {
        let opcode = memory.read_data_halfword(addr, false, &mut 0) as u32;
        return opcode & 0xFF;
    } else {
        let opcode = memory.read_data_word(addr, false, &mut 0);
        return opcode & 0xFFFFFF;
    }
}

pub fn run_cpu(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory) -> Option<(u32, u32)> {
    use pyrite_arm::cpu::CpuException;

    const MAX_INSTRUCTIONS: u32 = 1000000;

    let signal: Shared<Option<(u32, u32)>> = Shared::new(None);
    let should_break = Shared::new(false);

    let handler_signal = Shared::share(&signal);
    let handler_should_break = Shared::share(&should_break);
    let maybe_old_handler = cpu.set_exception_handler(Box::new(
        move |cpu, memory, exception_type, exception_addr| -> bool {
            if exception_type == CpuException::SWI {
                let swi_comment = read_swi_comment(memory, exception_addr, cpu.registers.getf_t());

                match swi_comment {
                    4 => {
                        *handler_signal.borrow_mut() =
                            Some((cpu.registers.read(0), cpu.registers.read(1)));
                        *handler_should_break.borrow_mut() = true;
                    }

                    16 => {
                        *handler_should_break.borrow_mut() = true;
                        println!("SWI: HALT");
                    }

                    17 => {
                        *handler_should_break.borrow_mut() = true;
                        panic!("SWI: BAD HALT (program main did not call halt)");
                    }

                    _ => {
                        panic!(
                            "Unknown Software Interrupt: {} ({:X})",
                            swi_comment, swi_comment
                        );
                    }
                }

                return true;
            }
            false
        },
    ));

    let mut instruction_count = 0;
    while !*should_break.borrow() {
        cpu.step(memory);
        instruction_count += 1;
        if instruction_count > MAX_INSTRUCTIONS {
            panic!("HIT TEST INSTRUCTION LIMIT");
        }
    }

    drop(cpu.remove_exception_handler());
    if let Some(old_handler) = maybe_old_handler {
        cpu.set_exception_handler(old_handler);
    }

    return Shared::unwrap(signal);
}
