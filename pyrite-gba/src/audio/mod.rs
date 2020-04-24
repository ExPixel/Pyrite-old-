// use crate::dma::GbaDMA;
// use crate::irq::Interrupt;
use crate::scheduler::{GbaEvent, SharedGbaScheduler};
use crate::GbaAudioOutput;
use pyrite_common::bits_set;

const CYCLES_PER_SECOND: u32 = 16 * 1024 * 1024;

pub struct GbaAudio {
    scheduler: SharedGbaScheduler,
    pub registers: GbaAudioRegisters,
    /// Cycles accumulated while GbaAudio is idling.
    dirty: u8,
    channel1: SquareWave,
    channel2: SquareWave,
}

impl GbaAudio {
    pub fn new(scheduler: SharedGbaScheduler) -> GbaAudio {
        GbaAudio {
            scheduler: scheduler,
            registers: GbaAudioRegisters::default(),
            dirty: 0,
            channel1: SquareWave::new(),
            channel2: SquareWave::new(),
        }
    }

    pub fn update(&mut self, audio: &mut dyn GbaAudioOutput) {
        if self.psg_channel_dirty(PSGChannel::ToneSweep) {
            audio.set_tone_sweep_state(self.channel1.state());
        }

        if self.psg_channel_dirty(PSGChannel::Tone) {
            audio.set_tone_state(self.channel2.state());
        }
        self.dirty = 0;
    }

    fn set_psg_channel_dirty(&mut self, channel: PSGChannel) {
        if self.dirty == 0 {
            self.scheduler.schedule(GbaEvent::AudioUpdate, 0);
        }
        self.dirty |= 1 << channel.index8();
    }

    fn clear_psg_channel_dirty(&mut self, channel: PSGChannel) {
        self.dirty &= !(1 << channel.index8());
    }

    fn psg_channel_dirty(&self, channel: PSGChannel) -> bool {
        ((self.dirty >> channel.index8()) & 1) != 0
    }

    /// Sweep shift for channel 1.
    pub(crate) fn psg_sweep_shift(&mut self, audio: &mut dyn GbaAudioOutput) {
        // If channel 1 was stopped before we got here then we can stop scheduling and bail.
        if !self.channel1.playing {
            return;
        }

        let continue_shifting = self.channel1.freq_setting.sweep_shift(
            self.registers.sound1cnt_l.sweep_direction(),
            self.registers.sound1cnt_l.sweep_shifts(),
        );

        if continue_shifting {
            self.scheduler.schedule(
                GbaEvent::PSGChannel0StepSweep,
                self.registers.sound1cnt_l.sweep_time().cycles(),
            );
        } else if self.registers.sound1cnt_l.sweep_direction() == SweepDirection::Increase {
            // When we reach the limit on sweep increase sound is just stopped.
            self.channel1.playing = false;
        }
        // When we reach the limit on sweep decrease, the last frequency is retained.
        // So we don't have to do anything :P

        audio.set_tone_sweep_state(self.channel1.state());
        self.clear_psg_channel_dirty(PSGChannel::ToneSweep);
    }

    pub(crate) fn psg_stop_channel(&mut self, audio: &mut dyn GbaAudioOutput, channel: PSGChannel) {
        match channel {
            PSGChannel::ToneSweep => {
                self.channel1.playing = false;
                audio.set_tone_sweep_state(self.channel1.state());
                self.clear_psg_channel_dirty(PSGChannel::ToneSweep);
            }
            PSGChannel::Tone => {
                self.channel2.playing = false;
                audio.set_tone_state(self.channel2.state());
                self.clear_psg_channel_dirty(PSGChannel::Tone);
            }
            PSGChannel::WaveOutput => { /* FIXME implement this */ }
            PSGChannel::Noise => { /* FIXME implement this */ }
        }
    }

