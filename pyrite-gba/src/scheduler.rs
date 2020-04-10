use super::dma::DMAChannelIndex;
use super::irq::Interrupt;

use std::cell::RefCell;
use std::rc::Rc;

pub const MAX_GBA_EVENTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GbaEvent {
    None,
    FireIRQ(Interrupt),
    FireDMA(DMAChannelIndex),
    HBlank,
    HDraw,
}

impl Default for GbaEvent {
    fn default() -> GbaEvent {
        GbaEvent::None
    }
}

#[derive(Default, Clone)]
struct GbaEventNode {
    cycles: u32,
    event: GbaEvent,
    next: u16,
}

/// Used for scheduling tasks in the GBA after some number of cycles have passed.
pub struct GbaScheduler {
    /// The cycles until the next event. `flush_cycles` must be called to update this value and the
    /// next event node before reading self.events[self.next_event].cycles.
    cycles_until_next_event: u32,

    /// The index of the next event to be fired.
    next_event: usize,

    /// The number of events that are currently queued.
    event_count: usize,

    /// Linked list of event nodes. GbaEvent::None means that an event slot is unoccupied.
    events: [GbaEventNode; MAX_GBA_EVENTS],

    /// Set after `step` when the number of cycles that have passed is greater than
    /// `cycles_until_next_event`.
    late: u32,
}

impl GbaScheduler {
    pub fn new() -> GbaScheduler {
        GbaScheduler {
            cycles_until_next_event: std::u32::MAX,
            next_event: 0,
            event_count: 0,
            late: 0,
            events: crate::util::array::new_array::<_, _>(GbaEventNode::default()),
        }
    }

    #[inline]
    pub fn step(&mut self, cycles: u32) -> bool {
        if cycles >= self.cycles_until_next_event {
            self.late = cycles - self.cycles_until_next_event;
            self.cycles_until_next_event = std::u32::MAX;
            true
        } else {
            self.cycles_until_next_event -= cycles;
            false
        }
    }

    /// This will pop the last fired event from the event list (0 or less cycles remaining). The
    /// value returned is a type with the event, the number of cycles it was late by, and a boolean
    /// that is true if there is another event to be processed. This should only be called if
    /// `step` returned true or if the last call to `pop_event` returned true in the last boolean
    /// in the tuple.
    pub fn pop_event(
        &mut self,
    ) -> Option<(
        /* event */ GbaEvent,
        /* late */ u32,
        /* has next */ bool,
    )> {
        let ret_event = self.events[self.next_event].event;
        let ret_late = self.late;

        self.events[self.next_event].event = GbaEvent::None;
        self.next_event = self.events[self.next_event].next as usize;

        if self.late >= self.events[self.next_event].cycles {
            self.late -= self.events[self.next_event].cycles;
            self.events[self.next_event].cycles = 0;
            Some((ret_event, ret_late, true))
        } else {
            self.events[self.next_event].cycles -= self.late;
            self.late = 0;
            Some((ret_event, ret_late, false))
        }
    }

    fn flush_cycles(&mut self) {
        self.events[self.next_event].cycles = self.cycles_until_next_event;
        self.cycles_until_next_event = 0;
    }

    /// Will add a new event to the scheduler. If an event is scheduled during event processing
    /// and it is zero cycles or scheduled in the past (because the previous event was late) it
    /// will be fired in the same event processing loop.
    pub fn schedule(&mut self, event: GbaEvent, cycles: u32) {
        if self.events[self.next_event].event == GbaEvent::None {
            self.next_event = 0;
            self.events[0] = GbaEventNode {
                event: event,
                cycles: cycles,
                next: 0, // just points back at itself
            };
            self.cycles_until_next_event = cycles;
            return;
        }

        assert!(self.event_count < MAX_GBA_EVENTS, "too many GBA events");
        let new_index = self
            .find_empty_slot()
            .expect("failed to find empty event slot");

        self.flush_cycles();
        if cycles < self.events[self.next_event].cycles {
            self.events[self.next_event].cycles -= cycles;
            self.cycles_until_next_event = self.events[self.next_event].cycles;
            self.events[new_index] = GbaEventNode {
                event: event,
                cycles: cycles,
                next: self.next_event as u16,
            };
            self.next_event = new_index;
            return;
        }

        self.push_event(new_index, event, cycles);
    }

    /// Called while scheduling to push an event some time after the next event.
    fn push_event(&mut self, new_index: usize, event: GbaEvent, cycles: u32) {
        let mut prev_index = self.next_event;
        let mut acc_cycles = self.events[self.next_event].cycles;

        loop {
            let next_index = self.events[prev_index].next as usize;

            // We're reached the end of the event chain if we reach a loop, or if the event type is
            // None, so we just append the event and exit.
            if next_index == prev_index || self.events[next_index].event == GbaEvent::None {
                self.events[prev_index].next = new_index as u16;
                self.events[new_index] = GbaEventNode {
                    event: event,
                    cycles: cycles - acc_cycles,
                    next: new_index as u16,
                };
                return;
            }

            let next_acc_cycles = acc_cycles + self.events[next_index].cycles;

            // After the next event, too many cycles would have passed, so we insert the new event
            // before it.
            if next_acc_cycles > cycles {
                self.events[prev_index].next = new_index as u16;
                self.events[new_index] = GbaEventNode {
                    event: event,
                    cycles: cycles - acc_cycles,
                    next: next_index as u16,
                };
                return;
            }

            // Not enough cycles have passed to insert the new event, so we move on:
            acc_cycles = next_acc_cycles;
            prev_index = next_index;
        }
    }

    /// Returns an empty slot for an event if one is found.
    fn find_empty_slot(&mut self) -> Option<usize> {
        for (idx, node) in self.events.iter().enumerate() {
            if node.event == GbaEvent::None {
                return Some(idx);
            }
        }
        None
    }
}

pub struct SharedGbaScheduler(Rc<RefCell<GbaScheduler>>);

impl SharedGbaScheduler {
    pub fn new() -> SharedGbaScheduler {
        SharedGbaScheduler(Rc::new(RefCell::new(GbaScheduler::new())))
    }

    #[inline]
    pub fn step(&self, cycles: u32) -> bool {
        self.0.borrow_mut().step(cycles)
    }

    pub fn pop_event(
        &self,
    ) -> Option<(
        /* event */ GbaEvent,
        /* late */ u32,
        /* has next */ bool,
    )> {
        self.0.borrow_mut().pop_event()
    }

    pub fn schedule(&self, event: GbaEvent, cycles: u32) {
        self.0.borrow_mut().schedule(event, cycles)
    }
}
