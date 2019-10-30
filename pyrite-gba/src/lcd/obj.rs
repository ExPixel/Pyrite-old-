use pyrite_common::{bits, bits_b};
use super::super::memory::GbaMemory;
use super::RawLine;
use super::blending::{ apply_mosaic_cond, poke_obj_pixel, Windows, SpecialEffects };
use super::super::memory::read16_le;
use crate::util::fixedpoint::{ FixedPoint32, FixedPoint16 };

pub fn draw_objects(line: u32, memory: &GbaMemory, tile_data_start: u32, raw_pixels: &mut RawLine, effects: SpecialEffects, windows: Windows) {
    let vram    = &memory.mem_vram;
    let oam     = &memory.mem_oam;
    let ioregs  = &memory.ioregs;
    let palette = &memory.palette;
    let one_dimensional = ioregs.dispcnt.obj_one_dimensional();
    // let debug_on = ioregs.keyinput.inner & (1 << 2) == 0; // #TODO remove this debug code.
 
    let mosaic_x = ioregs.mosaic.obj_h_size() as u32 + 1;
    let mosaic_y = ioregs.mosaic.obj_v_size() as u32 + 1;

    let mut cycles_available = if ioregs.dispcnt.hblank_interval_free() {
        954
    } else {
        1210
    };

    'obj_main_loop: for obj_idx in 0..128 {
        let obj_all_attrs = read48_le(oam, obj_idx as usize * 8);
        let attr = ObjAttr::new(obj_all_attrs as u16, (obj_all_attrs >> 16) as u16, (obj_all_attrs >> 32) as u16);

        let (mut obj_screen_left, obj_screen_top, mut obj_screen_right, obj_screen_bottom) = attr.bounds();
        let in_bounds_horizontal = obj_screen_left < 240 || obj_screen_right < 240;
        let in_bounds_vertical = obj_screen_top <= obj_screen_bottom && obj_screen_top <= line && obj_screen_bottom >= line;
        let in_bounds_vertical_wrapped = obj_screen_top > obj_screen_bottom && (obj_screen_top <= line || obj_screen_bottom >= line);

        if attr.disabled || !in_bounds_horizontal || (!in_bounds_vertical && !in_bounds_vertical_wrapped) {
            continue
        }

        let obj_screen_width = attr.display_width();

        // the start of end horizontal pixels of the object that are going to be draw:
        let (obj_xdraw_start, _obj_xdraw_end) = if obj_screen_left < obj_screen_right {
            (0, if obj_screen_right >= 240 {
                obj_screen_right = 239;
                240 - obj_screen_left - 1
            } else {
                obj_screen_width - 1
            })
        } else {
            // we have wrapped here so we need to start drawing farther to the right
            // of the object, but there will always be enough space on screen to draw the
            // object to the end.
            obj_screen_left = 0;
            (obj_screen_width - obj_screen_right - 1, obj_screen_width - 1)
        };

        let pixels_drawn = obj_screen_right - obj_screen_left + 1;
        if attr.rot_scal {
            // affine objects require 10 cycles to start
            if cycles_available > 10 {
                cycles_available -= 10;

                if (pixels_drawn * 2) > cycles_available {
                    obj_screen_right = obj_screen_left + (cycles_available / 2) - 1;
                    cycles_available = 0;
                } else {
                    cycles_available -= pixels_drawn * 2;
                }
            } else {
                break 'obj_main_loop;
            }
        } else {
            if pixels_drawn > cycles_available {
                obj_screen_right = obj_screen_left + cycles_available - 1;
                cycles_available = 0;
            } else {
                cycles_available -= pixels_drawn;
            }
        }

        let obj_dx; let obj_dmx;
        let obj_dy; let obj_dmy;

        let obj_origin_x = FixedPoint32::from( attr.display_width() / 2);
        let obj_origin_y = FixedPoint32::from(attr.display_height() / 2);

        let obj_xdraw_start = FixedPoint32::from(obj_xdraw_start);
        let obj_ydraw_start = if line > obj_screen_bottom {
            FixedPoint32::from(line - obj_screen_top)
        } else {
            FixedPoint32::from(attr.display_height() - (obj_screen_bottom - line) - 1)
        };

        let mut obj_xdraw_start_distance = obj_xdraw_start - obj_origin_x;
        let mut obj_ydraw_start_distance = obj_ydraw_start - obj_origin_y;

        if attr.rot_scal {
            let params_idx = attr.rot_scal_param as usize;
            obj_dx  = FixedPoint32::from(FixedPoint16::wrap((read16_le(oam, 0x06 + (params_idx * 32))) as i16));
            obj_dmx = FixedPoint32::from(FixedPoint16::wrap((read16_le(oam, 0x0E + (params_idx * 32))) as i16));
            obj_dy  = FixedPoint32::from(FixedPoint16::wrap((read16_le(oam, 0x16 + (params_idx * 32))) as i16));
            obj_dmy = FixedPoint32::from(FixedPoint16::wrap((read16_le(oam, 0x1E + (params_idx * 32))) as i16));
        } else {
            obj_dy  = FixedPoint32::from(0u32);
            obj_dmx = FixedPoint32::from(0u32);
            obj_dmy = FixedPoint32::from(1u32);

            if attr.horizontal_flip {
                obj_dx = FixedPoint32::from(-1i32);

                // @NOTE add 1 so that we start on the other side of the center line...if that makes sense :|
                obj_xdraw_start_distance += FixedPoint32::wrap(0x100); 
            } else {
                obj_dx  = FixedPoint32::from(1u32);
            }

            if attr.vertical_flip {
                obj_ydraw_start_distance = -obj_ydraw_start_distance;
            }
        }

        // Down here we use the real width and height for the origin instead of the double sized
        // because I randomly wrote it and it works. Maybe one day I'll actually do the math and
        // come up with an exact reason as to why. For now I just had a feeling and I was right.
        let mut obj_x = FixedPoint32::from( attr.width / 2) + (obj_ydraw_start_distance * obj_dmx) + (obj_xdraw_start_distance * obj_dx);
        let mut obj_y = FixedPoint32::from(attr.height / 2) + (obj_ydraw_start_distance * obj_dmy) + (obj_xdraw_start_distance * obj_dy);

        let tile_data = &vram[(tile_data_start as usize)..];
        let tile_stride = if one_dimensional {
            attr.width / 8
        } else {
            32
        };

        if attr.pal256 {
            const BYTES_PER_TILE: u32 = 32;
            const BYTES_PER_LINE: u32 = 4;

            for obj_screen_draw in (obj_screen_left as usize)..=(obj_screen_right as usize) {
                // converting them to u32s and comparing like this will also handle the 'less than 0' case
                if (obj_x.integer() as u32) < attr.width && (obj_y.integer() as u32) < attr.height {
                    let obj_x_i = apply_mosaic_cond(attr.mosaic, obj_x.integer() as u32, mosaic_x);
                    let obj_y_i = apply_mosaic_cond(attr.mosaic, obj_y.integer() as u32, mosaic_y);

                    let tile = ((attr.tile_number as u32) + ((obj_y_i / 8) * tile_stride) + (obj_x_i/8)) & 0x3FF;
                    let pixel_offset = (tile * BYTES_PER_TILE) + ((obj_y_i % 8) * BYTES_PER_LINE) + (obj_x_i % 8);
                    let palette_entry = tile_data[pixel_offset as usize];
                    let color = palette.get_obj256(palette_entry);
                    poke_obj_pixel(line, obj_screen_draw, color, attr.priority, attr.mode, raw_pixels, effects, windows);
                }

                obj_x += obj_dx;
                obj_y += obj_dy;
            }
        } else {
            const BYTES_PER_TILE: u32 = 32;
            const BYTES_PER_LINE: u32 = 4;

            for obj_screen_draw in (obj_screen_left as usize)..=(obj_screen_right as usize) {
                // converting them to u32s and comparing like this will also handle the 'less than 0' case
                if (obj_x.integer() as u32) < attr.width && (obj_y.integer() as u32) < attr.height {
                    let obj_x_i = apply_mosaic_cond(attr.mosaic, obj_x.integer() as u32, mosaic_x);
                    let obj_y_i = apply_mosaic_cond(attr.mosaic, obj_y.integer() as u32, mosaic_y);

                    let tile = ((attr.tile_number as u32) + ((obj_y_i / 8) * tile_stride) + (obj_x_i/8)) & 0x3FF;
                    let pixel_offset = (tile * BYTES_PER_TILE) + ((obj_y_i % 8) * BYTES_PER_LINE) + (obj_x_i % 8)/2;
                    let palette_entry = (tile_data[pixel_offset as usize] >> ((obj_x_i % 2) << 2)) & 0xF;
                    let color = palette.get_obj16(attr.palette_index, palette_entry);
                    poke_obj_pixel(line, obj_screen_draw, color, attr.priority, attr.mode, raw_pixels, effects, windows);
                }

                obj_x += obj_dx;
                obj_y += obj_dy;
            }
        }

        if cycles_available == 0 {
            break 'obj_main_loop;
        }
    }
}

