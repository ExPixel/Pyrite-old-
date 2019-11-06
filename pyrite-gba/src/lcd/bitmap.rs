use crate::hardware::{ VRAM, OAM };
use crate::util::memory::read_u16_unchecked;
use super::obj::ObjectPriority;
use super::LCDRegisters;
use super::palette::GbaPalette;

pub fn render_mode3(registers: &LCDRegisters, vram: &VRAM, oam: &OAM, pal: &GbaPalette, pixels: &mut [u16; 240]) {
    let object_priorities = ObjectPriority::sorted(oam);
    let bg2_priority = registers.bg_cnt[2].priority();

    if bg2_priority < 3 { render_objects_placeholder(object_priorities.objects_with_priority(3), vram, oam, pal, pixels); }
    if bg2_priority < 2 { render_objects_placeholder(object_priorities.objects_with_priority(2), vram, oam, pal, pixels); }
    if bg2_priority < 1 { render_objects_placeholder(object_priorities.objects_with_priority(1), vram, oam, pal, pixels); }

    render_mode3_bitmap_no_obj_window(registers.line as usize, 0, 240, vram, pixels);

    if bg2_priority >= 3 { render_objects_placeholder(object_priorities.objects_with_priority(3), vram, oam, pal, pixels); }
    if bg2_priority >= 2 { render_objects_placeholder(object_priorities.objects_with_priority(2), vram, oam, pal, pixels); }
    if bg2_priority >= 1 { render_objects_placeholder(object_priorities.objects_with_priority(1), vram, oam, pal, pixels); }

    render_objects_placeholder(object_priorities.objects_with_priority(0), vram, oam, pal, pixels); 
}

fn render_mode3_bitmap_no_obj_window(line: usize, left: usize, right: usize, vram: &VRAM, pixels: &mut [u16; 240]) {
    assert!(line < 160);
    assert!(left < 240 && right <= 240);

    const FRAMEBUFFER_OFFSET: usize = 0;

    let line_offset = 480 * line;
    for x in left..right {
        // Bounds checks are basically done at the top of the function using the asserts.
        // This ensures that we never go above 75KB (max address is actually 0x12BFE).
        // The compiler just doesn't seem to be able to optimize the checks away here though.
        // Doing it this way removes bounds checks and allows auto vectorization :o
        pixels[x] = unsafe { read_u16_unchecked(vram, line_offset + x * 2) } | 0x8000;
    }
}

#[inline(never)]
fn render_objects_placeholder(objects: &[u16], _vram: &VRAM, _oam: &OAM, _pal: &GbaPalette, _pixels: &mut [u16; 240]) {
}
