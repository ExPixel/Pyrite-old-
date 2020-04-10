use crate::hardware::{GbaHardware, HardwareEventQueue};
use pyrite_arm::{ArmCpu, ArmMemory};

pub struct GbaDMA {
    channels: [DMAChannel; 4],
    active_channels: u8,

    /// The last data that was transferred. This is used when the source address is invalid.
    dma_bus: u32,
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
            dma_bus: 0,
        }
    }

    pub fn start_hblank(&mut self, hw_events: &mut HardwareEventQueue) {
        if !self.channel_active(DMAChannelIndex::DMA0)
            && self.channel(DMAChannelIndex::DMA0).control.start_timing() == DMAStartTiming::HBlank
        {
            hw_events.push_dma_event(DMAChannelIndex::DMA0);
        }

        if !self.channel_active(DMAChannelIndex::DMA1)
            && self.channel(DMAChannelIndex::DMA1).control.start_timing() == DMAStartTiming::HBlank
        {
            hw_events.push_dma_event(DMAChannelIndex::DMA1);
        }

        if !self.channel_active(DMAChannelIndex::DMA2)
            && self.channel(DMAChannelIndex::DMA2).control.start_timing() == DMAStartTiming::HBlank
        {
            hw_events.push_dma_event(DMAChannelIndex::DMA2);
        }

        if !self.channel_active(DMAChannelIndex::DMA3)
            && self.channel(DMAChannelIndex::DMA3).control.start_timing() == DMAStartTiming::HBlank
        {
            hw_events.push_dma_event(DMAChannelIndex::DMA3);
        }
    }

    pub fn start_vblank(&mut self, hw_events: &mut HardwareEventQueue) {
        if !self.channel_active(DMAChannelIndex::DMA0)
            && self.channel(DMAChannelIndex::DMA0).control.start_timing() == DMAStartTiming::VBlank
        {
            hw_events.push_dma_event(DMAChannelIndex::DMA0);
        }

        if !self.channel_active(DMAChannelIndex::DMA1)
            && self.channel(DMAChannelIndex::DMA1).control.start_timing() == DMAStartTiming::VBlank
        {
            hw_events.push_dma_event(DMAChannelIndex::DMA1);
        }

        if !self.channel_active(DMAChannelIndex::DMA2)
            && self.channel(DMAChannelIndex::DMA2).control.start_timing() == DMAStartTiming::VBlank
        {
            hw_events.push_dma_event(DMAChannelIndex::DMA2);
        }

        if !self.channel_active(DMAChannelIndex::DMA3)
            && self.channel(DMAChannelIndex::DMA3).control.start_timing() == DMAStartTiming::VBlank
        {
            hw_events.push_dma_event(DMAChannelIndex::DMA3);
        }
    }

    #[inline]
    pub fn active(&self) -> bool {
        self.active_channels != 0
    }

    fn channel_active(&self, channel_index: DMAChannelIndex) -> bool {
        (self.active_channels & (1 << u8::from(channel_index))) != 0
    }

    pub fn begin_transfer(&mut self, channel_index: DMAChannelIndex, cpu: &mut ArmCpu) {
        self.active_channels |= 1 << u8::from(channel_index);
        self.channel_mut(channel_index).first_transfer = true;
        cpu.override_execution(Self::cpu_step_override);
    }

    /// This will resume a DMA transfer if one was interrupted temporarily by an IRQ.
    pub fn resume_transfer(&mut self, cpu: &mut ArmCpu) {
        if self.active_channels != 0 {
            cpu.override_execution(Self::cpu_step_override);
        }
    }

    pub fn end_transfer(
        &mut self,
        channel_index: DMAChannelIndex,
        hw_events: &mut HardwareEventQueue,
        cpu: &mut ArmCpu,
    ) {
        let remain_enabled = self.channel(channel_index).control.repeat()
            && (self.channel(channel_index).control.start_timing() != DMAStartTiming::Immediate);
        self.channel_mut(channel_index)
            .control
            .set_enabled(remain_enabled);
        if self.channel(channel_index).control.dst_control() == DMAAddressControl::IncReload {
            self.channel_mut(channel_index).reload(false);
        }
        self.channel_mut(channel_index).first_transfer = true;
        if self.channel(channel_index).control.irq() {
            hw_events.push_irq_event(crate::irq::Interrupt::dma(channel_index));
        }

        self.active_channels &= !(1 << u8::from(channel_index));

        if self.active_channels == 0 {
            cpu.resume_execution();
        }
    }

    fn transfer(hw: &mut GbaHardware, channel_index: DMAChannelIndex, cpu: &mut ArmCpu) -> u32 {
        let mut cycles = 0;

        let transfer_size;
        if hw.dma.channel(channel_index).valid_destination {
            let seq = !hw.dma.channel(channel_index).first_transfer;
            if hw.dma.channel(channel_index).control.transfer_type() == DMATransferType::Halfword {
                if hw.dma.channel(channel_index).valid_source {
                    let source_address = hw.dma.channel(channel_index).source;
                    hw.dma.dma_bus = hw.read_data_halfword(source_address, seq, &mut cycles) as u32;
                }
                let destination_address = hw.dma.channel(channel_index).destination;
                hw.write_data_halfword(
                    destination_address,
                    hw.dma.dma_bus as u16,
                    seq,
                    &mut cycles,
                );
                transfer_size = 2;
            } else {
                if hw.dma.channel(channel_index).valid_source {
                    let source_address = hw.dma.channel(channel_index).source;
                    hw.dma.dma_bus = hw.read_data_word(source_address, seq, &mut cycles);
                }
                let destination_address = hw.dma.channel(channel_index).destination;
                hw.write_data_word(destination_address, hw.dma.dma_bus, seq, &mut cycles);
                transfer_size = 4;
            }
        } else {
            if hw.dma.channel(channel_index).control.transfer_type() == DMATransferType::Halfword {
                transfer_size = 2;
            } else {
                transfer_size = 4;
            }
            cycles = 1;
        }

        hw.dma.channel_mut(channel_index).first_transfer = false;
        hw.dma.channel_mut(channel_index).count -= 1;

        match hw.dma.channel(channel_index).control.dst_control() {
            DMAAddressControl::Increment => {
                hw.dma.channel_mut(channel_index).destination += transfer_size;
                hw.dma.channel_mut(channel_index).validate_destination();
            }
            DMAAddressControl::Decrement => {
                hw.dma.channel_mut(channel_index).destination -= transfer_size;
                hw.dma.channel_mut(channel_index).validate_destination();
            }
            DMAAddressControl::Fixed => { /* NOP */ }
            DMAAddressControl::IncReload => {
                hw.dma.channel_mut(channel_index).destination += transfer_size;
                hw.dma.channel_mut(channel_index).validate_destination();
            }
        }

        match hw.dma.channel(channel_index).control.src_control() {
            DMAAddressControl::Increment => {
                hw.dma.channel_mut(channel_index).source += transfer_size;
                hw.dma.channel_mut(channel_index).validate_source();
            }
            DMAAddressControl::Decrement => {
                hw.dma.channel_mut(channel_index).source -= transfer_size;
                hw.dma.channel_mut(channel_index).validate_source();
            }
            DMAAddressControl::Fixed => { /* NOP */ }
            DMAAddressControl::IncReload => { /* NOP */ }
        }

        if hw.dma.channel(channel_index).count == 0 {
            hw.dma.end_transfer(channel_index, &mut hw.events, cpu);
        }

        return cycles;
    }

    pub fn cpu_step_override(cpu: &mut ArmCpu, memory: &mut dyn ArmMemory, _op: u32) -> u32 {
        let hw: &mut GbaHardware = match memory.as_mut_any().downcast_mut::<GbaHardware>() {
            Some(h) => h,
            _ => panic!("called DMA step with invalid memory implementation"),
        };

        if hw.dma.channel_active(DMAChannelIndex::DMA0) {
            Self::transfer(hw, DMAChannelIndex::DMA0, cpu)
        } else if hw.dma.channel_active(DMAChannelIndex::DMA1) {
            Self::transfer(hw, DMAChannelIndex::DMA1, cpu)
        } else if hw.dma.channel_active(DMAChannelIndex::DMA2) {
            Self::transfer(hw, DMAChannelIndex::DMA2, cpu)
        } else if hw.dma.channel_active(DMAChannelIndex::DMA3) {
            Self::transfer(hw, DMAChannelIndex::DMA3, cpu)
        } else {
            cpu.resume_execution();
            0
        }
    }

    #[inline(always)]
    pub fn channel(&self, channel_index: DMAChannelIndex) -> &DMAChannel {
        match channel_index {
            DMAChannelIndex::DMA0 => &self.channels[0],
            DMAChannelIndex::DMA1 => &self.channels[1],
            DMAChannelIndex::DMA2 => &self.channels[2],
            DMAChannelIndex::DMA3 => &self.channels[3],
        }
    }

    #[inline(always)]
    pub fn channel_mut(&mut self, channel_index: DMAChannelIndex) -> &mut DMAChannel {
        match channel_index {
            DMAChannelIndex::DMA0 => &mut self.channels[0],
            DMAChannelIndex::DMA1 => &mut self.channels[1],
            DMAChannelIndex::DMA2 => &mut self.channels[2],
            DMAChannelIndex::DMA3 => &mut self.channels[3],
        }
    }
}