    pub(crate) fn psg_envelope_step(
        &mut self,
        audio: &mut dyn GbaAudioOutput,
        channel: PSGChannel,
    ) {
        let mut continue_stepping = false;
        let mut step_cycles = 0;

        match channel {
            PSGChannel::ToneSweep => {
                if self.channel1.playing {
                    if self.registers.sound1cnt_h.envelope_direction()
                        == EnvelopeDirection::Increase
                    {
                        continue_stepping = self.channel1.volume.increase();
                    } else {
                        continue_stepping = self.channel1.volume.decrease();
                    }

                    self.channel1.playing = self.channel1.volume.0 > 0;

                    step_cycles = self.registers.sound1cnt_h.envelope_step_time().cycles();
                    audio.set_tone_sweep_state(self.channel1.state());
                    self.clear_psg_channel_dirty(PSGChannel::ToneSweep);
                }
            }
            PSGChannel::Tone => {
                if self.channel2.playing {
                    if self.registers.sound2cnt_l.envelope_direction()
                        == EnvelopeDirection::Increase
                    {
                        continue_stepping = self.channel2.volume.increase();
                    } else {
                        continue_stepping = self.channel2.volume.decrease();
                    }

                    self.channel2.playing = self.channel2.volume.0 > 0;

                    step_cycles = self.registers.sound2cnt_l.envelope_step_time().cycles();
                    audio.set_tone_state(self.channel2.state());
                    self.clear_psg_channel_dirty(PSGChannel::Tone);
                }
            }
            PSGChannel::WaveOutput => { /* FIXME implement this */ }
            PSGChannel::Noise => { /* FIXME implement this */ }
        }

        if continue_stepping {
            self.scheduler
                .schedule(GbaEvent::PSGChannelStepEnvelope(channel), step_cycles);
        }
    }

    pub(crate) fn set_nr10(&mut self, value: u8) {
        self.registers.sound1cnt_l.value =
            bits_set!(self.registers.sound1cnt_l.value, value as u16, 0, 7);

        if self.channel1.playing {
            self.scheduler.purge(GbaEvent::PSGChannel0StepSweep);

            if self.registers.sound1cnt_l.sweep_shifts() > 0 {
                self.scheduler.schedule(
                    GbaEvent::PSGChannel0StepSweep,
                    self.registers.sound1cnt_l.sweep_time().cycles(),
                );
            }
        }
    }

    pub(crate) fn set_nr11(&mut self, value: u8) {
        self.registers.sound1cnt_h.value =
            bits_set!(self.registers.sound1cnt_h.value, value as u16, 0, 7);

        if self.channel1.playing {
            // FIXME implement sound length
            self.channel1.duty_cycle = self.registers.sound1cnt_h.wave_pattern_duty();
            self.set_psg_channel_dirty(PSGChannel::ToneSweep);
        }
    }

    pub(crate) fn set_nr12(&mut self, value: u8) {
        self.registers.sound1cnt_h.value =
            bits_set!(self.registers.sound1cnt_h.value, value as u16, 8, 15);

        if self.channel1.playing {
            self.scheduler
                .purge(GbaEvent::PSGChannelStepEnvelope(PSGChannel::ToneSweep));

            // If it's a step time of 0, we just want to stop the envelope and skip scheduling
            // another step.
            if self.registers.sound1cnt_h.envelope_step_time().0 > 0 {
                self.scheduler.schedule(
                    GbaEvent::PSGChannelStepEnvelope(PSGChannel::ToneSweep),
                    self.registers.sound1cnt_h.envelope_step_time().cycles(),
                );
            }

            self.set_psg_channel_dirty(PSGChannel::ToneSweep);
        } else {
            // Envelope initial volume is only used when sound is first initialized, unless the
            // value is set to 0, in which case the sound will be stopped. So here we only change
            // the volume on the channel while it is disabled.
            self.channel1.volume = self.registers.sound1cnt_h.envelope_initial_volume();
        }
    }

    pub(crate) fn set_nr13(&mut self, value: u8) {
        self.registers.sound1cnt_x.value =
            bits_set!(self.registers.sound1cnt_x.value, value as u16, 0, 7);
    }

