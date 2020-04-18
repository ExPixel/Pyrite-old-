#include <gba_console.h>
#include <gba_video.h>
#include <gba_interrupt.h>
#include <gba_systemcalls.h>
#include <gba_timers.h>
#include <stdio.h>
#include <stdlib.h>

#include "../include/timer_ext.h"

void setup_timers();

int main(void) {
	// the vblank interrupt must be enabled for VBlankIntrWait() to work
	// since the default dispatcher handles the bios flags no vblank handler
	// is required
	irqInit();
	irqEnable(IRQ_VBLANK);

	consoleDemoInit();

    setup_timers();

    while (1) {
		VBlankIntrWait();

        u16 seconds = REG_TM2CNT_L >> 8;

        // ansi escape sequence to set print co-ordinates
        // /x1b[line;columnH
        iprintf("\x1b[10;10HHello: %d\n", seconds);
    }
}

void setup_timers() {
    // REG_TM2CNT_L >> 8 will count seconds (it wraps a lot :o).
    REG_TM2CNT_L = 0;
    REG_TM2CNT_H = TIMER_COUNT | TIMER_START;

    // TM1 will overflow after every 65,536 cycles
    REG_TM1CNT_L = 0;
    REG_TM1CNT_H = TIMER_COUNT | TIMER_START;

    // TM0 will overflow every cycle:
    REG_TM0CNT_L = 0xFFFF;
    REG_TM0CNT_H = TIMER_SCALE_1 | TIMER_START;
}