pub struct DMAChannel {
    index: DMAChannelIndex,
    source: u32,
    original_source: u32,
    valid_source: bool,
    destination: u32,
    original_destination: u32,
    valid_destination: bool,
    count: u32,
    original_count: u16,
    control: DMAControl,
    first_transfer: bool,
}

impl DMAChannel {
    pub fn new(index: DMAChannelIndex) -> DMAChannel {
        DMAChannel {
            index: index,
            source: 0,
            original_source: 0,
            valid_source: false,
            destination: 0,
            original_destination: 0,
            valid_destination: false,
            count: 0,
            original_count: 0,
            control: DMAControl::default(),
            first_transfer: false,
        }
    }

    pub fn set_source(&mut self, new_source: u32) {
        self.original_source = new_source;
    }

    pub fn set_destination(&mut self, new_destination: u32) {
        self.original_destination = new_destination;
    }

    pub fn validate_destination(&mut self) {
        self.valid_destination = if self.index == DMAChannelIndex::DMA3 {
            // Only DMA3 is allowed to access GamePak ROM/Flash ROM.
            // SRAM is not allowed.
            self.destination <= 0x0FFFFFFF
                && (self.destination < 0x0E000000 || self.destination > 0x0E00FFFF)
        } else {
            self.destination <= 0x07FFFFFF
        };
    }

