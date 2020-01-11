use super::obj::{render_objects, ObjectPriority};
use super::palette::GbaPalette;
use super::{LCDLineBuffer, LCDRegisters, WindowInfo};
use crate::hardware::{OAM, VRAM};
use crate::util::memory::read_u16_unchecked;

pub type Mode4FrameBuffer = [u8; 0x9600];
pub type Mode5FrameBuffer = [u8; 0xA000];

macro_rules! run_between_bm_objs {
    ($Registers:expr, $VRAM:expr, $OAM:expr, $PAL:expr, $Pixels:expr, $WindowInfo:expr, $RenderBlock:block) => {
        let object_priorities = ObjectPriority::sorted($OAM);
        let bg2_priority = $Registers.bg_cnt[2].priority();

        if $Registers.dispcnt.display_layer(4) {
            // Draw all OBJs that are below the bitmap layer (with a greather priority value).
            ((bg2_priority + 1)..=3).rev().for_each(|p| {
                render_objects(
                    $Registers,
                    object_priorities.objects_with_priority(p as usize),
                    $VRAM,
                    $OAM,
                    $PAL,
                    $Pixels,
                    $WindowInfo,
                )
            });
        }

        $RenderBlock

        if $Registers.dispcnt.display_layer(4) {
            // Draw ll OBJs that are above the bitmap layer (with a lower or equal priority value).
            (0u16..=bg2_priority)
                .rev()
                .for_each(|p| {
                    render_objects(
                        $Registers,
                        object_priorities.objects_with_priority(p as usize),
                        $VRAM,
                        $OAM,
                        $PAL,
                        $Pixels,
                        $WindowInfo,
                    )
                });
        }
    };
}

pub fn render_mode3(
    registers: &LCDRegisters,
    vram: &VRAM,
    oam: &OAM,
    pal: &GbaPalette,
    pixels: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
    run_between_bm_objs!(registers, vram, oam, pal, pixels, window_info, {
        if registers.dispcnt.display_layer(2) {
            render_mode3_bitmap(
                registers.line as usize,
                vram,
                registers.effects.is_first_target(2),
                registers.effects.is_second_target(2),
                pixels,
                window_info,
            );
        }
    });
}

fn render_mode3_bitmap(
    line: usize,
    vram: &VRAM,
    first_target: bool,
    second_target: bool,
    pixels: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
    // assert!(line < 160);

    // let line_offset = 480 * line;
    // for x in 0..240 {
    //     let effects = if !window_info.enabled {
    //         true
    //     } else {
    //         if let Some(effects) = window_info.check_pixel_obj_window(2, x as u16, line as u16) {
    //             effects
    //         } else {
    //             continue;
    //         }
    //     };

    //     // Bounds check at the top of the function ensures that we never go above 75KB (max address
    //     // is actually 0x12BFE).  The compiler just doesn't seem to be able to optimize the checks
    //     // away here though.  Doing it this way removes bounds checks and allows auto vectorization :o
    //     pixels.push_pixel(
    //         x,
    //         unsafe { read_u16_unchecked(vram, line_offset + x * 2) } | 0x8000,
    //         effects & first_target,
    //         effects & second_target,
    //         false,
    //     );
    // }
}

pub fn render_mode4(
    registers: &LCDRegisters,
    vram: &VRAM,
    oam: &OAM,
    pal: &GbaPalette,
    pixels: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
    const FRAMEBUFFER0_OFFSET: usize = 0x0000;
    const FRAMEBUFFER1_OFFSET: usize = 0xA000;
    const FRAMEBUFFER_SIZE: usize = 0x9600;

    run_between_bm_objs!(registers, vram, oam, pal, pixels, window_info, {
        if registers.dispcnt.display_layer(2) {
            let framebuffer_start = if registers.dispcnt.frame_select() == 0 {
                FRAMEBUFFER0_OFFSET
            } else {
                FRAMEBUFFER1_OFFSET
            };
            let framebuffer_end = framebuffer_start + FRAMEBUFFER_SIZE;
            assert!(vram.len() >= framebuffer_start && framebuffer_end <= vram.len());

            render_mode4_bitmap(
                registers.line as usize,
                unsafe {
                    std::mem::transmute((&vram[framebuffer_start..framebuffer_end]).as_ptr())
                },
                pal,
                registers.effects.is_first_target(2),
                registers.effects.is_second_target(2),
                pixels,
                window_info,
            );
        }
    });
}

fn render_mode4_bitmap(
    line: usize,
    framebuffer: &Mode4FrameBuffer,
    pal: &GbaPalette,
    first_target: bool,
    second_target: bool,
    pixels: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
    // assert!(line < 160);

    // let line_offset = 240 * line;
    // for x in 0..240 {
    //     if let Some(effects) = window_info.check_pixel_obj_window(2, x as u16, line as u16) {
    //         // Bounds check at the top of the function ensures that we never go above 75KB (max address
    //         // is actually 0x12BFE).  The compiler just doesn't seem to be able to optimize the checks
    //         // away here though.  Doing it this way removes bounds checks and allows auto vectorization :o
    //         let palette_entry = framebuffer[line_offset + x];
    //         if palette_entry != 0 {
    //             pixels.push_pixel(
    //                 x,
    //                 pal.bg256(palette_entry as usize),
    //                 effects & first_target,
    //                 effects & second_target,
    //                 false,
    //             );
    //         }
    //     }
    // }
}

pub fn render_mode5(
    registers: &LCDRegisters,
    vram: &VRAM,
    oam: &OAM,
    pal: &GbaPalette,
    pixels: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
    const FRAMEBUFFER0_OFFSET: usize = 0x0000;
    const FRAMEBUFFER1_OFFSET: usize = 0xA000;
    const FRAMEBUFFER_SIZE: usize = 0xA000;

    run_between_bm_objs!(registers, vram, oam, pal, pixels, window_info, {
        if registers.dispcnt.display_layer(2) {
            let framebuffer_start = if registers.dispcnt.frame_select() == 0 {
                FRAMEBUFFER0_OFFSET
            } else {
                FRAMEBUFFER1_OFFSET
            };
            let framebuffer_end = framebuffer_start + FRAMEBUFFER_SIZE;
            assert!(vram.len() >= framebuffer_start && framebuffer_end <= vram.len());

            if registers.line < 128 {
                render_mode5_bitmap(
                    registers.line as usize,
                    unsafe {
                        std::mem::transmute((&vram[framebuffer_start..framebuffer_end]).as_ptr())
                    },
                    registers.effects.is_first_target(2),
                    registers.effects.is_second_target(2),
                    pixels,
                    window_info,
                );
            }
        }
    });
}

fn render_mode5_bitmap(
    line: usize,
    framebuffer: &Mode5FrameBuffer,
    first_target: bool,
    second_target: bool,
    pixels: &mut LCDLineBuffer,
    window_info: &WindowInfo,
) {
    // assert!(line < 160);

    // let line_offset = 480 * line;
    // for x in 0..160 {
    //     if let Some(effects) = window_info.check_pixel_obj_window(2, x as u16, line as u16) {
    //         // Bounds checks are basically done at the top of the function using the asserts.
    //         // This ensures that we never go above 75KB (max address is actually 0x12BFE).
    //         // The compiler just doesn't seem to be able to optimize the checks away here though.
    //         // Doing it this way removes bounds checks and allows auto vectorization :o
    //         pixels.push_pixel(
    //             x,
    //             unsafe { read_u16_unchecked(framebuffer, line_offset + x * 2) } | 0x8000,
    //             effects & first_target,
    //             effects & second_target,
    //             false,
    //         );
    //     }
    // }
}
