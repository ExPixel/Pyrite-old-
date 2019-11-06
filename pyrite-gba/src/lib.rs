#[macro_use] mod util;
mod sysctl;
#[allow(dead_code)]
mod ioregs;
mod hardware;
mod lcd;

use hardware::GbaHardware;
use pyrite_arm::ArmCpu;
use pyrite_arm::cpu::CpuException;

pub struct Gba {
    pub cpu:        ArmCpu,
    pub hardware:   GbaHardware,
}

impl Gba {
    pub fn new() -> Gba {
        let mut g = Gba {
            cpu:        ArmCpu::new(),
            hardware:   GbaHardware::new(),
        };
        g.setup_handler();
        return g;
    }

    fn setup_handler(&mut self) {
        self.cpu.set_exception_handler(Box::new(|_cpu, _memory, exception, exception_addr| {
            match exception {
                CpuException::Reset => false,
                CpuException::SWI   => {
                    // println!("SWI from {:08X}", exception_addr);
                    false
                },
                CpuException::IRQ   => {
                    // println!("IRQ from {:08X}", exception_addr);
                    false
                },
                _ => {
                    eprintln!("error: {} exception at 0x{:08X}", exception.name(), exception_addr);
                    // consume the exception
                    true 
                },
            }
        }));
    }

    pub fn reset(&mut self, skip_bios: bool) {
        use pyrite_arm::registers;
        self.cpu.registers.setf_f(); // Disables FIQ interrupts (always high on the GBA)
        self.hardware.sysctl.set_reg_waitcnt(0x4317);
        if skip_bios {
            self.cpu.set_pc(0x08000000, &mut self.hardware);
            self.cpu.registers.setf_i(); // Disables IRQ interrupts
            self.cpu.registers.write_mode(registers::CpuMode::System);
            self.cpu.registers.write_with_mode(registers::CpuMode::User, 13, 0x03007F00); // Also System
            self.cpu.registers.write_with_mode(registers::CpuMode::IRQ, 13, 0x03007FA0);
            self.cpu.registers.write_with_mode(registers::CpuMode::Supervisor, 13, 0x03007FE0);
        } else {
            self.cpu.registers.setf_i(); // Disables IRQ interrupts
            self.cpu.set_pc(0x00000000, &mut self.hardware);
            self.cpu.registers.write_mode(registers::CpuMode::Supervisor);
        }

        // @TODO set the keys here
        // self.memory.ioregs.keyinput.inner = 0x3FF;
        // @TODO some more IO registers need to be set here.
    }

    pub fn set_rom(&mut self, rom: Vec<u8>) {
        self.hardware.set_gamepak_rom(rom);
    }

    pub fn set_bios(&mut self, bios: Vec<u8>) {
        self.hardware.set_bios_rom(&bios);
    }

    pub fn init(&mut self, video: &mut dyn GbaVideoOutput, _audio: &mut dyn GbaAudioOutput) {
        // @TODO reimplement lcd init
        // self.lcd.init(video);
    }

    #[inline]
    pub fn step(&mut self, video: &mut dyn GbaVideoOutput, _audio: &mut dyn GbaAudioOutput) {
        // @TODO reimplement DMA and other step stuff.
        let cycles = self.cpu.step(&mut self.hardware);
        self.hardware.lcd.step(cycles, &self.hardware.vram, &self.hardware.oam, &self.hardware.pal, video);
        // let cycles = if dma::is_any_dma_active(&self.memory) {
        //     dma::step_active_channels(&mut self.memory, !self.cpu.registers.getf_i())
        // } else if !self.memory.ioregs.internal_halt {
        //     self.cpu.step(&mut self.memory)
        // } else {
        //     8 // number of cycles that we advance each step while halted
        // };

        // @TODO reimplement timers
        // if timers::is_any_timer_active(&self.memory) {
        //     timers::step_active_timers(cycles, &mut self.memory, !self.cpu.registers.getf_i());
        // }

        // reimplement lcd step
        // self.lcd.step(cycles, &mut self.memory, video, !self.cpu.registers.getf_i());

        // @TODO reimplement IRQ
        // if !self.cpu.registers.getf_i() && self.memory.ioregs.interrupt_request.inner != 0 {
        //     self.memory.ioregs.internal_halt = false;
        //     self.cpu.set_pending_exception(CpuException::IRQ);
        // }
    }

    pub fn set_key_pressed(&mut self, key: KeypadInput, pressed: bool) {
        // @TODO reimplement key presses
        // 0 = Pressed, 1 = Released
        // if pressed {
        //     self.memory.ioregs.keyinput.inner &= !key.mask();
        // } else {
        //     self.memory.ioregs.keyinput.inner |= key.mask();
        // }
    }

    pub fn is_key_pressed(&mut self, key: KeypadInput) -> bool {
        return false;
        // @TODO reimplement key press checks
        // (self.memory.ioregs.keyinput.inner & (key.mask())) == 0
    }
}

#[derive(Clone, Copy)]
#[repr(u16)]
pub enum KeypadInput {
    ButtonA = 0,
    ButtonB = 1,
    Select  = 2,
    Start   = 3,
    Right   = 4,
    Left    = 5,
    Up      = 6,
    Down    = 7,
    ButtonR = 8,
    ButtonL = 9,
}

impl KeypadInput {
    fn mask(self) -> u16 {
        1 << (self as u16)
    }
}

pub trait GbaVideoOutput {
    /// Called at the beginning of line 0 to signal the start of a new frame.
    fn pre_frame(&mut self);

    /// Called after the last line has been drawn to signal the end of a frame.
    fn post_frame(&mut self);

    /// Called by the LCD every time a line is ready to be committed to the video
    /// output somehow.
    fn display_line(&mut self, line: u32, pixels: &[u16]);
}

pub trait GbaAudioOutput {
    // @TODO Not sure how I want to do this one yet. Instead of having all of the samples
    //       generated by the GBA, I might just send the various states of the channels
    //       instead and have the audio output device (whatever it is) handle generating
    //       the output for each. But that would rely on whatever is on the otherside generating
    //       samples knowing a lot ofthings about the GBA's internals which is what I've been trying to
    //       avoid.
    //
    //       -- Marc C. [25 September, 2019]
    fn play_samples(&mut self);
}

pub struct NoVideoOutput {
    pub frame_ready: bool,
}

impl NoVideoOutput {
    pub const fn new() -> NoVideoOutput {
        NoVideoOutput {
            frame_ready: false,
        }
    }
}

pub struct NoAudioOutput;

impl GbaVideoOutput for NoVideoOutput {
    fn pre_frame(&mut self) { /* NOP */ }
    fn post_frame(&mut self) {
        self.frame_ready = true;
    }
    fn display_line(&mut self, _line: u32, _pixels: &[u16]) { /* NOP */ }
}

impl GbaAudioOutput for NoAudioOutput {
    fn play_samples(&mut self) {
        /* NOP */
    }
}
