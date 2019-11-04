mod bitmap_modes;
mod tile_modes;
mod blending;
mod obj;

use super::dma;
use super::irq;
use super::{ GbaVideoOutput, GbaMemory };
use blending::{ RawPixel };

pub const HDRAW_WIDTH: u32 = 240;
pub const VDRAW_LINES: u32 = 160;

pub const HBLANK_WIDTH: u32 = 68;
pub const VBLANK_LINES: u32 = 67;

pub const HDRAW_CYCLES: u32 = 960;
pub const HBLANK_CYCLES: u32 = 272;

pub type Line = [u16; 240];
pub type RawLine = [RawPixel; 240];

pub struct GbaLCD {
    pub(crate) end_of_frame: bool,

    /// cycles remaining in the current state (HDRAW or HBLANK)
    cycles_remaining:   u32,
    in_hblank:          bool,
    line_number:        u32,
    line_pixels:        Line,

    /// A line before blending is applied.
    line_raw:           Box<RawLine>,
}

impl GbaLCD {
    pub fn new() -> GbaLCD {
        GbaLCD {
            cycles_remaining:   HDRAW_CYCLES,
            in_hblank:          false,
            line_number:        0,
            line_pixels:        [0; 240],
            line_raw:           Box::new([RawPixel::empty(); 240]),
            end_of_frame:       false,
        }
    }

    pub fn init(&mut self, video: &mut dyn GbaVideoOutput) {
        video.pre_frame();
    }

    #[inline(always)]
    pub fn step(&mut self, cycles: u32, memory: &mut GbaMemory, video: &mut dyn GbaVideoOutput, cpu_enable_irq: bool) {
        self.end_of_frame = false;
        if cycles >= self.cycles_remaining {
            // #NOTE: having this in a separate function and forcing the compiler to inline this
            // one increased performance by like 5-10%
            self.fire(cycles, memory, video, cpu_enable_irq);
        } else {
            self.cycles_remaining -= cycles;
        }
    }

    fn fire(&mut self, cycles: u32, memory: &mut GbaMemory, video: &mut dyn GbaVideoOutput, cpu_enable_irq: bool) {
        if cycles > self.cycles_remaining {
            self.cycles_remaining = cycles - self.cycles_remaining;
        } else {
            self.cycles_remaining = 0;
        }

        if self.in_hblank {
            self.enter_next_line_hdraw(memory, video, cpu_enable_irq);
            self.in_hblank = false;
            self.cycles_remaining += HDRAW_CYCLES;
        } else {
            self.enter_hblank(memory, video, cpu_enable_irq);
            self.in_hblank = true;
            self.cycles_remaining += HBLANK_CYCLES;
        }
    }

    fn enter_hblank(&mut self, memory: &mut GbaMemory, video: &mut dyn GbaVideoOutput, cpu_enable_irq: bool) {
        if self.line_number < VDRAW_LINES {
            self.render_line(memory);
            video.display_line(self.line_number, &self.line_pixels);
            if self.line_number == (VDRAW_LINES - 1) {
                self.end_of_frame = true;
                video.post_frame();
            }
        }

        memory.ioregs.dispstat.set_hblank(true);

        if memory.ioregs.dispstat.hblank_irq_enable() && cpu_enable_irq {
            irq::request_interrupt(memory, irq::GbaInterrupt::HBlank);
        }

        // activate all H-Blank DMAs on VISIBLE H-Blanks
        if self.line_number < VDRAW_LINES {
            for channel in 0..4 {
                let channel_control = memory.ioregs.dma_cnt_h[channel];
                let dma_active = memory.ioregs.internal_dma_registers[channel].active;
                if channel_control.enabled() && !dma_active && channel_control.start_timing() == dma::DMA_TIMING_HBLANK {
                    memory.ioregs.internal_dma_registers[channel].active = true;
                }
            }
        }
    }

    fn enter_next_line_hdraw(&mut self, memory: &mut GbaMemory, video: &mut dyn GbaVideoOutput, cpu_enable_irq: bool) {
        self.line_number += 1;
        memory.ioregs.dispstat.set_hblank(false);

        if self.line_number == (VDRAW_LINES + VBLANK_LINES) {
            memory.ioregs.dispstat.set_vblank(false);

            // on VDRAW start (VBLANK end) we copy the internal point registers into the
            // internal reference point registers for affine BGs.
            self.copy_bg_reference_point_registers(memory);
        } if self.line_number >= (VDRAW_LINES + VBLANK_LINES + 1) {
            self.line_number = 0;

            video.pre_frame();
        } else if self.line_number == VDRAW_LINES {
            memory.ioregs.dispstat.set_vblank(true);

            if memory.ioregs.dispstat.vblank_irq_enable() && cpu_enable_irq {
                irq::request_interrupt(memory, irq::GbaInterrupt::VBlank);
            }
            
            // activate all V-Blank DMAs
            for channel in 0..4 {
                let channel_control = memory.ioregs.dma_cnt_h[channel];
                let dma_active = memory.ioregs.internal_dma_registers[channel].active;
                if channel_control.enabled() && !dma_active && channel_control.start_timing() == dma::DMA_TIMING_VBLANK {
                    memory.ioregs.internal_dma_registers[channel].active = true;
                }
            }
        }

        let vcounter_match = self.line_number as u16 == memory.ioregs.dispstat.vcount_setting();
        memory.ioregs.dispstat.set_vcounter(vcounter_match);
        memory.ioregs.vcount.set_current_scanline(self.line_number as u16);

        if memory.ioregs.dispstat.vcount_irq_enable() && cpu_enable_irq && vcounter_match {
            irq::request_interrupt(memory, irq::GbaInterrupt::VCounterMatch);
        }
    }

    /// Copies the BG2 and BG3 reference point registers into the internal reference point
    /// registers.
    #[inline]
    fn copy_bg_reference_point_registers(&self, memory: &mut GbaMemory) {
        memory.ioregs.internal_bg2x = memory.ioregs.bg2x.to_fp32();
        memory.ioregs.internal_bg2y = memory.ioregs.bg2y.to_fp32();
        memory.ioregs.internal_bg3x = memory.ioregs.bg3x.to_fp32();
        memory.ioregs.internal_bg3y = memory.ioregs.bg3y.to_fp32();
    }

    fn render_line(&mut self, memory: &mut GbaMemory) {
        let backdrop_color = memory.palette.get_bg256(0) | 0x8000;
        let backdrop_raw = RawPixel::backdrop(memory.ioregs.bldcnt, backdrop_color);
        for p in self.line_raw.iter_mut() { *p = backdrop_raw; }

        match memory.ioregs.dispcnt.bg_mode() {
            0 => tile_modes::mode0(self.line_number, &mut self.line_raw, memory),
            1 => tile_modes::mode1(self.line_number, &mut self.line_raw, memory),
            2 => tile_modes::mode2(self.line_number, &mut self.line_raw, memory),
            3 => bitmap_modes::mode3(self.line_number, &mut self.line_raw, memory),
            4 => bitmap_modes::mode4(self.line_number, &mut self.line_raw, memory),
            5 => bitmap_modes::mode5(self.line_number, &mut self.line_raw, memory),

            bad_mode => {
                println!("BAD MODE {}", bad_mode);
                for out_pixel in self.line_pixels.iter_mut() {
                    *out_pixel = 0;
                }
            },
        }

        let special_effects = blending::SpecialEffects::new(memory.ioregs.bldcnt, memory.ioregs.bldalpha, memory.ioregs.bldy);
        blending::blend_raw_pixels(&self.line_raw, &mut self.line_pixels, special_effects);
    }
}
