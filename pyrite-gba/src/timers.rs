use super::memory::GbaMemory;

#[inline(always)]
pub fn is_any_timer_active(memory: &GbaMemory) -> bool {
    memory.ioregs.tm_cnt_h[0].enabled() |
    (memory.ioregs.tm_cnt_h[1].enabled() && !memory.ioregs.tm_cnt_h[1].count_up_timing()) |
    (memory.ioregs.tm_cnt_h[2].enabled() && !memory.ioregs.tm_cnt_h[2].count_up_timing()) |
    (memory.ioregs.tm_cnt_h[3].enabled() && !memory.ioregs.tm_cnt_h[3].count_up_timing())
}

pub fn step_active_timers(cycles: u32, memory: &mut GbaMemory) {
    for timer in 0usize..4usize {
        // we ignore count-up timers (unless it's timer #0), because they are incremented by the
        // previous timer.
        if memory.ioregs.tm_cnt_h[timer].enabled() && (timer == 0 || !memory.ioregs.tm_cnt_h[timer].count_up_timing()) {
            increment_timer(timer, cycles, memory);
        }
    }
}

fn increment_timer(mut timer: usize, mut cycles: u32, memory: &mut GbaMemory) {
    loop {
        if memory.ioregs.internal_tm_counter[timer].increment(cycles) {
            // timers are reloaded on overflow:
            memory.ioregs.internal_tm_counter[timer].set_counter(memory.ioregs.tm_cnt_l[timer].inner);

            // if the current timer has overflowed and the next timer has the count-up flag set, we
            // adjust the cycles and repeat this loop with the next timer:
            if (timer < 3) && memory.ioregs.tm_cnt_h[timer + 1].count_up_timing() {
                cycles = 1;
                timer += 1;
                continue;
            }
        }
        break;
    }
}
