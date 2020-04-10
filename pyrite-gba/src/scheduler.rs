use super::dma::DMAChannelIndex;
use super::irq::Interrupt;

use std::cell::UnsafeCell;
use std::rc::Rc;

pub const MAX_GBA_EVENTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GbaEvent {
    None,
    Halt,
    Stop,
    IRQ(Interrupt),
    DMA(DMAChannelIndex),
    HBlank,
    HDraw,
}

impl Default for GbaEvent {
    fn default() -> GbaEvent {
        GbaEvent::None
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct GbaEventIndex(u16);

impl GbaEventIndex {
    const ZERO: GbaEventIndex = GbaEventIndex(0);
}

impl Default for GbaEventIndex {
    fn default() -> Self {
        Self::ZERO
    }
}

struct GbaEventsArray([GbaEventNode; MAX_GBA_EVENTS]);

impl GbaEventsArray {
    fn new() -> GbaEventsArray {
        GbaEventsArray(crate::util::array::new_array::<_, _>(
            GbaEventNode::default(),
        ))
    }

    fn iter<'r>(&'r self) -> impl 'r + Iterator<Item = &GbaEventNode> {
        self.0.iter()
    }

    fn iter_mut<'r>(&'r mut self) -> impl 'r + Iterator<Item = &mut GbaEventNode> {
        self.0.iter_mut()
    }

    /// Returns an empty slot for an event if one is found.
    fn find_empty_slot(&mut self) -> Option<GbaEventIndex> {
        for (idx, node) in self.iter().enumerate() {
            if node.event == GbaEvent::None {
                return Some(GbaEventIndex(idx as u16));
            }
        }
        None
    }
}

impl std::ops::Index<GbaEventIndex> for GbaEventsArray {
    type Output = GbaEventNode;
    fn index(&self, idx: GbaEventIndex) -> &Self::Output {
        // I only every generate valid indices.
        unsafe { self.0.get_unchecked(idx.0 as usize) }
    }
}

impl std::ops::IndexMut<GbaEventIndex> for GbaEventsArray {
    fn index_mut(&mut self, idx: GbaEventIndex) -> &mut Self::Output {
        // I only every generate valid indices.
        unsafe { self.0.get_unchecked_mut(idx.0 as usize) }
    }
}

#[derive(Default, Clone)]
struct GbaEventNode {
    cycles: u32,
    event: GbaEvent,
    next: GbaEventIndex,
}

/// Used for scheduling tasks in the GBA after some number of cycles have passed.
pub struct GbaScheduler {
    /// The cycles until the next event. `flush_cycles` must be called to update this value and the
    /// next event node before reading self.events[self.next_event].cycles.
    cycles_until_next_event: u32,

    /// The index of the next event to be fired.
    next_event: GbaEventIndex,

    /// The number of events that are currently queued.
    event_count: usize,

    /// Linked list of event nodes. GbaEvent::None means that an event slot is unoccupied.
    events: GbaEventsArray,

    /// Set after `step` when the number of cycles that have passed is greater than
    /// `cycles_until_next_event`.
    late: u32,
}

impl GbaScheduler {
    pub fn new() -> GbaScheduler {
        GbaScheduler {
            cycles_until_next_event: std::u32::MAX,
            next_event: GbaEventIndex::ZERO,
            event_count: 0,
            late: 0,
            events: GbaEventsArray::new(),
        }
    }

    pub fn clear(&mut self) {
        self.events
            .iter_mut()
            .for_each(|e| e.event = GbaEvent::None);
    }

    #[inline]
    pub fn purge(&mut self, event: GbaEvent) {
        unsafe { self.pruge_unsafe(event) }
    }

