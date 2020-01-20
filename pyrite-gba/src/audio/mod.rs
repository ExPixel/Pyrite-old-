pub struct GbaAudio {
    pub registers: GbaAudioRegisters,
}

impl GbaAudio {
    pub fn new() -> GbaAudio {
        GbaAudio {
            registers: GbaAudioRegisters::default(),
        }
    }
}

#[derive(Default)]
pub struct GbaAudioRegisters {
    pub bias: SoundBias,
}

bitfields! (SoundBias: u32 {
    bias_level, set_bias_level: u32 = [1, 9],
    amplitude, set_amplitude: u32 = [14, 15],
});
