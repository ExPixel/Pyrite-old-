use crate::dma::GbaDMA;
use crate::hardware::HardwareEventQueue;
use crate::irq::Interrupt;
use crate::GbaAudioOutput;

const CYCLES_PER_SECOND: u32 = 16 * 1024 * 1024;

pub struct GbaAudio {
    pub registers: GbaAudioRegisters,

    /// Cycles accumulated while GbaAudio is idling.
    cycles_acc: u32,

    /// Cycles until GbaAudio needs to do something (sweeps, envelope, sound length ect.)
    cycles_to_next_event: u32,

    dirty: bool,

    channel1: SquareWave,
}

impl GbaAudio {
    pub fn new() -> GbaAudio {
        GbaAudio {
            registers: GbaAudioRegisters::default(),
            cycles_acc: 0,
            cycles_to_next_event: std::u32::MAX >> 1,
            dirty: false,
            channel1: SquareWave::new(),
        }
    }

    // FIXME What the fuck is an audio frame? I don't remember what I was thinking about.
    /// Returns true if this step was also the end of an audio frame.
    #[inline]
    pub fn step(
        &mut self,
        partial_cycles: u32,
        audio: &mut dyn GbaAudioOutput,
        dma: &mut GbaDMA,
        hw_events: &mut HardwareEventQueue,
    ) -> bool {
        self.cycles_acc += partial_cycles;

        if self.cycles_acc >= self.cycles_to_next_event {
            self.dirty = true;
            self.cycles_to_next_event = std::u32::MAX;
        }

        if self.dirty {
            self.dirty = false;
            self.step_fire(self.cycles_acc, audio, dma, hw_events);
            self.cycles_acc = 0;
        }

        false
    }

    #[cold]
    fn step_fire(
        &mut self,
        cycles: u32,
        audio: &mut dyn GbaAudioOutput,
        dma: &mut GbaDMA,
        hw_events: &mut HardwareEventQueue,
    ) -> bool {
        audio.set_tone_sweep_state(self.channel1.state());
        false
    }

    pub(crate) fn set_soundcnt_l(&mut self, value: u16) {
        self.registers.soundcnt_l.value = value;
    }

    pub(crate) fn set_soundcnt_h(&mut self, value: u16) {
        self.registers.soundcnt_h.value = value;
    }

    pub(crate) fn set_soundcnt_x(&mut self, value: u16) {
        let last_enable = self.registers.soundcnt_x.master_enable();
        self.registers.soundcnt_x.value &= 0x7F;
        self.registers.soundcnt_x.value |= value & 0x80;

        if last_enable == self.registers.soundcnt_x.master_enable() {
            return;
        }

        if !self.registers.soundcnt_x.master_enable() {
            self.registers.zero_sound_registers();
            self.dirty = true;
        }
    }

    pub(crate) fn set_sound1cnt_l(&mut self, value: u16) {
        self.registers.sound1cnt_l.value = value;
    }

    pub(crate) fn set_sound1cnt_h(&mut self, value: u16) {
        self.registers.sound1cnt_h.value = value;
    }

    pub(crate) fn set_sound1cnt_x(&mut self, value: u16) {
        self.registers.sound1cnt_x.value = value;
        self.channel1.freq_setting = self.registers.sound1cnt_x.freq_setting();
        println!(
            "FREQ SETTING: 0x{:04X}",
            self.registers.sound1cnt_x.value & 0x3FF
        );
        if self.registers.sound1cnt_x.init() {
            self.channel1.enabled = true;
            self.registers.sound1cnt_x.set_init(false);
        }
        self.dirty = true;
    }

    pub(crate) fn set_sound2cnt_l(&mut self, value: u16) {
        self.registers.sound2cnt_l.value = value;
    }

    pub(crate) fn set_sound2cnt_h(&mut self, value: u16) {
        self.registers.sound2cnt_h.value = value;
    }

    pub(crate) fn set_sound3cnt_l(&mut self, value: u16) {
        // TODO
    }

    pub(crate) fn set_sound3cnt_h(&mut self, value: u16) {
        // TODO
    }

    pub(crate) fn set_sound3cnt_x(&mut self, value: u16) {
        // TODO
    }

    pub(crate) fn set_wave_ram_byte(&mut self, index: u16, value: u8) {
        // TODO
    }

    pub(crate) fn read_wave_ram_byte(&self, index: u16) -> u8 {
        // TODO
        0
    }

    pub(crate) fn set_sound4cnt_l(&mut self, value: u16) {
        // TODO
    }

