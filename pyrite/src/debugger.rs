use pyrite_gba::{Gba, GbaAudioOutput, GbaVideoOutput};

// TODO turn this into an actual debugger :P
pub struct GbaDebugger {
    step_size: Option<GbaStepSize>,
    pub debugging: bool,
    pub paused: bool,
}

impl GbaDebugger {
    pub fn new() -> GbaDebugger {
        GbaDebugger {
            step_size: None,
            debugging: false,
            paused: false,
        }
    }

    pub fn step_gba_video_frame(
        &mut self,
        gba: &mut Gba,
        video: &mut dyn GbaVideoOutput,
        audio: &mut dyn GbaAudioOutput,
    ) {
        debug_assert!(self.debugging);
        while let (false, _) = gba.step(video, audio) {
            if self.check_break(gba) {
                log::debug!("PAUSE");
                self.paused = true;
                break;
            }
        }
    }

    #[allow(dead_code)]
    fn print_register_trace(gba: &Gba) {
        println!("[TRACE before 0x{:08X}]", gba.cpu.next_exec_address());

        println!(
            " r0 = 0x{:08X},  r1 = 0x{:08X},  r2 = 0x{:08X},  r3 = 0x{:08X}",
            gba.cpu.registers.read(0),
            gba.cpu.registers.read(1),
            gba.cpu.registers.read(2),
            gba.cpu.registers.read(3)
        );
        println!(
            " r4 = 0x{:08X},  r5 = 0x{:08X},  r6 = 0x{:08X},  r7 = 0x{:08X}",
            gba.cpu.registers.read(4),
            gba.cpu.registers.read(5),
            gba.cpu.registers.read(6),
            gba.cpu.registers.read(7)
        );
        println!(
            " r8 = 0x{:08X},  r9 = 0x{:08X}, r10 = 0x{:08X}, r11 = 0x{:08X}",
            gba.cpu.registers.read(8),
            gba.cpu.registers.read(9),
            gba.cpu.registers.read(10),
            gba.cpu.registers.read(11)
        );
        println!(
            "r12 = 0x{:08X}, r13 = 0x{:08X}, r14 = 0x{:08X}, r15 = 0x{:08X}",
            gba.cpu.registers.read(12),
            gba.cpu.registers.read(13),
            gba.cpu.registers.read(14),
            gba.cpu.registers.read(15)
        );

        println!(
            "CPSR =  0x{:08X} [{}{}{}{}{}{}{}] ({})",
            gba.cpu.registers.read_cpsr(),
            if gba.cpu.registers.getf_n() { "N" } else { "-" },
            if gba.cpu.registers.getf_z() { "Z" } else { "-" },
            if gba.cpu.registers.getf_c() { "C" } else { "-" },
            if gba.cpu.registers.getf_v() { "V" } else { "-" },
            if gba.cpu.registers.getf_i() { "I" } else { "-" },
            if gba.cpu.registers.getf_f() { "F" } else { "-" },
            if gba.cpu.registers.getf_t() { "T" } else { "-" },
            Self::short_mode_name(gba.cpu.registers.read_mode()),
        );
    }

    fn short_mode_name(mode: pyrite_arm::registers::CpuMode) -> &'static str {
        use pyrite_arm::registers::CpuMode;

        match mode {
            CpuMode::User => "USR",
            CpuMode::System => "SYS",
            CpuMode::FIQ => "FIQ",
            CpuMode::IRQ => "IRQ",
            CpuMode::Supervisor => "SVC",
            CpuMode::Abort => "ABT",
            CpuMode::Undefined => "UND",
            CpuMode::Invalid => "INV",
        }
    }

    #[allow(unused_variables)]
    fn check_break(&mut self, gba: &mut Gba) -> bool {
        false
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn break_execution(&mut self) {
        self.debugging = true;
        self.paused = true;
    }

    pub fn step(&mut self, step_size: GbaStepSize) {
        self.step_size = Some(step_size);
    }

    pub fn pop_step_size(&mut self) -> Option<GbaStepSize> {
        self.step_size.take()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GbaStepSize {
    Instruction,
    #[allow(dead_code)]
    VideoLine,
    #[allow(dead_code)]
    VideoFrame,
}
