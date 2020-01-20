// TODO turn this into an actual debugger :P
pub struct GbaDebugger {
    step_size: Option<GbaStepSize>,
    pub debugging: bool,
}

impl GbaDebugger {
    pub fn new() -> GbaDebugger {
        GbaDebugger {
            step_size: None,
            debugging: false,
        }
    }

    pub fn step(&mut self, step_size: GbaStepSize) {
        self.step_size = Some(step_size);
    }

    pub fn pop_step_size(&mut self) -> Option<GbaStepSize> {
        self.step_size.take()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GbaStepSize {
    Instruction,
    #[allow(dead_code)]
    VideoLine,
    #[allow(dead_code)]
    VideoFrame,
}
