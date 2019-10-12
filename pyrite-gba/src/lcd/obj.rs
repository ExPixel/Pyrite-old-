use pyrite_common::{bits, bits_b};
use super::Line;
use super::super::GbaMemory;
use super::super::memory::palette::Palette;
// use super::super::memory::read16_le;

pub fn draw_objects<F: FnMut(usize, u16, u8)>(line: u32, one_dimensional: bool, vram: &[u8], oam: &[u8], palette: &Palette, tile_data_start: u32, mut poke: F) {
    macro_rules! conditional_negate {
        ($Condition:expr, $Value:expr) => {{
            let value = $Value;
            let icondition = (if $Condition { 1 } else { 0 }) + (value & 0);// I add (value & 0) for type information
            (value ^ (!icondition).wrapping_add(1)).wrapping_add(icondition)
        }}
    }

    for obj_idx in 0..128 {
        let obj_all_attrs = read48_le(oam, obj_idx as usize * 8);
        let attr = ObjAttr::new(obj_all_attrs as u16, (obj_all_attrs >> 16) as u16, (obj_all_attrs >> 32) as u16);

        // check if the object is disabled or out of bounds
        let (top, bottom) = (attr.y as u32, attr.y.wrapping_add(attr.height) as u8 as u32);
        // the second condition correctly handles an object wrapping vertically around the screen.
        let in_bounds = (top <= line && bottom >= line) || (top > line && bottom >= line  && bottom < top);
        if attr.disabled || !in_bounds { continue }

        let obj_line = if attr.vertical_flip {
            bottom - line
        } else {
            attr.height as u32 - (bottom - line)
        };

        let (start_obj_x, end_obj_x) = if attr.x >= 240 {
            let obj_right = attr.x.wrapping_add(attr.width) & 0x1FF; // this is going to be the right pixel + 1
            if  obj_right >= 1 && obj_right <= 240 {
                (attr.width - obj_right, attr.width)
            } else {
                // object is out of bounds
                continue
            }
        } else {
            (0, std::cmp::min(attr.width, 240 - attr.x))
        };

        let tile_data = &vram[(tile_data_start as usize)..];

        let tile_stride = if one_dimensional {
            (attr.width / 8) as usize
        } else {
            32usize
        };

        let hflip_add = if attr.horizontal_flip { attr.width - 1 } else { 0 };

        if attr.pal256 {
            const BYTES_PER_TILE: usize = 64;
            const BYTES_PER_LINE: usize = 8;

            for obj_x in start_obj_x..end_obj_x {
                let screen_x = attr.x.wrapping_add(obj_x) & 0x1FF;
                // becomes (width - x - 1) if horizontal_flip is true, or just x if horizontal_flip
                // is false
                let obj_x = conditional_negate!(attr.horizontal_flip, obj_x).wrapping_add(hflip_add) as usize;
                let tile = (attr.tile_number as usize * BYTES_PER_TILE) + ((obj_line as usize / 8) * tile_stride) + (obj_x/8);
                let pixel_offset = (tile * BYTES_PER_TILE) + ((obj_line as usize % 8) * BYTES_PER_LINE) + (obj_x % 8);
                let palette_entry = tile_data[pixel_offset];
                let color = palette.get_obj256(palette_entry);

                // screen_x should always be in bounds [0, 240)
                poke(screen_x as usize, color, attr.priority);
            }
        } else {
            const BYTES_PER_TILE: usize = 32;
            const BYTES_PER_LINE: usize = 4;

            for obj_x in start_obj_x..end_obj_x {
                let screen_x = attr.x.wrapping_add(obj_x) & 0x1FF;
                let obj_x = conditional_negate!(attr.horizontal_flip, obj_x).wrapping_add(hflip_add) as usize;
                let tile = (attr.tile_number as usize * BYTES_PER_TILE) + ((obj_line as usize / 8) * tile_stride) + (obj_x/8);
                let pixel_offset = (tile * BYTES_PER_TILE) + ((obj_line as usize % 8) * BYTES_PER_LINE) + ((obj_x % 8) / 2);
                let palette_entry = if obj_x % 2 == 0 {
                    tile_data[pixel_offset] & 0xF
                } else {
                    tile_data[pixel_offset] >> 4
                };
                let color = palette.get_obj16(attr.palette_index, palette_entry);

                // screen_x should always be in bounds [0, 240)
                poke(screen_x as usize, color, attr.priority);
            }
        }
    }
}