    pub(crate) fn set_nr14(&mut self, value: u8) {
        self.registers.sound1cnt_x.value =
            bits_set!(self.registers.sound1cnt_x.value, value as u16, 8, 15);

        self.channel1.freq_setting = self.registers.sound1cnt_x.freq_setting();
        if self.registers.sound1cnt_x.init() {
            self.channel1.duty_cycle = self.registers.sound1cnt_h.wave_pattern_duty();
            self.channel1.volume = self.registers.sound1cnt_h.envelope_initial_volume();

            self.channel1.playing = true;

            if self.registers.sound1cnt_h.envelope_step_time().0 > 0 {
                self.scheduler.schedule_unique(
                    GbaEvent::PSGChannelStepEnvelope(PSGChannel::ToneSweep),
                    self.registers.sound1cnt_h.envelope_step_time().cycles(),
                );
            }

            if self.registers.sound1cnt_x.length_flag() {
                self.scheduler.schedule_unique(
                    GbaEvent::StopPSGChannel(PSGChannel::ToneSweep),
                    self.registers.sound1cnt_h.length().cycles(),
                );
            }

            if self.registers.sound1cnt_l.sweep_shifts() > 0 {
                self.scheduler.schedule_unique(
                    GbaEvent::PSGChannel0StepSweep,
                    self.registers.sound1cnt_l.sweep_time().cycles(),
                );
            }

            self.registers.sound1cnt_x.set_init(false);
        }
        self.set_psg_channel_dirty(PSGChannel::ToneSweep);
    }

    pub fn set_nr21(&mut self, value: u8) {
        self.registers.sound2cnt_l.value =
            bits_set!(self.registers.sound2cnt_l.value, value as u16, 0, 7);

        if self.channel2.playing {
            // FIXME implement sound length
            self.channel2.duty_cycle = self.registers.sound2cnt_l.wave_pattern_duty();
            self.set_psg_channel_dirty(PSGChannel::Tone);
        }
    }

    pub fn set_nr22(&mut self, value: u8) {
        let prev_sound2cnt_l = self.registers.sound2cnt_l;

        self.registers.sound2cnt_l.value =
            bits_set!(self.registers.sound2cnt_l.value, value as u16, 8, 15);

        if self.channel2.playing {
            // If envelope step time was changed while sound is playing, reschedule the envelope
            // steps.
            if prev_sound2cnt_l.envelope_step_time()
                != self.registers.sound2cnt_l.envelope_step_time()
            {
                self.scheduler
                    .purge(GbaEvent::PSGChannelStepEnvelope(PSGChannel::Tone));

                // If it's a step time of 0, we just want to stop the envelope and skip scheduling
                // another step.
                if self.registers.sound2cnt_l.envelope_step_time().0 > 0 {
                    self.scheduler.schedule(
                        GbaEvent::PSGChannelStepEnvelope(PSGChannel::Tone),
                        self.registers.sound2cnt_l.envelope_step_time().cycles(),
                    );
                }
            }

            self.set_psg_channel_dirty(PSGChannel::Tone);
        } else {
            // Envelope initial volume is only used when sound is first initialized, unless the
            // value is set to 0, in which case the sound will be stopped. So here we only change
            // the volume on the channel while it is disabled.
            self.channel2.volume = self.registers.sound2cnt_l.envelope_initial_volume();
        }
    }

    pub fn set_nr23(&mut self, value: u8) {
        self.registers.sound2cnt_h.value =
            bits_set!(self.registers.sound2cnt_h.value, value as u16, 0, 7);
        self.channel2.freq_setting = self.registers.sound2cnt_h.freq_setting();

        if self.channel2.playing {
            self.set_psg_channel_dirty(PSGChannel::Tone);
        }
    }

