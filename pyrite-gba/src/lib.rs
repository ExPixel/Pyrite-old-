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
                    CpuException::SWI => false,
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

        // Initialized by hardware to this value:
        self.hardware.sysctl.set_imemctl(0x0D000020);

        if skip_bios {
            // TODO this is supposed to be initialized to 0x0000 but I don't know of the BIOS changes it
            // so for now I'm just initializing it to the most common value:
            self.hardware.sysctl.set_reg_waitcnt(0x4317);

            let _ = self.cpu.set_pc(0x08000000, &mut self.hardware); // Start at the beginning of the ROM
            self.cpu.registers.setf_i(); // Disables IRQ interrupts
            self.cpu.registers.write_mode(registers::CpuMode::System);

            // Set up user stack base address:
            self.cpu
                .registers
                .write_with_mode(registers::CpuMode::User, 13, 0x03007F00);

            // Set up interrupt stack base address:
            self.cpu
                .registers
                .write_with_mode(registers::CpuMode::IRQ, 13, 0x03007FA0);

            // Set up BIOS stack base address:
            self.cpu
                .registers
                .write_with_mode(registers::CpuMode::Supervisor, 13, 0x03007FE0);

            // Set the post boot flag:
            self.hardware.sysctl.reg_postflg = true;
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
        audio: &mut dyn GbaAudioOutput,
    ) -> (bool, bool) {
        if self.hardware.events.count() > 0 {
            self.process_all_hardware_events();
        }

        // NOTE the call to `cpu.step` here is kind of misleading.
        // Despite `step` being only one line:
        //
        //     (self.decoded_fn)(self, memory, self.decoded_op)
        //
        // It handles running instructions, DMAs, exceptions, and CPU idling when the CPU is in the
        // halted or stopped state. `decoded_fn` does not always actually refer to code meant for
        // running a CPU instruction but can also just be some other arbitrary function set
        // somewhere else in the emulator. The destination of `decoded_fn` is changed via the
        // `set_idle` and `override_execution` functions.
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

        let audio_frame = self.hardware.audio.step(
            cycles,
            audio,
            &mut self.hardware.dma,
            &mut self.hardware.events,
        );

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
                if self.cpu.exception_enabled(CpuException::IRQ) && self.hardware.irq.request(irq) {
                    self.state = GbaSystemState::Running;

                    // This will automatically put the CPU in an active state if it's idling,
                    // and then change the next execution target to the exception handler.
                    self.cpu.set_pending_exception_active(CpuException::IRQ);

                    // Calling `set_pending_exception_active` will change the CPU's next execution
                    // so we call `resume_transfer` to resume a DMA transfer if one was in
                    // progress. It doesn't matter that this will override the exception because
                    // the CPU will "remember" that there is an exception waiting to be
                    // processed the next time we try to resume regular execution.
                    self.hardware.dma.resume_transfer(&mut self.cpu);
                }
            }

            HardwareEvent::DMA(dma) => {
                self.hardware.dma.begin_transfer(dma, &mut self.cpu);
            }

            HardwareEvent::Halt => {
                self.state = GbaSystemState::Halted;

                // We don't want to be too fine grained here or performance is bad.
                self.cpu.set_idle(true, 4);
            }

            HardwareEvent::Stop => {
                self.state = GbaSystemState::Stopped;

                // we use big steps for stop because everything we need high fidelity for is off.
                self.cpu.set_idle(true, 16);
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
    fn display_line(&mut self, line: u32, pixels: &[u16; 240]);
}

pub trait GbaAudioOutput {
    /// Sets the state of the programmable sound generators.
    fn set_tone_sweep_state(&mut self, state: audio::SquareWaveState);
    fn set_tone_state(&mut self, state: audio::SquareWaveState);
    fn set_wave_output_state(&mut self, state: audio::WaveOutputState);
    fn set_noise_state(&mut self, state: audio::NoiseState);

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
    fn display_line(&mut self, _line: u32, _pixels: &[u16; 240]) {
        /* NOP */
    }
}

impl GbaAudioOutput for NoAudioOutput {
    fn set_tone_sweep_state(&mut self, _state: audio::SquareWaveState) {
        /* NOP */
    }
    fn set_tone_state(&mut self, _state: audio::SquareWaveState) {
        /* NOP */
    }
    fn set_wave_output_state(&mut self, _state: audio::WaveOutputState) {
        /* NOP */
    }
    fn set_noise_state(&mut self, _state: audio::NoiseState) {
        /* NOP */
    }

    fn play_samples(&mut self) {
        /* NOP */
    }
}