#[inline]
pub fn get_obj_pixel_4bpp_one_dimensional(x: usize, y: usize, width: usize, height: usize, first_tile: usize, tile_data: &[u8], palette_index: u8, palette: &Palette) -> u16 {
    let tile = (first_tile * 32) + ((y / 8) * (width / 8)) + (x/8);
    let pixel_offset = (tile * 32) + ((y % 8) * 4) + ((x % 8) / 2);
    let palette_entry = if x % 2 == 0 {
        tile_data[pixel_offset] & 0xF
    } else {
        tile_data[pixel_offset] >> 4
    };
    palette.get_obj16(palette_index, palette_entry)
}

#[inline]
pub fn get_obj_pixel_8bpp_one_dimensional(x: usize, y: usize, width: usize, height: usize, first_tile: usize, tile_data: &[u8], palette: &Palette) -> u16 {
    let tile = (first_tile * 64) + ((y / 8) * (width / 8)) + (x/8);
    let pixel_offset = (tile * 64) + ((y % 8) * 8) + (x % 8);
    let palette_entry = if x % 2 == 0 {
        tile_data[pixel_offset] & 0xF
    } else {
        tile_data[pixel_offset] >> 4
    };
    palette.get_obj256(palette_entry)
}

#[inline]
pub fn get_obj_pixel_4bpp_two_dimensional(x: usize, y: usize, width: usize, height: usize, first_tile: usize, tile_data: &[u8], palette_index: u8, palette: &Palette) -> u16 {
    let tile = (first_tile * 32) + ((y / 8) * 32) + (x/8);
    let pixel_offset = (tile * 32) + ((y % 8) * 4) + ((x % 8) / 2);
    let palette_entry = if x % 2 == 0 {
        tile_data[pixel_offset] & 0xF
    } else {
        tile_data[pixel_offset] >> 4
    };
    palette.get_obj16(palette_index, palette_entry)
}

#[inline]
pub fn get_obj_pixel_8bpp_two_dimensional(x: usize, y: usize, widht: usize, height: usize, first_tile: usize, tile_data: &[u8], palette: &Palette) -> u16 {
    let tile = (first_tile * 64) + ((y / 8) * 32) + (x/8);
    let pixel_offset = (tile * 64) + ((y % 8) * 8) + (x % 8);
    let palette_entry = if x % 2 == 0 {
        tile_data[pixel_offset] & 0xF
    } else {
        tile_data[pixel_offset] >> 4
    };
    palette.get_obj256(palette_entry)
}

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
    pub y: u16,
    pub rot_scal: bool,
    pub double_size: bool,
    pub disabled: bool,
    pub mode: ObjMode,
    pub mosaic: bool,
    pub pal256: bool,
    pub shape: ObjShape,

    // attr 1
    pub x: u16,
    pub rot_scal_param: u8,
    pub horizontal_flip: bool,
    pub vertical_flip: bool,
    pub width: u16,
    pub height: u16,

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
        ObjAttr {
            // sign extend the y value to get it into range [-128, 127]
            y: bits!(attr0, 0, 7),
            rot_scal: rot_scal,
            double_size: if rot_scal { bits_b!(attr0, 9) } else { false },
            disabled: if !rot_scal { bits_b!(attr0, 9) } else { false },
            mode: ObjMode::from(bits!(attr0, 10, 11) as u8),
            mosaic: bits_b!(attr0, 12),
            pal256: bits_b!(attr0, 13),
            shape: shape,

            // sign extend the x value to get it into range [-256, 255]
            x: bits!(attr1, 0, 8),
            rot_scal_param: if rot_scal { bits!(attr1, 9, 13) as u8 } else { 0 },
            horizontal_flip: if !rot_scal { bits_b!(attr1, 12) } else { false },
            vertical_flip: if !rot_scal { bits_b!(attr1, 13) } else { false },
            width: width,
            height: height,

            tile_number: bits!(attr2, 0, 9),
            priority: bits!(attr2, 10, 11) as u8,
            palette_index: bits!(attr2, 12, 15) as u8,
        }
    }
}

#[derive(Clone, Copy)]
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
