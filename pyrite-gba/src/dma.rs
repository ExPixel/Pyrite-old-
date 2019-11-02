use pyrite_arm::cpu::ArmMemory;
use super::memory::GbaMemory;

pub const DMA_TIMING_IMMEDIATE: u16 = 0;
pub const DMA_TIMING_VBLANK: u16    = 1;
pub const DMA_TIMING_HBLANK: u16    = 2;
pub const DMA_TIMING_SPECIAL: u16   = 3;

#[inline(always)]
pub fn is_any_dma_active(memory: &GbaMemory) -> bool {
    memory.ioregs.internal_dma_registers[0].active |
    memory.ioregs.internal_dma_registers[1].active |
    memory.ioregs.internal_dma_registers[2].active |
    memory.ioregs.internal_dma_registers[3].active
}

pub fn step_active_channels(memory: &mut GbaMemory) -> u32 {
    for channel in 0usize..4 {
        if memory.ioregs.internal_dma_registers[channel].active {
            return dma_transfer(channel, memory);
        }
    }
    unreachable!("no DMA channels were actually active");
}

#[inline(always)]
fn dma_transfer(channel: usize, memory: &mut GbaMemory) -> u32 {
    let first_transfer = memory.ioregs.internal_dma_registers[channel].is_first_transfer();
    let source = memory.ioregs.internal_dma_registers[channel].source;
    let destination = memory.ioregs.internal_dma_registers[channel].destination;
    let word_transfer = memory.ioregs.dma_cnt_h[channel].transfer_word();

    let source_addr_control = DMAAddressControl::new(memory.ioregs.dma_cnt_h[channel].src_addr_control());
    let destination_addr_control = DMAAddressControl::new(memory.ioregs.dma_cnt_h[channel].dst_addr_control());

    if word_transfer {
        let data = memory.load32(source);
        memory.store32(destination, data);
        memory.ioregs.internal_dma_registers[channel].source = source_addr_control.apply(source, 4);
        memory.ioregs.internal_dma_registers[channel].destination = destination_addr_control.apply(destination, 4);
    } else {
        let data = memory.load16(source);
        memory.store16(destination, data);
        memory.ioregs.internal_dma_registers[channel].source = source_addr_control.apply(source, 2);
        memory.ioregs.internal_dma_registers[channel].destination = destination_addr_control.apply(destination, 2);
    }

    memory.ioregs.internal_dma_registers[channel].count -= 1;
    if memory.ioregs.internal_dma_registers[channel].count == 0 {
        memory.ioregs.internal_dma_registers[channel].active = false;
        if memory.ioregs.dma_cnt_h[channel].repeat() {
            memory.ioregs.internal_dma_registers[channel].load_count(channel, memory.ioregs.dma_cnt_l[channel].inner);
            if destination_addr_control.reload() {
                memory.ioregs.internal_dma_registers[channel].destination = memory.ioregs.dma_dad[channel].inner;
            }
        } else {
            memory.ioregs.dma_cnt_h[channel].set_enabled(false);
        }
    }

    return if !first_transfer {
        if word_transfer {
            memory.data_access_seq32(source) + memory.data_access_seq32(destination)
        } else {
            memory.data_access_seq32(source) + memory.data_access_seq32(destination)
        }
    } else {
        let internal_cycles = if (source >= 0x08000000) && (destination >= 0x08000000) { 4 } else { 2 };
        if word_transfer {
            memory.data_access_nonseq32(source) + memory.data_access_nonseq32(destination) + internal_cycles
        } else {
            memory.data_access_nonseq16(source) + memory.data_access_nonseq16(destination) + internal_cycles
        }
    };
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DMAAddressControl {
    Increment = 0,
    Decrement = 1,
    Fixed = 2,
    IncrementAndReload = 3,
}

impl DMAAddressControl {
    pub fn new(value: u16) -> DMAAddressControl {
        match value {
            0 => DMAAddressControl::Increment,
            1 => DMAAddressControl::Decrement,
            2 => DMAAddressControl::Fixed,
            3 => DMAAddressControl::IncrementAndReload,
            _ => unreachable!("bad DMA address control"),
        }
    }

    pub fn reload(self) -> bool {
        self == DMAAddressControl::IncrementAndReload
    }

    pub fn apply(self, addr: u32, unit_size: u32) -> u32 {
        match self {
            DMAAddressControl::Increment    => addr + unit_size,
            DMAAddressControl::Decrement    => addr - unit_size,
            DMAAddressControl::Fixed        => addr,
            DMAAddressControl::IncrementAndReload => addr + unit_size,
        }
    }
}