// #[inline(always)]
// pub fn conditional_negate(condition: bool, value: u32) -> u32 {
//     let icondition = condition as u32;
//     (value ^ (!icondition).wrapping_add(1)).wrapping_add(icondition)
// }

/// Reads a u32 from a byte array in little endian byte order.
#[inline]
pub fn read48_le(mem: &[u8], offset: usize) -> u64 {
    assert!(mem.len() > offset + 5, "48bit read out of range (offset: {}, len: {})", offset, mem.len());
    (mem[offset] as u64) |
        ((mem[offset + 1] as u64) <<  8) |
        ((mem[offset + 2] as u64) << 16) |
        ((mem[offset + 3] as u64) << 24) |
        ((mem[offset + 4] as u64) << 32) |
        ((mem[offset + 5] as u64) << 40)
}

/// OBJ Attributes
/// There are 128 entries in OAM for each OBJ0-OBJ127. Each entry consists of 6 bytes (three 16bit Attributes). Attributes for OBJ0 are located at 07000000, for OBJ1 at 07000008, OBJ2 at 07000010, and so on.
///
/// As you can see, there are blank spaces at 07000006, 0700000E, 07000016, etc. - these 16bit values are used for OBJ Rotation/Scaling (as described in the next chapter) - they are not directly related to the separate OBJs.
///
/// OBJ Attribute 0 (R/W)
///
///   Bit   Expl.
///   0-7   Y-Coordinate           (0-255)
///   8     Rotation/Scaling Flag  (0=Off, 1=On)
///   When Rotation/Scaling used (Attribute 0, bit 8 set):
///     9     Double-Size Flag     (0=Normal, 1=Double)
///   When Rotation/Scaling not used (Attribute 0, bit 8 cleared):
///     9     OBJ Disable          (0=Normal, 1=Not displayed)
///   10-11 OBJ Mode  (0=Normal, 1=Semi-Transparent, 2=OBJ Window, 3=Prohibited)
///   12    OBJ Mosaic             (0=Off, 1=On)
///   13    Colors/Palettes        (0=16/16, 1=256/1)
///   14-15 OBJ Shape              (0=Square,1=Horizontal,2=Vertical,3=Prohibited)
///
/// Caution: A very large OBJ (of 128 pixels vertically, ie. a 64 pixels OBJ in a Double Size area) located at Y>128 will be treated as at Y>-128, the OBJ is then displayed parts offscreen at the TOP of the display, it is then NOT displayed at the bottom.
///
/// OBJ Attribute 1 (R/W)
///
///   Bit   Expl.
///   0-8   X-Coordinate           (0-511)
///   When Rotation/Scaling used (Attribute 0, bit 8 set):
///     9-13  Rotation/Scaling Parameter Selection (0-31)
///           (Selects one of the 32 Rotation/Scaling Parameters that
///           can be defined in OAM, for details read next chapter.)
///   When Rotation/Scaling not used (Attribute 0, bit 8 cleared):
///     9-11  Not used
///     12    Horizontal Flip      (0=Normal, 1=Mirrored)
///     13    Vertical Flip        (0=Normal, 1=Mirrored)
///   14-15 OBJ Size               (0..3, depends on OBJ Shape, see Attr 0)
///           Size  Square   Horizontal  Vertical
///           0     8x8      16x8        8x16
///           1     16x16    32x8        8x32
///           2     32x32    32x16       16x32
///           3     64x64    64x32       32x64
///
///
/// OBJ Attribute 2 (R/W)
///
///   Bit   Expl.
///   0-9   Character Name          (0-1023=Tile Number)
///   10-11 Priority relative to BG (0-3; 0=Highest)
///   12-15 Palette Number   (0-15) (Not used in 256 color/1 palette mode)
pub struct ObjAttr {
    // attr 0
    pub y: u32,
    pub rot_scal: bool,
    pub double_size: bool,
    pub disabled: bool,
    pub mode: ObjMode,
    pub mosaic: bool,
    pub pal256: bool,
    pub shape: ObjShape,

