macro_rules! read_bytes_le {
    ($Src:expr, $Type: ty, $Size:expr) => {{
        let src = $Src;

        assert!($Size <= std::mem::size_of::<$Type>());
        assert!($Size <= src.len());

        let mut data: $Type = 0;
        unsafe {
            std::ptr::copy_nonoverlapping(src.as_ptr(), &mut data as *mut $Type as *mut u8, $Size);
        }

        #[cfg(target_endian = "big")]
        {
            data.swap_bytes()
        }

        #[cfg(target_endian = "little")]
        {
            data
        }
    }};
}

macro_rules! read_bytes_le_no_bounds_check {
    ($Src:expr, $Type: ty, $Size:expr) => {{
        let src = $Src;

        let mut data: $Type = 0;

        std::ptr::copy_nonoverlapping(src as *const _, &mut data as *mut $Type as *mut u8, $Size);

        #[cfg(target_endian = "big")]
        {
            data.swap_bytes()
        }

        #[cfg(target_endian = "little")]
        {
            data
        }
    }};
}

macro_rules! write_bytes_le {
    ($Dst:expr, $Data:expr, $Type:ty, $Size:expr) => {{
        let dst = $Dst;

        assert!($Size <= std::mem::size_of::<$Type>());
        assert!($Size <= dst.len());

        let data: $Type = if cfg!(target_endian = "little") {
            $Data
        } else {
            $Data.swap_bytes()
        };

        unsafe {
            std::ptr::copy_nonoverlapping(
                &data as *const $Type as *const u8,
                dst.as_mut_ptr(),
                $Size,
            );
        }
    }};
}

macro_rules! write_bytes_le_no_bounds_check {
    ($Dst:expr, $Data:expr, $Type:ty, $Size:expr) => {{
        let dst = $Dst;

        let data: $Type = if cfg!(target_endian = "little") {
            $Data
        } else {
            $Data.swap_bytes()
        };

        std::ptr::copy_nonoverlapping(&data as *const $Type as *const u8, dst.as_mut_ptr(), $Size);
    }};
}

pub fn memset<T: Copy>(dest: &mut [T], value: T) {
    for i in dest.iter_mut() {
        *i = value;
    }
}

#[inline]
pub unsafe fn read_u64_unchecked(mem: &[u8], offset: usize) -> u64 {
    read_bytes_le_no_bounds_check!(mem.get_unchecked(offset), u64, 8)
}

#[inline]
pub fn read_u32(mem: &[u8], offset: usize) -> u32 {
    read_bytes_le!(&mem[offset..], u32, 4)
}

#[inline]
pub unsafe fn read_u32_unchecked(mem: &[u8], offset: usize) -> u32 {
    read_bytes_le_no_bounds_check!(mem.get_unchecked(offset), u32, 4)
}

#[inline]
pub fn read_u16(mem: &[u8], offset: usize) -> u16 {
    read_bytes_le!(&mem[offset..], u16, 2)
}

#[inline]
pub unsafe fn read_u16_unchecked(mem: &[u8], offset: usize) -> u16 {
    read_bytes_le_no_bounds_check!(mem.get_unchecked(offset), u16, 2)
}

// pub fn write_u64(mem: &mut [u8], offset: usize, value: u64) {
//     write_bytes_le!(&mut mem[offset..], value, u64, 8);
// }

pub unsafe fn write_u64_unchecked(mem: &mut [u8], offset: usize, value: u64) {
    write_bytes_le_no_bounds_check!(&mut mem[offset..], value, u64, 8);
}

#[inline]
pub fn write_u32(mem: &mut [u8], offset: usize, value: u32) {
    write_bytes_le!(&mut mem[offset..], value, u32, 4);
}

#[inline]
pub fn write_u16(mem: &mut [u8], offset: usize, value: u16) {
    write_bytes_le!(&mut mem[offset..], value, u16, 2);
}
