use crate::scheduler::{GbaEvent, SharedGbaScheduler};

pub struct GbaTimers {
    timers: [GbaTimer; 4],
    active_timers: u8,
    cycles_acc: u32,
    scheduler: SharedGbaScheduler,
    last_overflow_calc: u32,
}

impl GbaTimers {
    pub fn new(scheduler: SharedGbaScheduler) -> GbaTimers {
        GbaTimers {
            timers: [
                GbaTimer::new(TimerIndex::TM0),
                GbaTimer::new(TimerIndex::TM1),
                GbaTimer::new(TimerIndex::TM2),
                GbaTimer::new(TimerIndex::TM3),
            ],
            active_timers: 0,
            cycles_acc: 0,
            scheduler: scheduler,
            last_overflow_calc: std::u32::MAX,
        }
    }

    pub fn write_timer_counter(&mut self, timer_index: TimerIndex, counter: u16) {
        self.timers[usize::from(timer_index)].reload = counter;
    }

    pub fn write_timer_control(&mut self, timer_index: TimerIndex, control: u16) {
        self.flush_acc_cycles();

        match self.timers[usize::from(timer_index)].set_control(control) {
            TimerStateChange::Active => {
                self.timers[usize::from(timer_index)].reload_counter();
                self.active_timers |= 1 << u8::from(timer_index);
            }

            TimerStateChange::Inactive => {
                if self.timers[usize::from(timer_index)].passive() {
                    // If the timer is a passive timer at this point that means it is a count-up
                    // timer that was enabled:
                    self.timers[usize::from(timer_index)].reload_counter();
                }

                self.active_timers &= !(1 << u8::from(timer_index));
            }

            TimerStateChange::None => { /* NOP */ }
        }

        if self.active_timers != 0 {
            self.calc_next_overflow();
        }
    }

    pub fn read_timer_counter(&self, timer_index: TimerIndex) -> u16 {
        if self.timers[usize::from(timer_index)].active() {
            // this is only relevant for counters that are currently active and aren't just
            // counter-up timers:
            self.timers[usize::from(timer_index)].counter_with_offset(self.cycles_acc)
        } else {
            self.timers[usize::from(timer_index)].counter()
        }
    }

    pub fn read_timer_control(&self, timer_index: TimerIndex) -> u16 {
        self.timers[usize::from(timer_index)].control.value
    }

    #[inline]
    pub fn active(&self) -> bool {
        self.active_timers != 0
    }

    #[inline]
    pub(crate) fn step(&mut self, cycles: u32) {
        // I do a wrapping add because I don't really care if this overflows.
        // If the value inside of it is being dependend on then `process_overflows` will
        // be called long before the value wraps.
        self.cycles_acc = self.cycles_acc.wrapping_add(cycles);
    }

    pub(crate) fn process_overflows(&mut self) {
        self.internal_step(self.cycles_acc);
        self.cycles_acc = 0;
        self.last_overflow_calc = std::u32::MAX;
        self.calc_next_overflow();
    }

    fn flush_acc_cycles(&mut self) {
        let acc = std::mem::replace(&mut self.cycles_acc, 0);
        self.timers
            .iter_mut()
            .filter(|timer| timer.active())
            .for_each(|timer| timer.counter += acc);
    }

    fn calc_next_overflow(&mut self) {
        if self.active_timers != 0 {
            let next_overflow_at = self
                .timers
                .iter()
                .filter(|t| t.active())
                .map(GbaTimer::cycles_to_overflow)
                .min()
                .unwrap_or(std::u32::MAX);

            if next_overflow_at < self.last_overflow_calc.saturating_sub(self.cycles_acc) {
                self.scheduler.purge(GbaEvent::TimerOverflows);
                self.scheduler
                    .schedule(GbaEvent::TimerOverflows, next_overflow_at);
                self.last_overflow_calc = next_overflow_at;
            }
        } else {
            self.scheduler.purge(GbaEvent::TimerOverflows);
        }
    }

    fn internal_step(&mut self, mut cycles: u32) {
        if self.active_timers == 0 {
            return;
        }

        if cycles % 1024 != 0 {
            self.increment_timers(cycles % 1024);
            cycles -= cycles % 1024;
        }

        while cycles >= 1024 {
            self.increment_timers(1024);
            cycles -= 1024;
        }
    }

    fn increment_timers(&mut self, cycles: u32) {
        if self.timers[0].active() {
            self.increment_timer(TimerIndex::TM0, cycles);
        }

        if self.timers[1].active() {
            self.increment_timer(TimerIndex::TM1, cycles);
        }

        if self.timers[2].active() {
            self.increment_timer(TimerIndex::TM2, cycles);
        }

        if self.timers[3].active() {
            self.increment_timer(TimerIndex::TM3, cycles);
        }
    }

