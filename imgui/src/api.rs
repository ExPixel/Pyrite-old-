use crate::imstr::{ImStr};
use crate::flags::*;
use crate::sys;

pub use crate::sys::{
    ImVec2,
    ImVec4,
    ImFontAtlas,
    ImGuiIO,
    ImGuiContext,
    ImDrawData,
    ImTextureID,
};

pub fn get_version() -> &'static ImStr {
    unsafe {
        ImStr::from_ptr(sys::igGetVersion())
    }
}

pub fn debug_version_and_data_layout(version: &ImStr, sz_io: usize, sz_style: usize, sz_vec2: usize, sz_vec4: usize, sz_vert: usize, sz_draw_idx: usize) -> bool {
    unsafe {
        sys::igDebugCheckVersionAndDataLayout(version.as_ptr(), sz_io, sz_style, sz_vec2, sz_vec4, sz_vert, sz_draw_idx)
    }
}

pub fn get_io() -> Option<&'static mut sys::ImGuiIO> {
    unsafe {
        sys::igGetIO().as_mut()
    }
}

pub fn create_context(shared_font_atlas: Option<&mut sys::ImFontAtlas>) -> Option<&'static mut sys::ImGuiContext> {
    unsafe {
        sys::igCreateContext(opt_mut_ptr(shared_font_atlas)).as_mut()
    }
}

pub fn destroy_context(context: Option<&mut sys::ImGuiContext>) {
    unsafe {
        sys::igDestroyContext(opt_mut_ptr(context))
    }
}

pub fn get_draw_data() -> Option<&'static mut sys::ImDrawData> {
    unsafe {
        sys::igGetDrawData().as_mut()
    }
}

pub fn style_colors_dark(dst: Option<&mut sys::ImGuiStyle>) {
    unsafe {
        sys::igStyleColorsDark(opt_mut_ptr(dst))
    }
}

pub fn style_colors_classic(dst: Option<&mut sys::ImGuiStyle>) {
    unsafe {
        sys::igStyleColorsClassic(opt_mut_ptr(dst))
    }
}

pub fn style_colors_light(dst: Option<&mut sys::ImGuiStyle>) {
    unsafe {
        sys::igStyleColorsLight(opt_mut_ptr(dst))
    }
}

pub fn new_frame() {
    unsafe {
        sys::igNewFrame()
    }
}

pub fn render() {
    unsafe {
        sys::igRender()
    }
}

pub fn begin(name: &ImStr, p_open: &mut bool, flags: WindowFlags) -> bool {
    unsafe {
        sys::igBegin(name.as_ptr(), p_open, flags.bits() as _)
    }
}

pub fn end() {
    unsafe {
        sys::igEnd()
    }
}

pub fn get_mouse_cursor() -> MouseCursor {
    unsafe {
        MouseCursor::from_bits(sys::igGetMouseCursor()).expect("invalid mouse cursor value")
    }
}

pub fn image(user_texture_id: sys::ImTextureID, size: ImVec2, uv0: Option<ImVec2>, uv1: Option<ImVec2>, tint_col: Option<ImVec4>, border_col: Option<ImVec4>) {
    unsafe {
        sys::igImage(user_texture_id, size,
            uv0.unwrap_or(vec2(0.0, 0.0)),
            uv1.unwrap_or(vec2(1.0, 1.0)),
            tint_col.unwrap_or(vec4(1.0, 1.0, 1.0, 1.0)),
            border_col.unwrap_or(vec4(0.0, 0.0, 0.0, 0.0)))
    }
}

pub fn image_with_size(user_texture_id: sys::ImTextureID, size: ImVec2) {
    image(user_texture_id, size, None, None, None, None)
}

pub fn get_window_content_region_max() -> ImVec2 {
    unsafe {
        sys::igGetContentRegionAvail_nonUDT2().into()
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
    pub fn add_input_characters_utf8(&mut self, s: &ImStr) {
        unsafe {
            sys::ImGuiIO_AddInputCharactersUTF8(self, s.as_ptr())
        }
    }
}
create_owned_impl!(IO, sys::ImGuiIO, sys::ImGuiIO_ImGuiIO, sys::ImGuiIO_destroy);

impl sys::ImFontAtlas {
    pub fn get_tex_data_as_rgba32(&mut self, out_pixels: &mut *mut u8, out_width: &mut i32, out_height: &mut i32, out_bytes_per_pixel: Option<&mut i32>) {
        unsafe {
            sys::ImFontAtlas_GetTexDataAsRGBA32(self, out_pixels as _, out_width as _, out_height as _, opt_mut_ptr(out_bytes_per_pixel))
        }
    }
}

impl sys::ImDrawData {
    pub fn scale_clip_rects(&mut self, fb_scale: ImVec2) {
        unsafe {
            sys::ImDrawData_ScaleClipRects(self, fb_scale)
        }
    }
}

impl std::convert::From<sys::ImVec2_Simple> for ImVec2 {
    #[inline(always)]
    fn from(simple: sys::ImVec2_Simple) -> Self {
        ImVec2 { x: simple.x, y: simple.y }
    }
}

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

// fn opt_ptr<T>(opt: Option<&T>) -> *const T {
//     if let Some(t) = opt {
//         t as *const T
//     } else {
//         std::ptr::null()
//     }
// }

fn opt_mut_ptr<T>(opt: Option<&mut T>) -> *mut T {
    if let Some(t) = opt {
        t as *mut T
    } else {
        std::ptr::null_mut()
    }
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
