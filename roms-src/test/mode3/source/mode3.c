#include <gba_types.h>
#include <gba_video.h>
#include <../../include/test.h>
#include <../../include/video_ext.h>

u16 color_for_coord(u32 x, u32 y);

int main(void) {
    TEST_STATUS = test_status_setup;
    REG_DISPCNT = MODE_3 | BG2_ENABLE;
    for (u32 y = 0; y < SCREEN_HEIGHT; y++) {
        for (u32 x = 0; x < SCREEN_WIDTH; x++) {
            MODE3_FB[y][x] = color_for_coord(x, y);
        }
    }
    busy_render_wait();
    TEST_STATUS = test_status_ready;
    while(1);
}

u16 color_for_coord(u32 x, u32 y) {
    u16 r = x & 0x1F;
    u16 g = y & 0x1F;
    u16 b = (x ^ y) & 0x1F;
    return RGB5(r, g, b);
}
