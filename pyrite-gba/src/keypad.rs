pub struct GbaKeypad {
    pub input:      u16,
    pub control:    u16,
}

impl GbaKeypad {
    pub fn new() -> GbaKeypad {
        GbaKeypad {
            input:      0x03FF,
            control:    0x0000,
        }
    }

    #[inline]
    pub fn is_pressed(&self, input: KeypadInput) -> bool {
        self.input & (input.mask()) == 0
    }

    #[inline]
    pub fn set_pressed(&mut self, input: KeypadInput, pressed: bool) {
        if pressed {
            self.input &= !input.mask();
        } else {
            self.input |= input.mask();
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(u16)]
pub enum KeypadInput {
    ButtonA = 0x0,
    ButtonB = 0x1,
    Select  = 0x2,
    Start   = 0x3,
    Right   = 0x4,
    Left    = 0x5,
    Up      = 0x6,
    Down    = 0x7,
    ButtonR = 0x8,
    ButtonL = 0x9,
}

impl KeypadInput {
    #[inline]
    pub fn mask(self) -> u16 {
        1 << (self as u16)
    }
}