    // attr 1
    pub x: u32,
    pub rot_scal_param: u8,
    pub horizontal_flip: bool,
    pub vertical_flip: bool,
    pub width:  u32,
    pub height: u32,

    // attr 2
    pub tile_number: u16,
    pub priority: u8,
    pub palette_index: u8,
}

impl ObjAttr {
    pub fn new(attr0: u16, attr1: u16, attr2: u16) -> ObjAttr {
        let rot_scal = bits_b!(attr0, 8);
        let shape = ObjShape::from(bits!(attr0, 14, 15) as u8);
        let (width, height) = obj_size(shape, bits!(attr1, 14, 15));
        let double_size = if rot_scal { bits_b!(attr0, 9) } else { false };

        ObjAttr {
            // sign extend the y value to get it into range [-128, 127]
            y: bits!(attr0, 0, 7) as u32,
            rot_scal: rot_scal,
            double_size: double_size,
            disabled: if !rot_scal { bits_b!(attr0, 9) } else { false },
            mode: ObjMode::from(bits!(attr0, 10, 11) as u8),
            mosaic: bits_b!(attr0, 12),
            pal256: bits_b!(attr0, 13),
            shape: shape,

            // sign extend the x value to get it into range [-256, 255]
            x: bits!(attr1, 0, 8) as u32,
            rot_scal_param: if rot_scal { bits!(attr1, 9, 13) as u8 } else { 0 },
            horizontal_flip: if !rot_scal { bits_b!(attr1, 12) } else { false },
            vertical_flip: if !rot_scal { bits_b!(attr1, 13) } else { false },
            width:  width as u32,
            height: height as u32,

            tile_number: bits!(attr2, 0, 9),
            priority: bits!(attr2, 10, 11) as u8,
            palette_index: bits!(attr2, 12, 15) as u8,
        }
    }

