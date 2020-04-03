#include <gba_types.h>
#include <gba_video.h>
#include <../../include/test.h>
#include <../../include/video_ext.h>

void setup_palette();

int main(void) {
    TEST_STATUS = test_status_setup;
    REG_DISPCNT = MODE_4 | BG2_ENABLE;
    setup_palette();

    for (u32 y = 0; y < SCREEN_HEIGHT; y++) {
        for (u32 x = 0; x < SCREEN_WIDTH; x++) {
            u8 entry = (u8)(x + y*SCREEN_WIDTH);
            MODE4_FB[y][x] = entry;
            MODE4_BB[y][x] = 255 - entry;
        }
    }
    busy_render_wait();
    TEST_STATUS = test_status_ready;

    REG_DISPCNT = MODE_4 | BG2_ENABLE | BACKBUFFER;
    busy_render_wait();
    TEST_STATUS = test_status_break;

    while(1);
}

void setup_palette() {
    u16 r = 0;
    u16 g = 0;
    u16 b = 0;

    for (int idx = 0; idx < 128; idx++) {
        r = (r + 1) & 0x1F;
        g = (g + r) & 0x1F;
        b = (b + g) & 0x1F;
        BG_PALETTE[idx] = RGB5(r, g, b);
    }

    for (int idx = 128; idx < 256; idx++) {
        b = (b + 3) & 0x1F;
        g = (g + b) & 0x1F;
        r = (r + g) & 0x1F;
        BG_PALETTE[idx] = RGB5(r, g, b);
    }
}
