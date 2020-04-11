use super::dma::DMAChannelIndex;
use super::irq::Interrupt;

use std::cell::UnsafeCell;
use std::rc::Rc;

pub const MAX_GBA_EVENTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GbaEvent {
    None,
    Halt,
    Stop,
    IRQ(Interrupt),
    DMA(DMAChannelIndex),
    HBlank,
    HDraw,
    TimerOverflows,
}

impl Default for GbaEvent {
    fn default() -> GbaEvent {
        GbaEvent::None
    }
}

#[derive(Copy, Debug, Default, Clone)]
struct GbaEventNode {
    cycles: u32,
    event: GbaEvent,
}

/// Used for scheduling tasks in the GBA after some number of cycles have passed.
pub struct GbaScheduler {
    /// The number of events that are currently queued.
    event_count: usize,

    events: [GbaEventNode; MAX_GBA_EVENTS],

    /// Set after `step` when the number of cycles that have passed is greater than
    /// `cycles_until_next_event`.
    late: u32,
}

impl GbaScheduler {
    pub fn new() -> GbaScheduler {
        let mut sched = GbaScheduler {
            event_count: 0,
            late: 0,
            events: crate::util::array::new_array::<_, _>(GbaEventNode::default()),
        };

        sched.events[0].cycles = std::u32::MAX;
        sched
    }

    pub fn clear(&mut self) {
        self.events
            .iter_mut()
            .for_each(|e| e.event = GbaEvent::None);
        self.event_count = 0;
    }

    #[inline]
    pub fn purge(&mut self, event: GbaEvent) {
        let mut idx = 0;
        while idx < self.event_count {
            if self.events[idx].event == event {
                if idx != MAX_GBA_EVENTS {
                    self.events[idx + 1].cycles += self.events[idx].cycles;
                    self.events
                        .copy_within((idx + 1)..(idx + 1 + self.event_count), idx);
                } else {
                    self.events[idx].event = GbaEvent::None;
                }
                self.event_count -= 1;
            } else {
                idx += 1;
            }
        }
    }

    #[inline(always)]
    pub fn step(&mut self, cycles: u32) -> bool {
        if cycles >= self.events[0].cycles {
            self.late = cycles - self.events[0].cycles;
            self.events[0].cycles = 0;
            true
        } else {
            self.events[0].cycles -= cycles;
            false
        }
    }

    pub fn contains(&self, event: GbaEvent) -> bool {
        self.events.iter().any(|node| node.event == event)
    }

    /// This will pop the last fired event from the event list (0 or less cycles remaining). The
    /// value returned is a type with the event, the number of cycles it was late by, and a boolean
    /// that is true if there is another event to be processed. This should only be called if
    /// `step` returned true or if the last call to `pop_event` returned true in the last boolean
    /// in the tuple.
    pub fn pop_event(
        &mut self,
    ) -> (
        /* event */ GbaEvent,
        /* late */ u32,
        /* has next */ bool,
    ) {
        let ret_event = self.events[0].event;
        let ret_late = self.late;

        self.event_count -= 1;
        if self.event_count > 0 {
            self.events.copy_within(1..(1 + self.event_count), 0);
            if self.late >= self.events[0].cycles {
                self.late -= self.events[0].cycles;
                self.events[0].cycles = 0;
                (ret_event, ret_late, true)
            } else {
                self.events[0].cycles -= self.late;
                self.late = 0;
                (ret_event, ret_late, false)
            }
        } else {
            self.events[0].cycles = std::u32::MAX;
            self.events[0].event = GbaEvent::None;
            (ret_event, ret_late, false)
        }
    }

    /// Will add a new event to the scheduler. If an event is scheduled during event processing
    /// and it is zero cycles or scheduled in the past (because the previous event was late) it
    /// will be fired in the same event processing loop.
    pub fn schedule(&mut self, event: GbaEvent, cycles: u32) {
        assert!(self.event_count < MAX_GBA_EVENTS);

        let mut cycles_acc = 0;
        let mut idx = 0;

        while idx < self.event_count {
            let cycles_acc_after = cycles_acc + self.events[idx].cycles;

            // Too many cycles would have passed after the event so we insert
            // the new one before it and remove the new event's cycles from the
            // old event at the position.
            if cycles_acc_after > cycles {
                self.events[idx].cycles -= cycles;
                self.events
                    .copy_within(idx..(idx + self.event_count), idx + 1);
                break;
            }

            cycles_acc = cycles_acc_after;
            idx += 1;
        }

        self.events[idx] = GbaEventNode {
            cycles: cycles - cycles_acc,
            event: event,
        };
        self.event_count += 1;
    }
}

pub struct SharedGbaScheduler(Rc<UnsafeCell<GbaScheduler>>);

impl SharedGbaScheduler {
    pub fn new() -> SharedGbaScheduler {
        SharedGbaScheduler(Rc::new(UnsafeCell::new(GbaScheduler::new())))
    }

    #[inline]
    pub fn step(&self, cycles: u32) -> bool {
        unsafe { (*self.0.get()).step(cycles) }
    }

    #[inline]
    pub fn pop_event(
        &self,
    ) -> (
        /* event */ GbaEvent,
        /* late */ u32,
        /* has next */ bool,
    ) {
        unsafe { (*self.0.get()).pop_event() }
    }

    #[inline]
    pub fn schedule(&self, event: GbaEvent, cycles: u32) {
        unsafe { (*self.0.get()).schedule(event, cycles) };
    }

    #[inline]
    pub fn purge(&self, event: GbaEvent) {
        unsafe { (*self.0.get()).purge(event) };
    }

    #[inline]
    pub fn clear(&self) {
        unsafe { (*self.0.get()).clear() };
    }
}

impl Clone for SharedGbaScheduler {
    fn clone(&self) -> SharedGbaScheduler {
        SharedGbaScheduler(Rc::clone(&self.0))
    }
}