    pub fn validate_source(&mut self) {
        self.valid_source = if self.index == DMAChannelIndex::DMA3 {
            // Only DMA3 is allowed to access GamePak ROM/Flash ROM.
            // SRAM is not allowed.
            self.source <= 0x0FFFFFFF && (self.source < 0x0E000000 || self.source > 0x0E00FFFF)
        } else {
            self.source <= 0x07FFFFFF
        };
    }

    pub fn set_source_lo(&mut self, new_source_lo: u16) {
        let new_source = (self.original_source & 0xFFFF0000) | new_source_lo as u32;
        self.set_source(new_source);
    }

    pub fn set_source_hi(&mut self, new_source_hi: u16) {
        let new_source = (self.original_source & 0x0000FFFF) | ((new_source_hi as u32) << 16);
        self.set_source(new_source);
    }

    pub fn set_destination_lo(&mut self, new_destination_lo: u16) {
        let new_destination = (self.original_destination & 0xFFFF0000) | new_destination_lo as u32;
        self.set_destination(new_destination);
    }

    pub fn set_destination_hi(&mut self, new_destination_hi: u16) {
        let new_destination =
            (self.original_destination & 0x0000FFFF) | ((new_destination_hi as u32) << 16);
        self.set_destination(new_destination);
    }

    pub fn set_count(&mut self, new_count: u16) {
        self.original_count = new_count;
    }

    pub fn control(&self) -> u16 {
        self.control.value
    }

    pub fn set_control(&mut self, new_control: u16, hw_events: &mut HardwareEventQueue) {
        let old_enabled = self.control.enabled();
        self.control.value = new_control;

        if self.control.src_control() == DMAAddressControl::IncReload {
            log::debug!(
                "prohibited value for DMA{} source address control",
                u8::from(self.index)
            );
        }

        if self.control.enabled() && old_enabled != self.control.enabled() {
            self.reload(true);
            if self.control.start_timing() == DMAStartTiming::Immediate {
                hw_events.push_dma_event(self.index);
            }

            if !self.valid_source {
                log::debug!(
                    "invalid source address 0x{:08X} used for DMA{}",
                    self.source,
                    u8::from(self.index)
                );
            }

            if !self.valid_destination {
                log::debug!(
                    "invalid destination address 0x{:08X} used for DMA{}",
                    self.destination,
                    u8::from(self.index)
                );
            }
        }
    }

    pub fn reload(&mut self, reload_source: bool) {
        if reload_source {
            self.source = self.original_source;
            self.validate_source();
        }
        self.destination = self.original_destination;
        self.validate_destination();
        self.count = if self.original_count == 0 {
            if self.index == DMAChannelIndex::DMA3 {
                0x10000
            } else {
                0x4000
            }
        } else {
            self.original_count as u32
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DMAChannelIndex {
    DMA0 = 0,
    DMA1 = 1,
    DMA2 = 2,
    DMA3 = 3,
}

impl From<DMAChannelIndex> for u8 {
    fn from(channel_index: DMAChannelIndex) -> u8 {
        match channel_index {
            DMAChannelIndex::DMA0 => 0,
            DMAChannelIndex::DMA1 => 1,
            DMAChannelIndex::DMA2 => 2,
            DMAChannelIndex::DMA3 => 3,
        }
    }
}

impl From<DMAChannelIndex> for usize {
    fn from(channel_index: DMAChannelIndex) -> usize {
        match channel_index {
            DMAChannelIndex::DMA0 => 0,
            DMAChannelIndex::DMA1 => 1,
            DMAChannelIndex::DMA2 => 2,
            DMAChannelIndex::DMA3 => 3,
        }
    }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DMATransferType {
    Halfword = 0,
    Word = 1,
}

impl_enum_bitfield_conv!(DMATransferType: u16, Halfword = 0, Word = 1,);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
