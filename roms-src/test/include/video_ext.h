#ifndef _video_ext_
#define _video_ext_

typedef u16 MODE4_LINE[120];

#define MODE4_FB ((MODE4_LINE *)0x06000000)
#define MODE4_BB ((MODE4_LINE *)0x0600A000)

/*
 * compressed version of this:
 *   u16 current = buffer[y][x >> 1];
 *   current &= 0xFF00 >> ((x & 1) << 3);
 *   current |= ((u16)entry) << ((x & 1) << 3);
 *   buffer[y][x >> 1] = current;
 */
#define MODE4_POKE(buffer, x, y, entry) \
    (buffer[y][x >> 1] = \
    (buffer[y][x >> 1] & 0xFF00 >> ((x & 1) << 3)) | \
    ((u16)entry) << ((x & 1) << 3)) \

/*
 * Wait for VBlank in a busy loop.
 * Continuously checks for the VBLANK flag in DISPSTAT.
 */
static void busy_vblank_wait() {
    // Just loop while the VBlank flag is clear.
    while (!(REG_DISPSTAT & LCDC_VBL_FLAG));
}

/*
 * Wait for VDraw in a busy loop.
 * Continuously checks for the VBLANK flag clear in DISPSTAT.
 */
static void busy_vdraw_wait() {
    // Just loop while the VBlank flag is set.
    while (REG_DISPSTAT & LCDC_VBL_FLAG);
}

/*
 * Waits for whatever was drawn to the framebuffer to be fully rendered.
 * Waits for VDraw to be entered and then waits until VBlank.
 */
static void busy_render_wait() {
    // If we're starting in the middle of VDraw, wait for vblank.
    if (!(REG_DISPSTAT & LCDC_VBL_FLAG)) {
        busy_vblank_wait();
    }
    busy_vdraw_wait();
    busy_vblank_wait();
}

#endif /* _video_ext_ */
