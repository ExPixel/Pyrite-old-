use super::{apply_mosaic_cond, LCDLineBuffer, LCDRegisters, Layer, Pixel};
use crate::hardware::{OAM, VRAM};
use crate::util::fixedpoint::{FixedPoint16, FixedPoint32};
use crate::util::memory::{read_u16, read_u16_unchecked};

const OBJ_WINDOW: bool = true;
const OBJ_RENDER: bool = false;

const BITMAP: bool = true;
const TILEMAP: bool = false;

macro_rules! define_obj_renderer {
    ($FunctionName:ident, $OBJWindowMode:expr, $BitmapMode:expr) => {
        pub fn $FunctionName(
            registers: &LCDRegisters,
            objects: &[u16],
            vram: &VRAM,
            oam: &OAM,
            pixels: &mut LCDLineBuffer,
        ) {
            if objects.len() == 0 {
                return;
            }

            if (pixels.windows.enabled && !pixels.windows.line_visible(Layer::OBJ, registers.line))
            {
                return;
            }

            let first_target = registers.effects.is_first_target(Layer::OBJ);
            let second_target = registers.effects.is_second_target(Layer::OBJ);

            for obj_index in objects.iter().map(|x| (*x & 0xFF) as usize) {
                let attr_index = obj_index * 8;
                let attrs = unsafe {
                    (
                        ObjAttr0::wrap(read_u16_unchecked(oam, attr_index)),
                        ObjAttr1::wrap(read_u16_unchecked(oam, attr_index + 2)),
                        ObjAttr2::wrap(read_u16_unchecked(oam, attr_index + 4)),
                    )
                };
                let one_dimensional = registers.dispcnt.one_dimensional_obj();
                let (mosaic_x, mosaic_y) =
                    (registers.mosaic.obj.0 as u16, registers.mosaic.obj.1 as u16);

                let (obj_width, obj_height) = attrs.0.shape().size(attrs.1.size_select());
                // @TODO probably don't need to check affine here because if it wasn't set the object would
                // be disabled an we wouldn't end up here:
                let (obj_display_width, obj_display_height) =
                    if attrs.0.affine() && attrs.0.double_size() {
                        (obj_width * 2, obj_height * 2)
                    } else {
                        (obj_width, obj_height)
                    };

                let mut obj_screen_left = attrs.1.x();
                let obj_screen_top = attrs.0.y();
                let obj_screen_bottom = (obj_screen_top + obj_display_height - 1) % 256;

                let in_bounds_vertical = obj_screen_top <= obj_screen_bottom
                    && obj_screen_top <= registers.line
                    && obj_screen_bottom >= registers.line;
                let in_bounds_vertical_wrapped = obj_screen_top > obj_screen_bottom
                    && (obj_screen_top <= registers.line || obj_screen_bottom >= registers.line);

                if !in_bounds_vertical && !in_bounds_vertical_wrapped {
                    continue;
                }

                let mut obj_screen_right;

                // horizontally offscreen objects still take cycles so we process those before horizontal
                // occlusion.
                if attrs.0.affine() {
                    // affine objects require 10 cycles to start
                    if pixels.obj_cycles > 10 {
                        pixels.obj_cycles -= 10;
                        if (obj_display_width * 2) > pixels.obj_cycles {
                            obj_screen_right =
                                (obj_screen_left + (pixels.obj_cycles / 2) - 1) % 512;
                            pixels.obj_cycles = 0;
                        } else {
                            obj_screen_right = (obj_screen_left + obj_display_width - 1) % 512;
                            pixels.obj_cycles -= obj_display_width * 2;
                        }
                    } else {
                        pixels.obj_cycles = 0;
                        return;
                    }
                } else {
                    if pixels.obj_cycles == 0 {
                        return;
                    }
                    if obj_display_width > pixels.obj_cycles {
                        obj_screen_right = (obj_screen_left + pixels.obj_cycles - 1) % 512;
                        pixels.obj_cycles = 0;
                    } else {
                        obj_screen_right = (obj_screen_left + obj_display_width - 1) % 512;
                        pixels.obj_cycles -= obj_display_width;
                    }
                }

                let in_bounds_horizontal = obj_screen_left < 240 || obj_screen_right < 240;
                if !in_bounds_horizontal {
                    continue;
                }

                let (obj_xdraw_start, _obj_xdraw_end) = if obj_screen_left < obj_screen_right {
                    (
                        0,
                        if obj_screen_right >= 240 {
                            obj_screen_right = 239;
                            240 - obj_screen_left - 1
                        } else {
                            obj_display_width - 1
                        },
                    )
                } else {
                    // we have wrapped here so we need to start drawing farther to the right
                    // of the object, but there will always be enough space on screen to draw the
                    // object to the end.
                    obj_screen_left = 0;
                    (
                        obj_display_width - obj_screen_right - 1,
                        obj_display_width - 1,
                    )
                };

                let obj_origin_x = FixedPoint32::from(obj_display_width / 2);
                let obj_origin_y = FixedPoint32::from(obj_display_height / 2);

                let obj_xdraw_start = FixedPoint32::from(obj_xdraw_start);
                let obj_ydraw_start = if registers.line > obj_screen_bottom {
                    FixedPoint32::from(registers.line - obj_screen_top)
                } else {
                    FixedPoint32::from(
                        obj_display_height - (obj_screen_bottom - registers.line) - 1,
                    )
                };

                let mut obj_xdraw_start_distance = obj_xdraw_start - obj_origin_x;
                let mut obj_ydraw_start_distance = obj_ydraw_start - obj_origin_y;

                let obj_dx;
                let obj_dmx;
                let obj_dy;
                let obj_dmy;
                if attrs.0.affine() {
                    let params_idx = attrs.1.affine_param_index() as usize;
                    obj_dx = FixedPoint32::from(FixedPoint16::wrap(
                        (read_u16(oam, 0x06 + (params_idx * 32))) as i16,
                    ));
                    obj_dmx = FixedPoint32::from(FixedPoint16::wrap(
                        (read_u16(oam, 0x0E + (params_idx * 32))) as i16,
                    ));
                    obj_dy = FixedPoint32::from(FixedPoint16::wrap(
                        (read_u16(oam, 0x16 + (params_idx * 32))) as i16,
                    ));
                    obj_dmy = FixedPoint32::from(FixedPoint16::wrap(
                        (read_u16(oam, 0x1E + (params_idx * 32))) as i16,
                    ));
                } else {
                    obj_dy = FixedPoint32::from(0u32);
                    obj_dmx = FixedPoint32::from(0u32);
                    obj_dmy = FixedPoint32::from(1u32);

                    if attrs.1.flip_horizontal() {
                        obj_dx = FixedPoint32::from(-1i32);

                        // @NOTE add 1 so that we start on the other side of the center line...if that makes sense :|
                        obj_xdraw_start_distance += FixedPoint32::wrap(0x100);
                    } else {
                        obj_dx = FixedPoint32::from(1u32);
                    }

                    if attrs.1.flip_vertical() {
                        obj_ydraw_start_distance = -obj_ydraw_start_distance;
                    }
                }

                // Down here we use the real width and height for the origin instead of the double sized
                // because I randomly wrote it and it works. Maybe one day I'll actually do the math and
                // come up with an exact reason as to why. For now I just had a feeling and I was right.
                let mut obj_x = FixedPoint32::from(obj_width / 2)
                    + (obj_ydraw_start_distance * obj_dmx)
                    + (obj_xdraw_start_distance * obj_dx);
                let mut obj_y = FixedPoint32::from(obj_height / 2)
                    + (obj_ydraw_start_distance * obj_dmy)
                    + (obj_xdraw_start_distance * obj_dy);

                let tile_data = &vram[0x10000..];
                let tile_stride: usize = if one_dimensional {
                    obj_width as usize / 8
                } else {
                    if attrs.0.palette256() {
                        16
                    } else {
                        32
                    }
                };

                let mut pflags = if $OBJWindowMode {
                    0
                } else {
                    let semi_transparent = attrs.0.mode() == ObjMode::SemiTransparent;
                    Pixel::layer_mask(Layer::OBJ)
                        | Pixel::priority_mask(attrs.2.priority())
                        | (if first_target { Pixel::FIRST_TARGET } else { 0 })
                        | (if second_target {
                            Pixel::SECOND_TARGET
                        } else {
                            0
                        })
                        | (if semi_transparent {
                            Pixel::SEMI_TRANSPARENT
                        } else {
                            0
                        })
                };

                if attrs.0.palette256() {
                    const BYTES_PER_TILE: usize = 64;
                    const BYTES_PER_LINE: usize = 8;

                    for obj_screen_draw in (obj_screen_left as usize)..=(obj_screen_right as usize)
                    {
                        // converting them to u32s and comparing like this will also handle the 'less than 0' case
                        if (obj_x.integer() as u32) < obj_width as u32
                            && (obj_y.integer() as u32) < obj_height as u32
                        {
                            let obj_x_i = apply_mosaic_cond(
                                attrs.0.mosaic(),
                                obj_x.integer() as u16,
                                mosaic_x,
                            ) as usize;
                            let obj_y_i = apply_mosaic_cond(
                                attrs.0.mosaic(),
                                obj_y.integer() as u16,
                                mosaic_y,
                            ) as usize;

                            let tile = (((attrs.2.first_tile_index() / 2) as usize)
                                + ((obj_y_i / 8) * tile_stride)
                                + (obj_x_i / 8))
                                & 0x3FF;
                            if !$BitmapMode || tile >= 512 {
                                let pixel_offset = (tile * BYTES_PER_TILE)
                                    + ((obj_y_i % 8) * BYTES_PER_LINE)
                                    + (obj_x_i % 8);
                                let palette_entry = tile_data[pixel_offset as usize] as usize;

                                if palette_entry != 0 {
                                    if $OBJWindowMode {
                                        pixels.windows.obj_window.set(obj_screen_draw);
                                    } else {
                                        pixels.push_obj_pixel(
                                            obj_screen_draw,
                                            Pixel(pflags | (palette_entry as u16)),
                                        );
                                    }
                                }
                            }
                        }

                        obj_x += obj_dx;
                        obj_y += obj_dy;
                    }
                } else {
                    const BYTES_PER_TILE: usize = 32;
                    const BYTES_PER_LINE: usize = 4;

                    pflags |= attrs.2.palette_number() << 4;

                    for obj_screen_draw in (obj_screen_left as usize)..=(obj_screen_right as usize)
                    {
                        // converting them to u32s and comparing like this will also handle the 'less than 0' case
                        if (obj_x.integer() as u32) < obj_width as u32
                            && (obj_y.integer() as u32) < obj_height as u32
                        {
                            let obj_x_i = apply_mosaic_cond(
                                attrs.0.mosaic(),
                                obj_x.integer() as u16,
                                mosaic_x,
                            ) as usize;
                            let obj_y_i = apply_mosaic_cond(
                                attrs.0.mosaic(),
                                obj_y.integer() as u16,
                                mosaic_y,
                            ) as usize;

                            let tile = ((attrs.2.first_tile_index() as usize)
                                + ((obj_y_i / 8) * tile_stride)
                                + (obj_x_i / 8))
                                & 0x3FF;
                            let pixel_offset = (tile * BYTES_PER_TILE)
                                + ((obj_y_i % 8) * BYTES_PER_LINE)
                                + (obj_x_i % 8) / 2;
                            let palette_entry =
                                (tile_data[pixel_offset as usize] >> ((obj_x_i % 2) << 2)) & 0xF;

                            if palette_entry != 0 {
                                if $OBJWindowMode {
                                    pixels.windows.obj_window.set(obj_screen_draw);
                                } else {
                                    pixels.push_obj_pixel(
                                        obj_screen_draw,
                                        Pixel(pflags | (palette_entry as u16)),
                                    );
                                }
                            }
                        }

                        obj_x += obj_dx;
                        obj_y += obj_dy;
                    }
                }
            }
        }
    };
}