    pub(crate) fn set_sound4cnt_h(&mut self, value: u16) {
        // TODO
    }

    pub(crate) fn set_sound_bias(&mut self, value: u16) {
        self.registers.bias.value = value;
    }
}

pub struct SquareWave {
    freq_setting: SquareFreqSetting,
    enabled: bool,
}

impl SquareWave {
    pub fn new() -> SquareWave {
        SquareWave {
            freq_setting: SquareFreqSetting(0),
            enabled: false,
        }
    }

    pub fn state(&self) -> SquareWaveState {
        let mut state = SquareWaveState::default();
        if self.enabled {
            state.set_enabled(true);
            state.set_freq_setting(self.freq_setting);
        } else {
            state.set_enabled(false);
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

#[derive(Default)]
struct GbaSquareWave {
    /// Frequency setting: 131072/(2048-n)Hz  (0-2047)
    frequency: u16,

    /// Volume setting: 1-15 (0 = no sound)
    volume: u16,
}

bitfields! (SquareWaveState: u32 {
    enabled, set_enabled: bool = [0, 0],
    freq_setting, set_freq_setting: SquareFreqSetting = [1, 11],
    volume_setting, set_volume_setting: u16 = [12, 15],
    duty_cycle, set_duty_cycle: u16 = [16, 17],
});

impl SquareWaveState {
    const VOLUME_STEP: f64 = 1.0f64 / 15.0f64;

    pub fn amplitude(&self) -> f64 {
        Self::VOLUME_STEP * self.volume_setting() as f64
    }
}

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
    shifts, set_shifts: u16 = [0, 2],
    sweep_direction, set_sweep_direction: SweepDirection = [3, 3],
    sweep_time, set_sweep_time: SweepTime = [4, 6],
});

// Low sound control registers.
// SOUND1CNT_H (NR11, NR12)
// SOUND2CNT_L (NR21, NR22)
bitfields! (SquarePSGControlLo: u16 {
    length, set_length: PSGSoundLength = [0, 5],
    wave_pattern_duty, set_wave_pattern_duty: u32 = [0, 3],
    envelope_step_time, set_envelope_step_time: EnvelopeStepTime = [8, 10],
    envelope_direction, set_sweep_direction: EnvelopeDirection = [11, 11],
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

impl crate::util::bitfields::FieldConvert<u16> for SweepTime {
    fn convert(self) -> u16 {
        self.0 as u16
    }
}

impl crate::util::bitfields::FieldConvert<SweepTime> for u16 {
    fn convert(self) -> SweepTime {
        SweepTime(self as u8)
    }
}

/// Sound length (units of (64-n)/256s)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PSGSoundLength(u8);

impl PSGSoundLength {
    /// The number of cycles that this this length represents.
    pub fn cycles(self) -> u32 {
        (64 - self.0 as u32) * (CYCLES_PER_SECOND / 256)
    }
}

impl crate::util::bitfields::FieldConvert<u16> for PSGSoundLength {
    fn convert(self) -> u16 {
        self.0 as u16
    }
}

impl crate::util::bitfields::FieldConvert<PSGSoundLength> for u16 {
    fn convert(self) -> PSGSoundLength {
        PSGSoundLength(self as u8)
    }
}

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
        (self.0 as u32) * (CYCLES_PER_SECOND / 64)
    }
}

impl crate::util::bitfields::FieldConvert<u16> for EnvelopeStepTime {
    fn convert(self) -> u16 {
        self.0 as u16
    }
}

impl crate::util::bitfields::FieldConvert<EnvelopeStepTime> for u16 {
    fn convert(self) -> EnvelopeStepTime {
        EnvelopeStepTime(self as u8)
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
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
pub struct SquareFreqSetting(u8);

impl SquareFreqSetting {
    pub fn frequency(&self) -> f64 {
        131072.0f64 / (2048.0f64 - self.0 as f64)
    }
}

impl crate::util::bitfields::FieldConvert<u16> for SquareFreqSetting {
    fn convert(self) -> u16 {
        self.0 as u16
    }
}

impl crate::util::bitfields::FieldConvert<SquareFreqSetting> for u16 {
    fn convert(self) -> SquareFreqSetting {
        SquareFreqSetting(self as u8)
    }
}

impl crate::util::bitfields::FieldConvert<u32> for SquareFreqSetting {
    fn convert(self) -> u32 {
        self.0 as u32
    }
}

impl crate::util::bitfields::FieldConvert<SquareFreqSetting> for u32 {
    fn convert(self) -> SquareFreqSetting {
        SquareFreqSetting(self as u8)
    }
}