    fn increment_timer(&mut self, mut timer_index: TimerIndex, mut cycles: u32) {
        loop {
            let timer = usize::from(timer_index);
            let overflows = self.timers[timer].increment(cycles);

            // If no overflows occured, we can just exit.
            if overflows == 0 {
                return;
            }

            if self.timers[timer].control.irq() {
                self.scheduler
                    .schedule(GbaEvent::IRQ(crate::irq::Interrupt::timer(timer_index)), 0);
            }

            // If there is a timer after this one and it is using count-up timing, set it as the
            // current timer_index value, use the number of overflows to increment, and then
            // continue to the loop.
            if let Some(next_timer_index) = timer_index.next() {
                let next_timer = usize::from(next_timer_index);
                if self.timers[next_timer].passive() {
                    timer_index = next_timer_index;
                    cycles = overflows;
                    continue;
                }
            }

            return;
        }
    }
}

struct GbaTimer {
    #[allow(dead_code)]
    index: TimerIndex,

    /// This actually contains an unsigned fixed point value with a fractional part that
    /// is the size of the value of `prescaler` which will be one of { 0, 6, 8, 10 }. The integer
    /// part of this number will always be 16 bits wide and all bits beyond the integer part of the
    /// counter will be set to 1. This way the counter can just be incremented by one for each
    /// cycle and overflows will happen on time.
    counter: u32,

    reload: u16,

    control: TimerControl,
}

impl GbaTimer {
    pub fn new(index: TimerIndex) -> GbaTimer {
        GbaTimer {
            index: index,
            counter: 0xFFFF0000,
            reload: 0,
            control: TimerControl::default(),
        }
    }

    pub fn active(&self) -> bool {
        self.control.enabled() && !self.control.count_up_timing()
    }

    pub fn passive(&self) -> bool {
        self.control.enabled() && self.control.count_up_timing()
    }

    pub fn set_counter(&mut self, new_counter: u16) {
        self.counter = (self.counter & !(0xFFFF << self.prescaler()))
            | ((new_counter as u32) << self.prescaler());
    }

    pub fn counter(&self) -> u16 {
        (self.counter >> self.prescaler()) as u16
    }

    fn counter_with_offset(&self, offset: u32) -> u16 {
        ((self.counter + offset) >> self.prescaler()) as u16
    }

    pub fn cycles_to_overflow(&self) -> u32 {
        ((0xFFFF - self.counter()) as u32) + 1
    }

    fn prescaler(&self) -> u32 {
        if self.control.count_up_timing() {
            return 0;
        }

        match self.control.prescaler_selection() {
            0 => 0,
            1 => 6,
            2 => 8,
            3 => 10,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    fn set_control(&mut self, control: u16) -> TimerStateChange {
        let counter = self.counter();
        let old_active = self.active();
        self.control.value = control;
        self.set_counter(counter); // We set it again because the prescaler may have changed.

        if old_active != self.active() {
            if self.active() {
                TimerStateChange::Active
            } else {
                TimerStateChange::Inactive
            }
        } else {
            TimerStateChange::None
        }
    }

    fn reload_counter(&mut self) {
        self.set_counter(self.reload);
    }

    /// Increments the timer by some number of cycles and returns
    /// the number of overflows that occurred.
    fn increment(&mut self, mut cycles: u32) -> u32 {
        let mut overflows = 0;
        'ov_loop: loop {
            match self.counter.overflowing_add(cycles) {
                (new_value, false) => {
                    self.counter = new_value;
                    break 'ov_loop;
                }

                (overflow_value, true) => {
                    cycles = cycles - (std::u32::MAX - self.counter) - 1;
                    self.reload_counter();
                    overflows += 1;
                }
            }
        }
        return overflows;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimerStateChange {
    None,
    Active,
    Inactive,
}

bitfields! (TimerControl: u16 {
    prescaler_selection, set_prescaler_selection: u16 = [0, 1],
    count_up_timing, set_count_up_timing: bool = [2, 2],
    irq, set_irq: bool = [6, 6],
    enabled, set_enabled: bool = [7, 7],
});

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimerIndex {
    TM0 = 0,
    TM1 = 1,
    TM2 = 2,
    TM3 = 3,
}

impl TimerIndex {
    #[inline]
    pub fn next(self) -> Option<TimerIndex> {
        match self {
            TimerIndex::TM0 => Some(TimerIndex::TM1),
            TimerIndex::TM1 => Some(TimerIndex::TM2),
            TimerIndex::TM2 => Some(TimerIndex::TM3),
            TimerIndex::TM3 => None,
        }
    }
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
