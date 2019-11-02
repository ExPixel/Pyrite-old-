use pyrite_arm::cpu::ArmMemory;
use super::memory::GbaMemory;

pub const DMA_TIMING_IMMEDIATE: u16 = 0;
pub const DMA_TIMING_VBLANK: u16    = 1;
pub const DMA_TIMING_HBLANK: u16    = 2;
pub const DMA_TIMING_SPECIAL: u16   = 3;

pub const DMA_ADDR_CONTROL_INCREMENT: u16   = 0;
pub const DMA_ADDR_CONTROL_DECREMENT: u16   = 1;
pub const DMA_ADDR_CONTROL_FIXED: u16       = 2;
pub const DMA_ADDR_CONTROL_RELOAD: u16      = 3;

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

    let source_addr_control = memory.ioregs.dma_cnt_h[channel].src_addr_control();
    let destination_addr_control = memory.ioregs.dma_cnt_h[channel].dst_addr_control();

    if word_transfer {
        let data = memory.load32(source);
        memory.store32(destination, data);
        memory.ioregs.internal_dma_registers[channel].source = dma_next_addr(source, source_addr_control, 4);
        memory.ioregs.internal_dma_registers[channel].destination = dma_next_addr(destination, destination_addr_control, 4);
    } else {
        let data = memory.load16(source);
        memory.store16(destination, data);
        memory.ioregs.internal_dma_registers[channel].source = dma_next_addr(source, source_addr_control, 2);
        memory.ioregs.internal_dma_registers[channel].destination = dma_next_addr(destination, destination_addr_control, 2);
    }

    memory.ioregs.internal_dma_registers[channel].count -= 1;
    if memory.ioregs.internal_dma_registers[channel].count == 0 {
        memory.ioregs.internal_dma_registers[channel].active = false;
        if memory.ioregs.dma_cnt_h[channel].repeat() {
            memory.ioregs.internal_dma_registers[channel].load_count(channel, memory.ioregs.dma_cnt_l[channel].inner);
            if destination_addr_control == DMA_ADDR_CONTROL_RELOAD {
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

fn dma_next_addr(address: u32, control: u16, unit_size: u32) -> u32 {
    match control {
        DMA_ADDR_CONTROL_INCREMENT  => address + unit_size,
        DMA_ADDR_CONTROL_DECREMENT  => address - unit_size,
        DMA_ADDR_CONTROL_FIXED      => address,
        DMA_ADDR_CONTROL_RELOAD     => address + unit_size,
        bad_control                 => unreachable!("BAD ADDR CONTROL: {}", bad_control),
    }
}