    /// Returns the bounds of the object in the format: (left, top, right, bottom)
    #[inline]
    pub fn bounds(&self) -> (u32, u32, u32, u32) {
        let right   = (self.x +  self.display_width() - 1) % 512;
        let bottom  = (self.y + self.display_height() - 1) % 256;
        return (self.x, self.y, right, bottom)
    }

    #[inline]
    pub fn display_width(&self) -> u32 {
        if self.double_size {
            self.width * 2
        } else {
            self.width
        }
    }

    pub fn display_height(&self) -> u32 {
        if self.double_size {
            self.height * 2
        } else {
            self.height
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjMode {
    Normal,
    SemiTransparent,
    OBJWindow,
    Prohibited,
}

impl From<u8> for ObjMode {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Normal,
            1 => Self::SemiTransparent,
            2 => Self::OBJWindow,
            _ => Self::Prohibited,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum ObjShape {
    Square,
    Horizontal,
    Vertical,
    Prohibited,
}

impl From<u8> for ObjShape {
    fn from(val: u8) -> ObjShape {
        match val {
            0 => Self::Square,
            1 => Self::Horizontal,
            2 => Self::Vertical,
            _ => Self::Prohibited,
        }
    }
}

fn obj_size(shape: ObjShape, size_val: u16) -> (u16, u16) {
    match (shape, size_val) {
        (ObjShape::Square, 0) => ( 8,  8),
        (ObjShape::Square, 1) => (16, 16),
        (ObjShape::Square, 2) => (32, 32),
        (ObjShape::Square, 3) => (64, 64),

        (ObjShape::Horizontal, 0) => (16,  8),
        (ObjShape::Horizontal, 1) => (32,  8),
        (ObjShape::Horizontal, 2) => (32, 16),
        (ObjShape::Horizontal, 3) => (64, 32),

        (ObjShape::Vertical, 0) => ( 8, 16),
        (ObjShape::Vertical, 1) => ( 8, 32),
        (ObjShape::Vertical, 2) => (16, 32),
        (ObjShape::Vertical, 3) => (32, 64),
        
        _ => (8, 8),
    }
}
