pub struct GbaTimers {
    timers: [GbaTimer; 4],
    active_timers: u8,
}

impl GbaTimers {
    pub fn new() -> GbaTimers {
        GbaTimers {
            timers: [
                GbaTimer::new(TimerIndex::TM0),
                GbaTimer::new(TimerIndex::TM1),
                GbaTimer::new(TimerIndex::TM2),
                GbaTimer::new(TimerIndex::TM3),
            ],
            active_timers: 0,
        }
    }

    #[inline]
    pub fn active(&self) -> bool {
        self.active_timers != 0
    }

    fn timer_active(&self, timer_index: TimerIndex) -> bool {
        (self.active_timers & (1 << u8::from(timer_index))) != 0
    }

    pub fn step(&mut self, mut cycles: u32) {
        while cycles > 1024 {
            self.safe_step(1024);
            cycles -= 1024;
        }
        self.safe_step(cycles);
    }

    fn safe_step(&mut self, cycles: u32) {
        if self.timers[0].control.enabled() && !self.timers[0].control.count_up_timing() {
            self.safe_step_single_timer(0, cycles);
        }

        if self.timers[1].control.enabled() && !self.timers[1].control.count_up_timing() {
            self.safe_step_single_timer(1, cycles);
        }

        if self.timers[2].control.enabled() && !self.timers[2].control.count_up_timing() {
            self.safe_step_single_timer(2, cycles);
        }

        if self.timers[3].control.enabled() && !self.timers[3].control.count_up_timing() {
            self.safe_step_single_timer(3, cycles);
        }
    }

    fn safe_step_single_timer(&mut self, mut timer: usize, mut cycles: u32) {
        todo!();
    }
}

struct GbaTimer {
    index: TimerIndex,

    /// This actually contains an unsigned fixed point value with a fractional part that
    /// is the size of the value of `prescaler` which will be one of { 0, 6, 8, 10 }. The integer
    /// part of this number will always be 16 bits wide and all bits beyond the integer part of the
    /// counter will be set to 1. This way the counter can just be incremented by one for each
    /// cycle and overflows will happen on time.
    counter: u32,

    control: TimerControl,
}

impl GbaTimer {
    pub fn new(index: TimerIndex) -> GbaTimer {
        GbaTimer {
            index: index,
            counter: 0xFFFF0000,
            control: TimerControl::default(),
        }
    }

    pub fn set_counter(&mut self, new_counter: u16) {
        self.counter = (self.counter & (0xFFFF << self.prescaler()))
            | ((new_counter as u32) << self.prescaler());
    }

    pub fn counter(&mut self) -> u16 {
        (self.counter >> self.prescaler()) as u16
    }

    fn prescaler(&self) -> u32 {
        match self.control.prescaler_selection() {
            0 => 0,
            1 => 6,
            2 => 8,
            3 => 10,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// Increments the timer and returns true if an overflow occurred.
    /// No more than 1024 cycles should be passed in at one time.
    pub fn increment(&mut self, mut cycles: u32) -> bool {
        debug_assert!(cycles <= 1024);

        let mut overflow_occurred = false;
        loop {
            let (value, overflow) = self.counter.overflowing_add(cycles);
            if !overflow {
                self.counter = value;
                return overflow_occurred;
            }
            overflow_occurred = true;

            // At this point `value` contains the number of cycles that the counter overflowed by.
            // So we set cycles, reset the counter, and continue until no more overflows occur.
            cycles = value;
            self.set_counter(0);
        }
    }
}

bitfields! (TimerControl: u16 { 
    prescaler_selection, set_prescaler_selection: u16 = [0, 1],
    count_up_timing, set_count_up_timing: bool = [2, 2],
    irq, set_irq: bool = [6, 6],
    enabled, set_enabled: bool = [7, 7],
});

pub enum TimerIndex {
    TM0 = 0,
    TM1 = 1,
    TM2 = 2,
    TM3 = 3,
}

impl From<TimerIndex> for u8 {
    #[inline(always)]
    fn from(timer_index: TimerIndex) -> u8 {
        match timer_index {
            TimerIndex::TM0 => 0,
            TimerIndex::TM1 => 1,
            TimerIndex::TM2 => 2,
            TimerIndex::TM3 => 3,
        }
    }
}

impl From<TimerIndex> for usize {
    #[inline(always)]
    fn from(timer_index: TimerIndex) -> usize {
        match timer_index {
            TimerIndex::TM0 => 0,
            TimerIndex::TM1 => 1,
            TimerIndex::TM2 => 2,
            TimerIndex::TM3 => 3,
        }
    }
}