define_obj_renderer!(render_objects_tm, OBJ_RENDER, TILEMAP);
define_obj_renderer!(render_objects_bm, OBJ_RENDER, BITMAP);
define_obj_renderer!(process_window_objects_tm, OBJ_WINDOW, TILEMAP);
define_obj_renderer!(process_window_objects_bm, OBJ_WINDOW, BITMAP);

bitfields!(ObjAttr0: u16 {
    y, set_y: u16 = [0, 7],
    affine, set_affine: bool = [8, 8],
    double_size, set_double_size: bool = [9, 9],
    disabled, set_disabled: bool = [9, 9],
    mode, set_mode: ObjMode = [10, 11],
    mosaic, set_mosaic: bool = [12, 12],
    palette256, set_palette256: bool = [13, 13],
    shape, set_shape: ObjShape = [14, 15],
});

bitfields!(ObjAttr1: u16 {
    x, set_x: u16 = [0, 8],
    affine_param_index, set_affine_param_index: u16 = [9, 13],
    flip_horizontal, set_flip_horizontal: bool = [12, 12],
    flip_vertical, set_flip_vertical: bool = [13, 13],
    size_select, set_size_select: u16 = [14, 15],
});

bitfields!(ObjAttr2: u16 {
    first_tile_index, set_first_tile_index: u16 = [0, 9],
    priority, set_priority: u16 = [10, 11],
    palette_number, set_palette_number: u16 = [12, 15],
});

