use crate::hardware::{ VRAM, OAM };
use crate::util::memory::read_u16_unchecked;
use super::obj::ObjectPriority;
use super::{ LCDRegisters, LCDLineBuffer };
use super::palette::GbaPalette;

pub type Mode4FrameBuffer = [u8; 0x9600];
pub type Mode5FrameBuffer = [u8; 0xA000];

pub fn render_mode3(registers: &LCDRegisters, vram: &VRAM, oam: &OAM, pal: &GbaPalette, pixels: &mut LCDLineBuffer) {
    let object_priorities = ObjectPriority::sorted(oam);
    let bg2_priority = registers.bg_cnt[2].priority();

    if registers.dispcnt.display_layer(4) {
        if bg2_priority < 3 { render_objects_placeholder(object_priorities.objects_with_priority(3), vram, oam, pal, pixels); }
        if bg2_priority < 2 { render_objects_placeholder(object_priorities.objects_with_priority(2), vram, oam, pal, pixels); }
        if bg2_priority < 1 { render_objects_placeholder(object_priorities.objects_with_priority(1), vram, oam, pal, pixels); }
    }

    if registers.dispcnt.display_layer(2) {
        render_mode3_bitmap(registers.line as usize, 0, 240, vram, pixels);
    }

    if registers.dispcnt.display_layer(4) {
        if bg2_priority >= 3 { render_objects_placeholder(object_priorities.objects_with_priority(3), vram, oam, pal, pixels); }
        if bg2_priority >= 2 { render_objects_placeholder(object_priorities.objects_with_priority(2), vram, oam, pal, pixels); }
        if bg2_priority >= 1 { render_objects_placeholder(object_priorities.objects_with_priority(1), vram, oam, pal, pixels); }
        render_objects_placeholder(object_priorities.objects_with_priority(0), vram, oam, pal, pixels); 
    }
}

fn render_mode3_bitmap(line: usize, left: usize, right: usize, vram: &VRAM, pixels: &mut LCDLineBuffer) {
    assert!(line < 160);
    assert!(left < 240 && right <= 240);

    let line_offset = 480 * line;
    for x in left..right {
        // Bounds checks are basically done at the top of the function using the asserts.
        // This ensures that we never go above 75KB (max address is actually 0x12BFE).
        // The compiler just doesn't seem to be able to optimize the checks away here though.
        // Doing it this way removes bounds checks and allows auto vectorization :o
        pixels.push_pixel_fast(x, unsafe { read_u16_unchecked(vram, line_offset + x * 2) } | 0x8000);
    }
}

pub fn render_mode4(registers: &LCDRegisters, vram: &VRAM, oam: &OAM, pal: &GbaPalette, pixels: &mut LCDLineBuffer) {
    const FRAMEBUFFER0_OFFSET: usize = 0x0000;
    const FRAMEBUFFER1_OFFSET: usize = 0xA000;
    const FRAMEBUFFER_SIZE: usize = 0x9600;

    let object_priorities = ObjectPriority::sorted(oam);
    let bg2_priority = registers.bg_cnt[2].priority();

    if registers.dispcnt.display_layer(4) {
        if bg2_priority < 3 { render_objects_placeholder(object_priorities.objects_with_priority(3), vram, oam, pal, pixels); }
        if bg2_priority < 2 { render_objects_placeholder(object_priorities.objects_with_priority(2), vram, oam, pal, pixels); }
        if bg2_priority < 1 { render_objects_placeholder(object_priorities.objects_with_priority(1), vram, oam, pal, pixels); }
    }

    if registers.dispcnt.display_layer(2) {
        let framebuffer_start = if registers.dispcnt.frame_select() == 0 { FRAMEBUFFER0_OFFSET } else { FRAMEBUFFER1_OFFSET };
        let framebuffer_end = framebuffer_start + FRAMEBUFFER_SIZE;
        assert!(vram.len() >= framebuffer_start && framebuffer_end <= vram.len());

        render_mode4_bitmap(registers.line as usize, 0, 240, unsafe {
            std::mem::transmute((&vram[framebuffer_start..framebuffer_end]).as_ptr())
        }, pal, pixels);
    }

    if registers.dispcnt.display_layer(4) {
        if bg2_priority >= 3 { render_objects_placeholder(object_priorities.objects_with_priority(3), vram, oam, pal, pixels); }
        if bg2_priority >= 2 { render_objects_placeholder(object_priorities.objects_with_priority(2), vram, oam, pal, pixels); }
        if bg2_priority >= 1 { render_objects_placeholder(object_priorities.objects_with_priority(1), vram, oam, pal, pixels); }
        render_objects_placeholder(object_priorities.objects_with_priority(0), vram, oam, pal, pixels); 
    }
}

