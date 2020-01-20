#[macro_use]
mod util;
pub mod audio;
pub mod dma;
mod hardware;
#[allow(dead_code)]
mod ioregs;
pub mod irq;
pub mod keypad;
pub mod lcd;
mod sysctl;
pub mod timers;

use hardware::GbaHardware;
use pyrite_arm::cpu::CpuException;
use pyrite_arm::ArmCpu;

pub struct Gba {
    pub cpu: ArmCpu,
    pub hardware: GbaHardware,
    state: GbaSystemState,
}

impl Gba {
    #[inline]
    pub fn new() -> Gba {
        let mut g = Gba {
            cpu: ArmCpu::new(),
            hardware: GbaHardware::new(),
            state: GbaSystemState::Running,
        };
        g.setup_handler();
        return g;
    }

    #[inline]
    pub fn alloc() -> Box<Gba> {
        let mut g = Box::new(Gba {
            cpu: ArmCpu::new(),
            hardware: GbaHardware::new(),
            state: GbaSystemState::Running,
        });
        g.setup_handler();
        return g;
    }

    fn setup_handler(&mut self) {
        self.cpu
            .set_exception_handler(Box::new(|_cpu, _memory, exception, exception_addr| {
                match exception {
                    CpuException::Reset => false,
                    CpuException::SWI => {
                        log::debug!("SWI from 0x{:08X}", exception_addr);
                        false
                    }
                    CpuException::IRQ => false,
                    _ => {
                        log::warn!("{} exception at 0x{:08X}", exception.name(), exception_addr);
                        // consume the exception
                        true
                    }
                }
            }));
    }

    pub fn reset(&mut self, skip_bios: bool) {
        use pyrite_arm::registers;
        self.cpu.registers.setf_f(); // Disables FIQ interrupts (always high on the GBA)
        self.hardware.sysctl.set_reg_waitcnt(0x4317);
        if skip_bios {
            let _ = self.cpu.set_pc(0x08000000, &mut self.hardware);
            self.cpu.registers.setf_i(); // Disables IRQ interrupts
            self.cpu.registers.write_mode(registers::CpuMode::System);
            self.cpu
                .registers
                .write_with_mode(registers::CpuMode::User, 13, 0x03007F00); // Also System
            self.cpu
                .registers
                .write_with_mode(registers::CpuMode::IRQ, 13, 0x03007FA0);
            self.cpu
                .registers
                .write_with_mode(registers::CpuMode::Supervisor, 13, 0x03007FE0);
        } else {
            self.cpu.registers.setf_i(); // Disables IRQ interrupts
            let _ = self.cpu.set_pc(0x00000000, &mut self.hardware);
            self.cpu
                .registers
                .write_mode(registers::CpuMode::Supervisor);
        }
    }

    pub fn set_rom(&mut self, rom: Vec<u8>) {
        self.hardware.set_gamepak_rom(rom);
    }

    pub fn set_bios(&mut self, bios: Vec<u8>) {
        self.hardware.set_bios_rom(&bios);
    }

    /// Returns a tuple with the first value being true if this step marked the end of a video
    /// frame, and the second value being true if this step marked the end of an audio frame.
    #[inline]
    pub fn step(
        &mut self,
        video: &mut dyn GbaVideoOutput,
        _audio: &mut dyn GbaAudioOutput,
    ) -> (bool, bool) {
        if self.hardware.events.count() > 0 {
            self.process_all_hardware_events();
        }

        let cycles = self.cpu.step(&mut self.hardware);

        if self.hardware.timers.active() {
            self.hardware.timers.step(cycles, &mut self.hardware.events);
        }

        let video_frame = self.hardware.lcd.step(
            cycles,
            &self.hardware.vram,
            &self.hardware.oam,
            &self.hardware.pal,
            video,
            &mut self.hardware.dma,
            &mut self.hardware.events,
        );
        let audio_frame = false;

        return (video_frame, audio_frame);
    }

    #[cold]
    fn process_all_hardware_events(&mut self) {
        while self.hardware.events.count() > 0 {
            let event = self.hardware.events.pop();
            self.process_hardware_event(event);
        }
    }

    fn process_hardware_event(&mut self, event: hardware::HardwareEvent) {
        use hardware::HardwareEvent;

        match event {
            HardwareEvent::IRQ(irq) => {
                if !self.cpu.registers.getf_i() && self.hardware.irq.request(irq) {
                    self.state = GbaSystemState::Running;
                    self.cpu.set_pending_exception_active(CpuException::IRQ);
                    self.hardware.dma.resume_transfer(&mut self.cpu); // will only resume if there is a DMA
                }
            }

            HardwareEvent::DMA(dma) => {
                self.hardware.dma.begin_transfer(dma, &mut self.cpu);
            }

            HardwareEvent::Halt => {
                self.state = GbaSystemState::Halted;
                self.cpu.set_idle(true, 4); // We don't want to be too fine grained here or performance is bad.
            }

            HardwareEvent::Stop => {
                log::warn!("stop event is still unstable");
                self.state = GbaSystemState::Stopped;
                self.cpu.set_idle(true, 16); // we use big steps for stop because everything we need high fidelity for is off.
            }

            HardwareEvent::None => {
                unreachable!("HardwareEvent::None");
            }
        }
    }

    /// Steps the GBA until the end of a video frame.
    #[inline]
    pub fn video_frame(&mut self, video: &mut dyn GbaVideoOutput, audio: &mut dyn GbaAudioOutput) {
        // #NOTE: this draws a blank frame without rendering:
        // let mut cycles = 0;
        // while cycles < 280896 {
        //     cycles += self.cpu.step(&mut self.hardware);
        // }

        while let (false, _) = self.step(video, audio) { /* NOP */ }
    }

    #[inline]
    pub fn set_key_pressed(&mut self, key: keypad::KeypadInput, pressed: bool) {
        self.hardware.keypad.set_pressed(key, pressed);
    }

    #[inline]
    pub fn is_key_pressed(&mut self, key: keypad::KeypadInput) -> bool {
        self.hardware.keypad.is_pressed(key)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GbaSystemState {
    Running = 0,
    Halted = 1,
    Stopped = 2,
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

pub struct NoVideoOutput;
pub struct NoAudioOutput;

impl GbaVideoOutput for NoVideoOutput {
    fn pre_frame(&mut self) {
        /* NOP */
    }
    fn post_frame(&mut self) {
        /* NOP */
    }
    fn display_line(&mut self, _line: u32, _pixels: &[u16]) {
        /* NOP */
    }
}

impl GbaAudioOutput for NoAudioOutput {
    fn play_samples(&mut self) {
        /* NOP */
    }
}