    /// Removes all instances of an event from the queue and returns the number of events that were
    /// removed.
    unsafe fn pruge_unsafe(&mut self, event: GbaEvent) {
        // We exit early if trying to purge none or if there are not valid events queued up.
        if event == GbaEvent::None || self.events[self.next_event].event == GbaEvent::None {
            return;
        }

        self.flush_cycles();

        let mut ptr = &mut self.next_event as *mut GbaEventIndex;
        loop {
            // We've entered a loop in event pointers so we're done.
            if self.events[*ptr].next == *ptr {
                return;
            }

            // We've hit and empty event, so we're done.
            if self.events[*ptr].event == GbaEvent::None {
                return;
            }

            if self.events[*ptr].event == event {
                self.events[*ptr].event = GbaEvent::None;
                let cycles = self.events[*ptr].cycles;
                ptr = &mut self.events[*ptr].next as *mut GbaEventIndex;
                self.events[*ptr].cycles += cycles;
            }
        }

        if self.events[self.next_event].event == GbaEvent::None {
            self.cycles_until_next_event = std::u32::MAX;
        } else {
            self.cycles_until_next_event = self.events[self.next_event].cycles;
        }
    }

    #[inline(always)]
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
    ) -> (
        /* event */ GbaEvent,
        /* late */ u32,
        /* has next */ bool,
    ) {
        self.event_count -= 1;

        let ret_event = self.events[self.next_event].event;
        let ret_late = self.late;

        self.events[self.next_event].event = GbaEvent::None;
        self.next_event = self.events[self.next_event].next;

        if self.late >= self.events[self.next_event].cycles {
            self.late -= self.events[self.next_event].cycles;
            self.events[self.next_event].cycles = 0;
            (ret_event, ret_late, true)
        } else {
            self.events[self.next_event].cycles -= self.late;
            self.late = 0;
            (ret_event, ret_late, false)
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
        assert!(self.event_count < MAX_GBA_EVENTS, "too many GBA events");

        self.event_count += 1;

        if self.events[self.next_event].event == GbaEvent::None {
            self.next_event = GbaEventIndex::ZERO;
            self.events[GbaEventIndex::ZERO] = GbaEventNode {
                event: event,
                cycles: cycles,
                next: GbaEventIndex::ZERO, // just points back at itself
            };
            self.cycles_until_next_event = cycles;
            return;
        }

        let new_index = self
            .events
            .find_empty_slot()
            .expect("failed to find empty event slot");

        self.flush_cycles();
        if cycles < self.events[self.next_event].cycles {
            self.events[self.next_event].cycles -= cycles;
            self.cycles_until_next_event = self.events[self.next_event].cycles;
            self.events[new_index] = GbaEventNode {
                event: event,
                cycles: cycles,
                next: self.next_event as GbaEventIndex,
            };
            self.next_event = new_index;
            return;
        }

        self.push_event(new_index, event, cycles);
    }

    /// Called while scheduling to push an event some time after the next event.
    fn push_event(&mut self, new_index: GbaEventIndex, event: GbaEvent, cycles: u32) {
        let mut prev_index = self.next_event;
        let mut acc_cycles = self.events[self.next_event].cycles;

        loop {
            let next_index = self.events[prev_index].next;

            // We're reached the end of the event chain if we reach a loop, or if the event type is
            // None, so we just append the event and exit.
            if next_index == prev_index || self.events[next_index].event == GbaEvent::None {
                self.events[prev_index].next = new_index as GbaEventIndex;
                self.events[new_index] = GbaEventNode {
                    event: event,
                    cycles: cycles - acc_cycles,
                    next: new_index,
                };
                return;
            }

            let next_acc_cycles = acc_cycles + self.events[next_index].cycles;

            // After the next event, too many cycles would have passed, so we insert the new event
            // before it.
            if next_acc_cycles > cycles {
                self.events[prev_index].next = new_index as GbaEventIndex;
                self.events[new_index] = GbaEventNode {
                    event: event,
                    cycles: cycles - acc_cycles,
                    next: next_index,
                };
                return;
            }

            // Not enough cycles have passed to insert the new event, so we move on:
            acc_cycles = next_acc_cycles;
            prev_index = next_index;
        }
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