#[derive(Clone, Copy)]
pub enum ObjShape {
    Square,
    Horizontal,
    Vertical,
    Prohibited,
}

impl ObjShape {
    pub fn size(self, size_select: u16) -> (u16, u16) {
        match (self, size_select) {
            (ObjShape::Square, 0) => (8, 8),
            (ObjShape::Square, 1) => (16, 16),
            (ObjShape::Square, 2) => (32, 32),
            (ObjShape::Square, 3) => (64, 64),

            (ObjShape::Horizontal, 0) => (16, 8),
            (ObjShape::Horizontal, 1) => (32, 8),
            (ObjShape::Horizontal, 2) => (32, 16),
            (ObjShape::Horizontal, 3) => (64, 32),

            (ObjShape::Vertical, 0) => (8, 16),
            (ObjShape::Vertical, 1) => (8, 32),
            (ObjShape::Vertical, 2) => (16, 32),
            (ObjShape::Vertical, 3) => (32, 64),

            _ => (8, 8),
        }
    }
}

impl crate::util::bitfields::FieldConvert<u16> for ObjShape {
    fn convert(self) -> u16 {
        match self {
            ObjShape::Square => 0,
            ObjShape::Horizontal => 1,
            ObjShape::Vertical => 2,
            ObjShape::Prohibited => 3,
        }
    }
}

