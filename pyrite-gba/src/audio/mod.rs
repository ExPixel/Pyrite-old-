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

// // Sound Sweep Register
// // SOUND1CNT_L (NR10)
// bitfields! (SoundSweep: u16 {
//     shifts, set_shifts: u16 = [0, 2],
//     direction, set_direction: SweepDirection = [3, 3],
//     sweep_time, set_sweep_time: SweepTime = [4, 6],
// });

// // SoundDelta registers describe how the wave
// // pattern changes over time (length, wave duty, envelope)
// // SOUND1CNT_H (NR11, NR12)
// // SOUND2CNT_L (NR21, NR22)
// bitfields! (SoundDelta: u16 {
// });

// // Main Sound Control
// // SOUND1CNT_X (NR13, NR14)
// // SOUND2CNT_H (NR23, NR24)
// bitfields! (SoundControl: u16 {
// });

// #[derive(Debug, Copy, Clone, PartialEq, Eq)]
// pub enum SweepDirection {
//     Increase,
//     Decrease,
// }
// impl_enum_bitfield_conv!(SweepDirection: u16, Increase = 0, Decrease = 1,);

// /// Sweep time in units of 7.8ms
// #[derive(Debug, Copy, Clone, PartialEq, Eq)]
// pub struct SweepTime(u8);

// impl SweepTime {
//     /// The number of cycles between each sweep shift.
//     pub fn cycles(self) -> u32 {
//         self * 128 * 1024
//     }
// }

// impl crate::util::bitfields::FieldConvert<u16> for SweepTime {
//     fn convert(self) -> u16 {
//         self.0 as u16
//     }
// }

// impl crate::util::bitfields::FieldConvert<SweepTime> for u16 {
//     fn convert(self) -> SweepTime {
//         SweepTime(self as u8)
//     }
// }
