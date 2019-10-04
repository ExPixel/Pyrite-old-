use std::ffi::CStr;
use crate::flags::*;

use crate::sys;
pub use crate::sys::ImVec2;
pub use crate::sys::ImVec4;

pub fn get_version() -> &'static CStr {
    unsafe {
        CStr::from_ptr(sys::igGetVersion())
    }
}

pub fn debug_version_and_data_layout(version: &CStr, sz_io: usize, sz_style: usize, sz_vec2: usize, sz_vec4: usize, sz_vert: usize, sz_draw_idx: usize) -> bool {
    unsafe {
        sys::igDebugCheckVersionAndDataLayout(version.as_ptr(), sz_io, sz_style, sz_vec2, sz_vec4, sz_vert, sz_draw_idx)
    }
}

pub fn get_io() -> Option<&'static mut sys::ImGuiIO> {
    unsafe {
        sys::igGetIO().as_mut()
    }
}

pub fn create_context(shared_font_atlas: &mut sys::ImFontAtlas) -> Option<&'static mut sys::ImGuiContext> {
    unsafe {
        sys::igCreateContext(shared_font_atlas).as_mut()
    }
}

pub fn begin(name: &CStr, p_open: &mut bool, flags: WindowFlags) -> bool {
    unsafe {
        sys::igBegin(name.as_ptr(), p_open, flags.bits() as _)
    }
}

pub fn get_mouse_cursor() -> MouseCursor {
    unsafe {
        MouseCursor::from_bits(sys::igGetMouseCursor()).expect("invalid mouse cursor value")
    }
}

pub fn end() {
    unsafe {
        sys::igEnd()
    }
}

/////////////////////////////////////////////
//
//          Struct Implementations
//
////////////////////////////////////////////
macro_rules! create_owned_impl {
    ($OwnedType:ident, $TargetType:ty, $Constructor:path, $Destructor:path) => {
        pub struct $OwnedType(*mut $TargetType);

        impl $OwnedType {
            pub fn new() -> $OwnedType {
                $OwnedType(unsafe { $Constructor() } )
            }

            pub unsafe fn leak(self) -> *mut $TargetType {
                let ptr = self.0;
                std::mem::forget(self);
                return ptr;
            }
        }

        impl Drop for $OwnedType {
            fn drop(&mut self) {
                unsafe {
                    $Destructor(self.0);
                }
                self.0 = std::ptr::null_mut();
            }
        }

        impl std::ops::Deref for $OwnedType {
            type Target = $TargetType;
            fn deref(&self) -> &$TargetType {
                unsafe {
                    self.0.as_ref().unwrap()
                }
            }
        }

        impl std::ops::DerefMut for $OwnedType {
            fn deref_mut(&mut self) -> &mut $TargetType {
                unsafe {
                    self.0.as_mut().unwrap()
                }
            }
        }
    }
}

impl sys::ImGuiIO {
    pub fn add_input_characters_utf8(&mut self, s: &CStr) {
        unsafe {
            sys::ImGuiIO_AddInputCharactersUTF8(self, s.as_ptr())
        }
    }
}
create_owned_impl!(IO, sys::ImGuiIO, sys::ImGuiIO_ImGuiIO, sys::ImGuiIO_destroy);

/////////////////////////////////////////////
//
//             UTILITY FUNCTIONS
//
////////////////////////////////////////////

pub const fn vec2(x: f32, y: f32) -> ImVec2 {
    ImVec2 { x, y }
}

pub const fn vec4(x: f32, y: f32, z: f32, w: f32) -> ImVec4 {
    ImVec4 { x, y, z, w }
}

#[cfg(test)]
mod tests {
    #[test]
    fn check_version_and_data_layout() {
        use std::mem::size_of;
        assert_eq!(super::debug_version_and_data_layout(
            super::sys::get_version(),
            size_of::<super::sys::ImGuiIO>(),
            size_of::<super::sys::ImGuiStyle>(),
            size_of::<super::sys::ImVec2>(),
            size_of::<super::sys::ImVec4>(),
            size_of::<super::sys::ImDrawVert>(),
            size_of::<super::sys::ImDrawIdx>()
        ), true);
    }
}
