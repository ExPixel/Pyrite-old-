#[macro_export]
macro_rules! str {
    ($RustStr:expr) => {
        #[allow(unused_unsafe)]
        unsafe {
            $crate::imstr::ImStr::from_bytes_with_nul_unchecked(concat!($RustStr, '\0').as_bytes())
        }
    };
}

#[macro_export]
macro_rules! str_buf {
    ($Buffer:expr, $RustStr:expr) => {
        {
            use std::io::Write;

            let mut cursor = std::io::Cursor::new($Buffer);
            write!(&mut cursor, concat!($RustStr, '\0')).expect("error while formatting imgui string into buffer");

            #[allow(unused_unsafe)]
            unsafe {
                $crate::imstr::ImStr::from_bytes_with_nul_unchecked(&cursor.into_inner())
            }
        }
    };

    ($Buffer:expr, $RustStr:expr, $($Arg:expr),* $(,)?) => {
        {
            use std::io::Write;

            let mut cursor = std::io::Cursor::new($Buffer);
            write!(cursor, concat!($RustStr, '\0'), $($Arg,)*).expect("error while formatting imgui string into buffer");

            #[allow(unused_unsafe)]
            unsafe {
                $crate::imstr::ImStr::from_bytes_with_nul_unchecked(&cursor.into_inner())
            }
        }
    };
}

#[macro_export]
macro_rules! str_gbuf {
    ($RustStr:expr) => {
        $crate::str_buf!($crate::api::global_fmt_buffer(), $RustStr)
    };

    ($RustStr:expr, $($Arg:expr),* $(,)?) => {
        $crate::str_buf!($crate::api::global_fmt_buffer(), $RustStr, $($Arg,)*)
    };
}

#[allow(non_upper_case_globals)]
mod flags;
pub mod api;
pub mod impls;
pub mod imstr;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
mod sys;

pub use api::*;
pub use flags::*;

#[macro_export]
macro_rules! check_version {
    () => ({
        use std::mem::size_of;
        $crate::api::debug_version_and_data_layout(
            $crate::api::get_version(),
            size_of::<$crate::sys::ImGuiIO>(),
            size_of::<$crate::sys::ImGuiStyle>(),
            size_of::<$crate::sys::ImVec2>(),
            size_of::<$crate::sys::ImVec4>(),
            size_of::<$crate::sys::ImDrawVert>(),
            size_of::<$crate::sys::ImDrawIdx>()
        )
    })
}
