use crate::hardware::HardwareEventQueue;
use pyrite_arm::{ArmCpu, ArmMemory};

pub struct GbaDMA {
    channels: [DMAChannel; 4],
    active_channels: u8,
}

impl GbaDMA {
    pub fn new() -> GbaDMA {
        GbaDMA {
            channels: [
                DMAChannel::new(DMAChannelIndex::DMA0),
                DMAChannel::new(DMAChannelIndex::DMA1),
                DMAChannel::new(DMAChannelIndex::DMA2),
                DMAChannel::new(DMAChannelIndex::DMA3),
            ],
            active_channels: 0,
        }
    }

    #[inline]
    pub fn active(&self) -> bool {
        self.active_channels == 0
    }

    pub fn begin_transfer(&mut self, channel_index: DMAChannelIndex) {
        todo!();
    }

    pub fn channel_mut(&mut self, channel_index: DMAChannelIndex) -> &mut DMAChannel {
        match channel_index {
            DMAChannelIndex::DMA0 => &mut self.channels[0],
            DMAChannelIndex::DMA1 => &mut self.channels[1],
            DMAChannelIndex::DMA2 => &mut self.channels[2],
            DMAChannelIndex::DMA3 => &mut self.channels[3],
        }
    }

    pub fn cpu_step_override(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, _op: u32) -> u32 {
        todo!("DMA EXECUTION");
    }
}

pub struct DMAChannel {
    index: DMAChannelIndex,
    source: u32,
    destination: u32,
    count: u32,
    control: DMAControl,
}

impl DMAChannel {
    pub fn new(index: DMAChannelIndex) -> DMAChannel {
        DMAChannel {
            index: index,
            source: 0,
            destination: 0,
            count: 0,
            control: DMAControl::default(),
        }
    }

    pub fn set_source(&mut self, new_source: u32) {
        todo!("DMAChannel::set_source 0x{:08X}", new_source);
    }

    pub fn set_destination(&mut self, new_destination: u32) {
        todo!("DMAChannel::set_destination 0x{:08X}", new_destination);
    }

    pub fn set_source_lo(&mut self, new_source_lo: u16) {
        let new_source = (self.source & 0xFFFF0000) | new_source_lo as u32;
        self.set_source(new_source);
    }

    pub fn set_source_hi(&mut self, new_source_hi: u16) {
        let new_source = (self.source & 0x0000FFFF) | new_source_hi as u32;
        self.set_source(new_source);
    }

    pub fn set_destination_lo(&mut self, new_destination_lo: u16) {
        let new_destination = (self.destination & 0xFFFF0000) | new_destination_lo as u32;
        self.set_destination(new_destination);
    }

    pub fn set_destination_hi(&mut self, new_destination_hi: u16) {
        let new_destination = (self.destination & 0x0000FFFF) | new_destination_hi as u32;
        self.set_destination(new_destination);
    }

    pub fn set_count(&mut self, new_count: u16) {
        if new_count == 0 {
            if self.index == DMAChannelIndex::DMA3 {
                self.count = 0x10000;
            } else {
                self.count = 0x4000;
            }
        } else {
            self.count = new_count as u32;
        }
    }

    pub fn set_control(&mut self, new_control: u16, hw_events: &mut HardwareEventQueue) {
        todo!("DMAChannel::set_control 0x{:04X}", new_control);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DMAChannelIndex {
    DMA0,
    DMA1,
    DMA2,
    DMA3,
}

bitfields! (DMAControl: u16 { 
    dst_control, set_dst_control: DMAAddressControl = [5, 6],
    src_control, set_src_control: DMAAddressControl = [7, 8],
    repeat, set_repeat: bool = [9, 9],
    transfer_type, set_transfer_type: DMATransferType = [10, 10],
    gamepak_drq, set_gamepak_drq: bool = [11, 11],
    start_timing, set_start_timing: DMAStartTiming = [12, 13],
    irq, set_irq: bool = [14, 14],
    enabled, set_enabled: bool = [15, 15],
});

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DMAStartTiming {
    Immediate,
    VBlank,
    HBlank,
    Special,
}

impl_enum_bitfield_conv!(
    DMAStartTiming: u16,
    Immediate = 0,
    VBlank = 1,
    HBlank = 2,
    Special = 3,
);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DMATransferType {
    Halfword = 0,
    Word = 1,
}

impl_enum_bitfield_conv!(DMATransferType: u16, Halfword = 0, Word = 1,);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DMAAddressControl {
    Increment = 0,
    Decrement = 1,
    Fixed = 2,
    IncReload = 3,
}

impl_enum_bitfield_conv!(
    DMAAddressControl: u16,
    Increment = 0,
    Decrement = 1,
    Fixed = 2,
    IncReload = 3,
);
