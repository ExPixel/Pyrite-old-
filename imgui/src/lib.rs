#[macro_export]
macro_rules! str {
    ($RustStr:expr) => {
        unsafe {
            CStr::from_bytes_with_nul_unchecked(concat!($RustStr, '\0').as_bytes())
        }
    }
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
