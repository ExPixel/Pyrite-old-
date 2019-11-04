use super::memory::GbaMemory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum GbaInterrupt {
    VBlank = 0,
    HBlank = 1,
    VCounterMatch = 2,
    Timer0Overflow = 3,
    Timer1Overflow = 4,
    Timer2Overflow = 5,
    Timer3Overflow = 6,
    SerialComm = 7,
    DMA0 = 8,
    DMA1 = 9,
    DMA2 = 10,
    DMA3 = 11,
    Keypad = 12,
    GamePak = 13,
}

impl GbaInterrupt {
    #[inline(always)]
    pub fn mask(self) -> u16 {
        (1 << (self as u16))
    }

    #[inline(always)]
    pub fn for_dma(channel: u16) -> GbaInterrupt {
        match channel {
            0 => GbaInterrupt::DMA0,
            1 => GbaInterrupt::DMA1,
            2 => GbaInterrupt::DMA2,
            3 => GbaInterrupt::DMA3,
            bad_channel => unreachable!("DMA channel {} out of range for interrupt request", bad_channel),
        }
    }

    #[inline(always)]
    pub fn for_timer(timer: u16) -> GbaInterrupt {
        match timer {
            0 => GbaInterrupt::Timer0Overflow,
            1 => GbaInterrupt::Timer1Overflow,
            2 => GbaInterrupt::Timer2Overflow,
            3 => GbaInterrupt::Timer3Overflow,
            bad_timer => unreachable!("Timer {} out of range for interrupt request", bad_timer),
        }
    }
}

/// Set the corresponding bit for an interrupt in REG_IF if it is enabled in REG_IE
pub fn request_interrupt(memory: &mut GbaMemory, interrupt: GbaInterrupt) {
    // if IME master enable is on and the corresponding interrupt is enabled in IE
    if (memory.ioregs.ime.inner & 1) != 0 && (memory.ioregs.interrupt_enable.inner & interrupt.mask()) != 0 {
        memory.ioregs.interrupt_request.inner |= interrupt.mask();
    }
}