fn render_mode4_bitmap(line: usize, left: usize, right: usize, framebuffer: &Mode4FrameBuffer, pal: &GbaPalette, pixels: &mut LCDLineBuffer) {
    assert!(line < 160);
    assert!(left < 240 && right <= 240);

    let line_offset = 240 * line;
    for x in left..right {
        // Bounds checks are basically done at the top of the function using the asserts.
        // This ensures that we never go above 75KB (max address is actually 0x12BFE).
        // The compiler just doesn't seem to be able to optimize the checks away here though.
        // Doing it this way removes bounds checks and allows auto vectorization :o
        let palette_entry = framebuffer[line_offset + x];
        if palette_entry != 0 {
            pixels.push_pixel_fast(x, pal.bg256(palette_entry as usize));
        }
    }
}

pub fn render_mode5(registers: &LCDRegisters, vram: &VRAM, oam: &OAM, pal: &GbaPalette, pixels: &mut LCDLineBuffer) {
    const FRAMEBUFFER0_OFFSET: usize = 0x0000;
    const FRAMEBUFFER1_OFFSET: usize = 0xA000;
    const FRAMEBUFFER_SIZE: usize = 0xA000;

    let object_priorities = ObjectPriority::sorted(oam);
    let bg2_priority = registers.bg_cnt[2].priority();

    if registers.dispcnt.display_layer(4) {
        if bg2_priority < 3 { render_objects_placeholder(object_priorities.objects_with_priority(3), vram, oam, pal, pixels); }
        if bg2_priority < 2 { render_objects_placeholder(object_priorities.objects_with_priority(2), vram, oam, pal, pixels); }
        if bg2_priority < 1 { render_objects_placeholder(object_priorities.objects_with_priority(1), vram, oam, pal, pixels); }
    }

    if registers.dispcnt.display_layer(2) {
        let framebuffer_start = if registers.dispcnt.frame_select() == 0 { FRAMEBUFFER0_OFFSET } else { FRAMEBUFFER1_OFFSET };
        let framebuffer_end = framebuffer_start + FRAMEBUFFER_SIZE;
        assert!(vram.len() >= framebuffer_start && framebuffer_end <= vram.len());

        if registers.line < 128 {
            render_mode5_bitmap(registers.line as usize, 0, 240, unsafe {
                std::mem::transmute((&vram[framebuffer_start..framebuffer_end]).as_ptr())
            }, pixels);
        }
    }

    if registers.dispcnt.display_layer(4) {
        if bg2_priority >= 3 { render_objects_placeholder(object_priorities.objects_with_priority(3), vram, oam, pal, pixels); }
        if bg2_priority >= 2 { render_objects_placeholder(object_priorities.objects_with_priority(2), vram, oam, pal, pixels); }
        if bg2_priority >= 1 { render_objects_placeholder(object_priorities.objects_with_priority(1), vram, oam, pal, pixels); }
        render_objects_placeholder(object_priorities.objects_with_priority(0), vram, oam, pal, pixels); 
    }
}

fn render_mode5_bitmap(line: usize, left: usize, right: usize, framebuffer: &Mode5FrameBuffer, pixels: &mut LCDLineBuffer) {
    assert!(line < 160);
    assert!(left < 240 && right <= 240);

    let line_offset = 480 * line;
    for x in left..std::cmp::min(right, 160) {
        // Bounds checks are basically done at the top of the function using the asserts.
        // This ensures that we never go above 75KB (max address is actually 0x12BFE).
        // The compiler just doesn't seem to be able to optimize the checks away here though.
        // Doing it this way removes bounds checks and allows auto vectorization :o
        pixels.push_pixel_fast(x, unsafe { read_u16_unchecked(framebuffer, line_offset + x * 2) } | 0x8000);
    }
}

#[inline(never)]
fn render_objects_placeholder(objects: &[u16], _vram: &VRAM, _oam: &OAM, _pal: &GbaPalette, _pixels: &mut LCDLineBuffer) {
}
