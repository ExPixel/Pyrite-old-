use miniaudio::{
    Device, DeviceConfig, DeviceType, Format, FramesMut, RingBufferRecv, RingBufferSend,
};
use pyrite_gba::audio::{NoiseState, SquareWaveState, WaveOutputState};
use pyrite_gba::GbaAudioOutput;

const RB_SUBBUFFER_LEN: usize = 8;
const RB_SUBBUFFER_COUNT: usize = 32;

/// Abstraction used to output sound.
pub struct PlatformAudio {
    device: Device,
    messages: RingBufferSend<GbaAudioMessage>,
}

impl PlatformAudio {
    const DEVICE_FORMAT: Format = Format::F32;
    const DEVICE_CHANNELS: u32 = 2;
    const DEVICE_SAMPLE_RATE: u32 = miniaudio::SAMPLE_RATE_44100;

    pub fn new() -> PlatformAudio {
        let mut device_config = DeviceConfig::new(DeviceType::Playback);
        device_config.playback_mut().set_format(Self::DEVICE_FORMAT);
        device_config
            .playback_mut()
            .set_channels(Self::DEVICE_CHANNELS);
        device_config.set_sample_rate(Self::DEVICE_SAMPLE_RATE);

        let (msg_send, msg_recv) = miniaudio::ring_buffer(RB_SUBBUFFER_LEN, RB_SUBBUFFER_COUNT)
            .expect("failed to create audio message ring buffer");

        let mut gba_playback = GbaAudioPlayback::new(msg_recv);
        device_config.set_data_callback(move |_device, output, _input| {
            gba_playback.output_frames(output);
        });

        device_config.set_stop_callback(|_device| {
            log::info!("stopped audio device");
        });

        let device = Device::new(None, &device_config).expect("failed to open playback device");
        device.start().expect("failed to start playback device");

        log::info!("started audio device");
        log::info!("device playback format: {:?}", device.playback().format());
        log::info!("device playback channels: {}", device.playback().channels());
        log::info!("device sample rate: {}", device.sample_rate());

        PlatformAudio {
            device: device,
            messages: msg_send,
        }
    }

    pub fn init(&mut self) {
        /* NOP */
    }

    pub fn set_paused(&mut self, _paused: bool) {
        // TODO
    }

    /// Push some samples to be played.
    pub fn push_samples(&mut self, _samples: &[u16]) {
        // TODO
    }

    fn try_send_message(&self, message: GbaAudioMessage) -> bool {
        // This should usually just work on the first attempt.
        for _ in 0..RB_SUBBUFFER_COUNT {
            if self.messages.write(&[message]) > 0 {
                return true;
            }
        }
        log::error!("UH-OH, AUDIO EMERGENCY");
        return false;
    }
}

impl GbaAudioOutput for PlatformAudio {
    fn set_tone_sweep_state(&mut self, state: SquareWaveState) {
        self.try_send_message(GbaAudioMessage::Channel0(state));
    }

    fn set_tone_state(&mut self, state: SquareWaveState) {
        self.try_send_message(GbaAudioMessage::Channel1(state));
    }

    fn set_wave_output_state(&mut self, _state: WaveOutputState) {
        /* NOP */
    }
    fn set_noise_state(&mut self, _state: NoiseState) {
        /* NOP */
    }

    fn play_samples(&mut self) {
        /* NOP */
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GbaAudioMessage {
    /// Request to change the state of channel0's square wave.
    Channel0(SquareWaveState),

    /// Request to change the state of channel1's square wave.
    Channel1(SquareWaveState),
}

#[derive(Clone)]
pub struct GbaAudioPlayback {
    square_wave_0: SquareWave,
    square_wave_1: SquareWave,
    messages: Option<RingBufferRecv<GbaAudioMessage>>,
}

impl GbaAudioPlayback {
    pub fn new(messages: RingBufferRecv<GbaAudioMessage>) -> GbaAudioPlayback {
        GbaAudioPlayback {
            square_wave_0: SquareWave::new(PlatformAudio::DEVICE_SAMPLE_RATE, 64.0, 0.0, 0.5),
            square_wave_1: SquareWave::new(PlatformAudio::DEVICE_SAMPLE_RATE, 64.0, 0.0, 0.5),
            messages: Some(messages),
        }
    }

    fn process_messages(&mut self) {
        // I do all of this so I can read messages without copying into an intermediary buffer
        // first. But basically I make the messages unavailable while they are being processed
        // by using an Option like this.
        let messages = self.messages.take().unwrap();
        for _ in 0..(RB_SUBBUFFER_COUNT / 2) {
            messages.read_with(RB_SUBBUFFER_LEN, |buf| {
                buf.iter()
                    .copied()
                    .for_each(|msg| self.process_single_message(msg));
            });
        }
        self.messages = Some(messages);
    }

    fn process_single_message(&mut self, message: GbaAudioMessage) {
        match message {
            GbaAudioMessage::Channel0(state) => {
                if state.playing() {
                    self.square_wave_0.amplitude = state.volume_setting().amplitude() as f32;
                } else {
                    self.square_wave_0.amplitude = 0.0;
                }

                self.square_wave_0.frequency = state.freq_setting().frequency();
                self.square_wave_0.duty_cycle = state.duty_cycle().wave_duty();
            }

            GbaAudioMessage::Channel1(state) => {
                if state.playing() {
                    self.square_wave_1.amplitude = state.volume_setting().amplitude() as f32;
                } else {
                    self.square_wave_1.amplitude = 0.0;
                }

                self.square_wave_1.frequency = state.freq_setting().frequency();
                self.square_wave_1.duty_cycle = state.duty_cycle().wave_duty();
            }
        }
    }

    pub fn output_frames(&mut self, output: &mut FramesMut) {
        self.process_messages();

        self.square_wave_0.add_pcm_frames(output);
        self.square_wave_1.add_pcm_frames(output);

        // Normalize the audio.
        output
            .as_samples_mut::<f32>()
            .iter_mut()
            .for_each(|s| *s *= 0.5);
    }
}

#[derive(Clone)]
pub struct SquareWave {
    /// Number of seconds that have passed.
    time: f64,

    /// How many seconds (or fractions of a second usually) pass after each sample.
    /// This is set based the sample rate that is used during construction.
    advance: f64,

    frequency: f64,

    /// The value of the signal when it is high.
    amplitude: f32,

    /// How much of each period the signal is set high for.
    /// e.g. If this is set to 75% (0.75) for a wave with a frequency of 1Hz,
    /// The signal will be on for 3/4 of a second and then off for 1/4
    /// of a second (this pattern will be repeated once a second).
    /// A duty cycle of 100% or 1.0 just generates a signal
    /// that is always on.
    duty_cycle: f64,
}

impl SquareWave {
    pub fn new(sample_rate: u32, frequency: f64, amplitude: f32, duty_cycle: f64) -> SquareWave {
        SquareWave {
            time: 0.0,
            advance: 1.0 / (sample_rate as f64),
            frequency,
            amplitude,
            duty_cycle,
        }
    }

    /// Reads PCM frames from the square wave and adds them to `output_frames`.
    /// This function expects that the format of `output_frames` is f32.
    pub fn add_pcm_frames(&mut self, output_frames: &mut FramesMut) {
        let d = 1.0 - self.duty_cycle;
        for frame in output_frames.frames_mut::<f32>() {
            let v = if (self.time * self.frequency).fract() < d {
                -self.amplitude
            } else {
                self.amplitude
            };
            self.time += self.advance;

            for sample in frame.iter_mut() {
                *sample += v;
            }
        }
    }
}
