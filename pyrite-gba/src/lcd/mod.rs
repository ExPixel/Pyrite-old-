mod bitmap_modes;
mod tile_modes;

use super::{ GbaVideoOutput, GbaMemory, ArmCpu };

pub const HDRAW_WIDTH: u32 = 240;
pub const VDRAW_LINES: u32 = 160;

pub const HBLANK_WIDTH: u32 = 68;
pub const VBLANK_LINES: u32 = 68;

pub const HDRAW_CYCLES: u32 = 960;
pub const HBLANK_CYCLES: u32 = 272;

pub struct GbaLCD {
    pub(crate) end_of_frame: bool,

    /// cycles remaining in the current state (HDRAW or HBLANK)
    cycles_remaining:   u32,
    in_hblank:          bool,
    line_number:        u32,
    line_pixels:        [u16; 240],
}

impl GbaLCD {
    pub fn new() -> GbaLCD {
        GbaLCD {
            cycles_remaining:   HDRAW_CYCLES,
            in_hblank:          false,
            line_number:        0,
            line_pixels:        [0; 240],
            end_of_frame:       false,
        }
    }

    pub fn init(&mut self, _cpu: &mut ArmCpu, _memory: &mut GbaMemory, video: &mut dyn GbaVideoOutput) {
        video.pre_frame();
    }

    pub fn step(&mut self, cycles: u32, cpu: &mut ArmCpu, memory: &mut GbaMemory, video: &mut dyn GbaVideoOutput) {
        self.end_of_frame = false;
        if cycles >= self.cycles_remaining {
            if cycles > self.cycles_remaining {
                self.cycles_remaining = cycles - self.cycles_remaining;
            } else {
                self.cycles_remaining = 0;
            }

            if self.in_hblank {
                self.enter_next_line_hdraw(cpu, memory, video);
                self.in_hblank = false;
                self.cycles_remaining += HDRAW_CYCLES;
            } else {
                self.enter_hblank(cpu, memory, video);
                self.in_hblank = true;
                self.cycles_remaining += HBLANK_CYCLES;
            }
        } else {
            self.cycles_remaining -= cycles;
        }
    }

    fn enter_hblank(&mut self, _cpu: &mut ArmCpu, memory: &mut GbaMemory, video: &mut dyn GbaVideoOutput) {
        if self.line_number < VDRAW_LINES {
            self.render_line(memory);
            video.display_line(self.line_number, &self.line_pixels);
            if self.line_number == (VDRAW_LINES - 1) {
                self.end_of_frame = true;
                video.post_frame();
            }
        }

        memory.ioregs.dispstat.set_hblank(true);
    }

    fn enter_next_line_hdraw(&mut self, _cpu: &mut ArmCpu, memory: &mut GbaMemory, video: &mut dyn GbaVideoOutput) {
        self.line_number += 1;
        memory.ioregs.dispstat.set_hblank(false);

        if self.line_number >= (VDRAW_LINES + VBLANK_LINES) {
            self.line_number = 0;
            memory.ioregs.dispstat.set_vblank(false);
            video.pre_frame();
        } else if self.line_number >= VDRAW_LINES {
            memory.ioregs.dispstat.set_vblank(true);
        } else {
            memory.ioregs.dispstat.set_vblank(false);
        }

        memory.ioregs.dispstat.set_vcounter(self.line_number as u16 == memory.ioregs.dispstat.vcount_setting());
        memory.ioregs.vcount.set_current_scanline(self.line_number as u16);
    }

    fn render_line(&mut self, memory: &mut GbaMemory) {
        match memory.ioregs.dispcnt.bg_mode() {
            0 => tile_modes::mode0(self.line_number, &mut self.line_pixels, memory),
            1 => tile_modes::mode1(self.line_number, &mut self.line_pixels, memory),
            2 => tile_modes::mode2(self.line_number, &mut self.line_pixels, memory),
            3 => bitmap_modes::mode3(self.line_number, &mut self.line_pixels, memory),
            4 => bitmap_modes::mode4(self.line_number, &mut self.line_pixels, memory),
            5 => bitmap_modes::mode5(self.line_number, &mut self.line_pixels, memory),

            bad_mode => {
                println!("BAD MODE {}", bad_mode);
                for out_pixel in self.line_pixels.iter_mut() {
                    *out_pixel = 0;
                }
            },
        }
    }
}