    pub fn set_nr24(&mut self, value: u8) {
        self.registers.sound2cnt_h.value =
            bits_set!(self.registers.sound2cnt_h.value, value as u16, 8, 15);

        self.channel2.freq_setting = self.registers.sound2cnt_h.freq_setting();
        if self.registers.sound2cnt_h.init() {
            self.channel2.duty_cycle = self.registers.sound2cnt_l.wave_pattern_duty();
            self.channel2.volume = self.registers.sound2cnt_l.envelope_initial_volume();

            self.channel2.playing = true;

            if self.registers.sound2cnt_l.envelope_step_time().0 > 0 {
                self.scheduler.schedule_unique(
                    GbaEvent::PSGChannelStepEnvelope(PSGChannel::Tone),
                    self.registers.sound2cnt_l.envelope_step_time().cycles(),
                );
            }

            if self.registers.sound2cnt_h.length_flag() {
                self.scheduler.schedule_unique(
                    GbaEvent::StopPSGChannel(PSGChannel::Tone),
                    self.registers.sound2cnt_l.length().cycles(),
                );
            }

            self.registers.sound2cnt_h.set_init(false);
        }

        if self.channel2.playing {
            self.set_psg_channel_dirty(PSGChannel::Tone);
        }
    }

    pub fn set_nr30(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr31(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr32(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr33(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr34(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr41(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr42(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr43(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr44(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr50(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr51(&mut self, _value: u8) {
        // FIXME todo
    }

    pub fn set_nr52(&mut self, value: u8) {
        let enable = (value & 0x80) != 0;

        // If sound is being turned off: zero all of the registers and
        // mark all of the sound outputs as dirty.
        if !enable && self.registers.soundcnt_x.master_enable() {
            self.registers.zero_sound_registers();
            self.dirty = 0xFF;
        }

        self.registers.soundcnt_x.set_master_enable(enable);
    }

    pub fn set_wave_ram_byte(&mut self, _offset: u16, _data: u8) {
        // TODO
    }

    pub fn wave_ram_byte(&self, _offset: u16) -> u8 {
        0
    }

    pub(crate) fn set_soundcnt_h(&mut self, value: u16) {
        self.registers.soundcnt_h.value = value;
    }

    pub(crate) fn set_sound_bias(&mut self, value: u16) {
        self.registers.bias.value = value;
    }
}

pub struct SquareWave {
    freq_setting: SquareFreqSetting,
    duty_cycle: SquareWaveDutyCycle,
    volume: PSGVolume,
    playing: bool,
}

impl SquareWave {
    pub fn new() -> SquareWave {
        SquareWave {
            freq_setting: SquareFreqSetting(0),
            volume: PSGVolume(0),
            playing: false,
            duty_cycle: SquareWaveDutyCycle(2),
        }
    }

    pub fn state(&self) -> SquareWaveState {
        let mut state = SquareWaveState::default();
        if self.playing {
            state.set_playing(true);
            state.set_freq_setting(self.freq_setting);
            state.set_duty_cycle(self.duty_cycle);
            state.set_volume_setting(self.volume);
        } else {
            state.set_playing(false);
        }
        state
    }
}

#[derive(Default)]
pub struct GbaAudioRegisters {
    pub bias: SoundBias,
    pub soundcnt_l: PSGSoundControl,
    pub soundcnt_h: DMASoundControl,
    pub soundcnt_x: SoundEnable,

    pub sound1cnt_l: SquarePSGSweep,
    pub sound1cnt_h: SquarePSGControlLo,
    pub sound1cnt_x: SquarePSGControlHi,

    pub sound2cnt_l: SquarePSGControlLo,
    pub sound2cnt_h: SquarePSGControlHi,

    pub sound3cnt_l: UnimplementedSound,
    pub sound3cnt_h: UnimplementedSound,
    pub sound3cnt_x: UnimplementedSound,

    pub sound4cnt_l: UnimplementedSound,
    pub sound4cnt_h: UnimplementedSound,
}

impl GbaAudioRegisters {
    /// Called when master enable is set to zero.
    /// All sound registers are zeroed and must be reinitialized.
    pub fn zero_sound_registers(&mut self) {
        self.soundcnt_l.value = 0;

        self.sound1cnt_l.value = 0;
        self.sound1cnt_h.value = 0;
        self.sound1cnt_x.value = 0;

        self.sound2cnt_l.value = 0;
        self.sound2cnt_h.value = 0;

        self.sound3cnt_l.value = 0;
        self.sound3cnt_h.value = 0;
        self.sound3cnt_x.value = 0;

        self.sound4cnt_l.value = 0;
        self.sound4cnt_l.value = 0;
    }
}

bitfields! (UnimplementedSound: u16 {
    // TODO find these and implement them.
});

bitfields! (SquareWaveState: u32 {
    playing, set_playing: bool = [0, 0],
    freq_setting, set_freq_setting: SquareFreqSetting = [1, 11],
    volume_setting, set_volume_setting: PSGVolume = [12, 15],
    duty_cycle, set_duty_cycle: SquareWaveDutyCycle = [16, 17],
});

bitfields! (WaveOutputState: u32 {
});

bitfields! (NoiseState: u32 {
});

bitfields! (PSGSoundControl: u16 {
    master_volume_right, set_master_volume_right: u8 = [0, 2],
    master_volume_left, set_master_volume_left: u8 = [4, 6],
});

impl PSGSoundControl {
    pub fn enabled_right(&self, channel: PSGChannel) -> bool {
        (self.value >> (channel.index16() + 8)) & 1 != 0
    }

    pub fn set_enabled_right(&mut self, channel: PSGChannel, enabled: bool) {
        if enabled {
            self.value |= 1 << (channel.index16() + 8);
        } else {
            self.value &= !(1 << (channel.index16() + 8));
        }
    }

    pub fn enabled_left(&self, channel: PSGChannel) -> bool {
        (self.value >> (channel.index16() + 12)) & 1 != 0
    }

    pub fn set_enabled_left(&mut self, channel: PSGChannel, enabled: bool) {
        if enabled {
            self.value |= 1 << (channel.index16() + 12);
        } else {
            self.value &= !(1 << (channel.index16() + 12));
        }
    }
}

bitfields! (DMASoundControl: u16 {
});

bitfields! (SoundEnable: u16 {
    master_enable, set_master_enable: bool = [7, 7],
});

impl SoundEnable {
    pub fn psg_enabled(&self, channel: PSGChannel) -> bool {
        (self.value >> channel.index16()) & 1 != 0
    }

    pub fn set_psg_enabled(&mut self, channel: PSGChannel, enabled: bool) {
        if enabled {
            self.value |= 1 << channel.index16();
        } else {
            self.value &= !(1 << channel.index16());
        }
    }
}

bitfields! (SoundBias: u16 {
    bias_level, set_bias_level: u32 = [1, 9],
    amplitude, set_amplitude: u32 = [14, 15],
});

// Sound Sweep Register
// SOUND1CNT_L (NR10)
bitfields! (SquarePSGSweep: u16 {
    sweep_shifts, set_sweep_shifts: u16 = [0, 2],
    sweep_direction, set_sweep_direction: SweepDirection = [3, 3],
    sweep_time, set_sweep_time: SweepTime = [4, 6],
});

// Low sound control registers.
// SOUND1CNT_H (NR11, NR12)
// SOUND2CNT_L (NR21, NR22)
bitfields! (SquarePSGControlLo: u16 {
    length, set_length: SoundLength = [0, 5],
    wave_pattern_duty, set_wave_pattern_duty: SquareWaveDutyCycle = [6, 7],
    envelope_step_time, set_envelope_step_time: EnvelopeStepTime = [8, 10],
    envelope_direction, set_envelope_direction: EnvelopeDirection = [11, 11],
    envelope_initial_volume, set_envelope_initial_volume: PSGVolume = [12, 15],
});

// Hi sound control registers.
// SOUND1CNT_X (NR13, NR14)
// SOUND2CNT_H (NR23, NR24)
bitfields! (SquarePSGControlHi: u16 {
    freq_setting, set_freq_setting: SquareFreqSetting = [0, 10],
    length_flag, set_length_flag: bool = [14, 14],
    init, set_init: bool = [15, 15],
});

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SweepDirection {
    Increase,
    Decrease,
}
impl_enum_bitfield_conv!(SweepDirection: u16, Increase = 0, Decrease = 1,);

/// Sweep time in units of 7.8ms (128KHz)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SweepTime(u8);

impl SweepTime {
    /// The number of cycles between each sweep shift.
    pub fn cycles(self) -> u32 {
        self.0 as u32 * 128 * 1024
    }
}

impl_unit_struct_field_convert!(SweepTime, u16);

/// Sound length (units of (64-n)/256s)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SoundLength(u8);

impl SoundLength {
    /// The number of cycles that this this length represents.
    pub fn cycles(self) -> u32 {
        ((64 - self.0 as u32) * CYCLES_PER_SECOND) / 256
    }
}

impl_unit_struct_field_convert!(SoundLength, u16);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EnvelopeDirection {
    Increase,
    Decrease,
}
impl_enum_bitfield_conv!(EnvelopeDirection: u16, Increase = 1, Decrease = 0,);

/// Sound length (units of n/64s)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EnvelopeStepTime(u8);

impl EnvelopeStepTime {
    /// The number of cycles that this this length represents.
    pub fn cycles(self) -> u32 {
        (self.0 as u32 * CYCLES_PER_SECOND) / 64
    }
}

impl_unit_struct_field_convert!(EnvelopeStepTime, u16);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PSGChannel {
    ToneSweep = 0,
    Tone = 1,
    WaveOutput = 2,
    Noise = 3,
}

impl PSGChannel {
    #[inline(always)]
    pub fn from_index(channel_index: usize) -> PSGChannel {
        match channel_index {
            0 => Self::ToneSweep,
            1 => Self::Tone,
            2 => Self::WaveOutput,
            3 => Self::Noise,
            _ => panic!("not a valid PSG channel"),
        }
    }

    #[inline(always)]
    pub const fn index(self) -> usize {
        self as u8 as usize
    }

    #[inline(always)]
    pub const fn index8(self) -> u8 {
        self as u8
    }

    #[inline(always)]
    pub const fn index16(self) -> u16 {
        self as u8 as u16
    }
}

/// Sound length (units of n/64s)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SquareFreqSetting(u16);

impl SquareFreqSetting {
    pub fn frequency(&self) -> f64 {
        131072.0f64 / (2048.0f64 - self.0 as f64)
    }

    pub fn sweep_shift(&mut self, direction: SweepDirection, shifts: u16) -> bool {
        let freq = self.0 as u32;
        let change = (freq as u32) / (1 << shifts as u32);

        if direction == SweepDirection::Increase {
            if freq + change > 0x7FF {
                false
            } else {
                self.0 = (freq + change) as u16;
                true
            }
        } else {
            if change > freq {
                false
            } else {
                self.0 = (freq - change) as u16;
                true
            }
        }
    }
}
impl_unit_struct_field_convert!(SquareFreqSetting, u16);
impl_unit_struct_field_convert!(SquareFreqSetting, u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SquareWaveDutyCycle(u8);

impl SquareWaveDutyCycle {
    pub fn wave_duty(self) -> f64 {
        match self.0 {
            0 => 0.125,
            1 => 0.25,
            2 => 0.5,
            3 => 0.75,

            // FIXME maybe I should just error here, idk.
            _ => 1.0,
        }
    }
}

impl_unit_struct_field_convert!(SquareWaveDutyCycle, u16);
impl_unit_struct_field_convert!(SquareWaveDutyCycle, u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PSGVolume(u8);

impl PSGVolume {
    const VOLUME_STEP: f64 = 1.0 / 15.0;

    pub fn amplitude(&mut self) -> f64 {
        self.0 as f64 * Self::VOLUME_STEP
    }

    pub fn increase(&mut self) -> bool {
        if self.0 < 15 {
            self.0 += 1;
            true
        } else {
            false
        }
    }

    pub fn decrease(&mut self) -> bool {
        if self.0 > 0 {
            self.0 -= 1;
            true
        } else {
            false
        }
    }
}

impl_unit_struct_field_convert!(PSGVolume, u16);
impl_unit_struct_field_convert!(PSGVolume, u32);
