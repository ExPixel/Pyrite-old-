#include <gba_sound.h>
#include "../include/video_ext.h"

int main(void) {
    busy_render_wait();

    // Turn on the sound circuit.
    REG_SOUNDCNT_X = 0x80;

    // Full volume.
    REG_SOUNDCNT_L = 0x1177;
    // Full output ratio.
    REG_SOUNDCNT_H = 0x2;

    REG_SOUND1CNT_H = 0xF780;
    REG_SOUND1CNT_X = 0x8400;

    REG_SOUND2CNT_L = 0xF780;
    REG_SOUND2CNT_H = 0x84FC;

    while(1);
}