impl crate::util::bitfields::FieldConvert<ObjShape> for u16 {
    fn convert(self) -> ObjShape {
        match self {
            0 => ObjShape::Square,
            1 => ObjShape::Horizontal,
            2 => ObjShape::Vertical,
            _ => ObjShape::Prohibited,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjMode {
    Normal,
    SemiTransparent,
    Window,
    Prohibited,
}

impl crate::util::bitfields::FieldConvert<u16> for ObjMode {
    fn convert(self) -> u16 {
        match self {
            ObjMode::Normal => 0,
            ObjMode::SemiTransparent => 1,
            ObjMode::Window => 2,
            ObjMode::Prohibited => 3,
        }
    }
}

impl crate::util::bitfields::FieldConvert<ObjMode> for u16 {
    fn convert(self) -> ObjMode {
        match self {
            0 => ObjMode::Normal,
            1 => ObjMode::SemiTransparent,
            2 => ObjMode::Window,
            _ => ObjMode::Prohibited,
        }
    }
}

pub struct ObjectPriority {
    pub enabled_objects: usize,
    pub visible_objects: usize,
    pub sorted_objects: [u16; 128],
}

impl ObjectPriority {
    pub fn sorted(oam: &OAM) -> ObjectPriority {
        macro_rules! mkobj {
            ($Index:expr, $Priority:expr) => {
                (($Priority as u16) << 8) | ($Index as u16)
            };
        }

        let mut priority_pos = [(0, 0); 6];
        let mut objects = [0u16; 128];

        let mut enabled_objects = 0;
        let mut visible_objects = 0;

        for index in 0..128 {
            let attr_index = index * 8;
            // NOTE compile doesn't seem to be able to remove the bounds check here :(
            let attr0_hi = unsafe { *oam.get_unchecked(attr_index + 1) };

            // Check if this object is disabled:
            if attr0_hi & 0x1 == 0 && (attr0_hi >> 1) & 0x1 != 0 {
                continue;
            }

            enabled_objects += 1;

            // NOTE compile doesn't seem to be able to remove the bounds check here :(
            let attr0_hi = unsafe { *oam.get_unchecked(attr_index + 1) };

            // Check if ths mode of this object is window mode:
            if (attr0_hi >> 2) & 0x3 == 2 {
                priority_pos[4].1 += 1;
                objects[index] |= mkobj!(index, 4);
                continue;
            }

            visible_objects += 1;

            // NOTE compile doesn't seem to be able to remove the bounds check here :(
            let attr2_hi = unsafe { *oam.get_unchecked(attr_index + 5) };

            let priority = (attr2_hi >> 2) & 0x3;
            priority_pos[priority as usize].1 += 1;
            objects[index] = mkobj!(index, priority);
        }

        // we only bother to sort if there are visible objects:
        if visible_objects > 0 {
            (&mut objects[0..enabled_objects]).sort_unstable_by(|lhs, rhs| {
                let prio_lhs = lhs >> 4;
                let prio_rhs = rhs >> 4;

                if prio_lhs == 4u16 || prio_rhs == 4u16 {
                    // For window objects we can just sort naturally because they will always end
                    // up at the end of the array:
                    std::cmp::Ord::cmp(lhs, rhs)
                } else {
                    // For visible objects we just sort them backwards
                    // because we want objects to be drawn from back to front:
                    std::cmp::Ord::cmp(rhs, lhs)
                }
            });
        }

        return ObjectPriority {
            enabled_objects: enabled_objects,
            visible_objects: visible_objects,
            sorted_objects: objects,
        };
    }

    pub fn window_objects(&self) -> &[u16] {
        let end = self.visible_objects + (self.enabled_objects - self.visible_objects);
        return &self.sorted_objects[self.visible_objects..end];
    }

    pub fn visible_objects(&self) -> &[u16] {
        return &self.sorted_objects[0..self.visible_objects];
    }
}
