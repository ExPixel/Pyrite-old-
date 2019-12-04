use crate::imstr::{ImStr, ImStrBuf};
use crate::flags::*;
use crate::sys;

pub use crate::sys::{
    ImVec2,
    ImVec4,
    ImFontAtlas,
    ImDrawData,
    ImTextureID,

    ImGuiIO,
    ImGuiContext,
    ImGuiSizeCallbackData,
};

static mut GLOBAL_FORMATTING_BUFFER: [u8; 1025] = [0u8; 1025];

/// Returns a global text buffer that can be used for formatting without allocations.
/// At the moment this is not thread safe and should be done on the same thread as ImGui.
#[inline]
pub fn global_fmt_buffer() -> &'static mut [u8] {
    unsafe {
        &mut GLOBAL_FORMATTING_BUFFER
    }
}

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

pub fn get_style() -> Option<&'static mut sys::ImGuiStyle> {
    unsafe {
        sys::igGetStyle().as_mut()
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

pub fn get_window_draw_list() -> Option<&'static mut sys::ImDrawList> {
    unsafe {
        sys::igGetWindowDrawList().as_mut()
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

pub fn begin_child(str_id: &ImStr, size: ImVec2, border: bool, flags: WindowFlags) -> bool {
    unsafe {
        sys::igBeginChild(str_id.as_ptr(), size, border, flags.bits() as _)
    }
}

pub fn end_child() {
    unsafe {
        sys::igEndChild()
    }
}

#[inline]
pub fn same_line(offset_from_start_x: f32) {
    unsafe {
        sys::igSameLine(offset_from_start_x, -1.0f32);
    }
}

#[inline]
pub fn same_line_with_spacing(offset_from_start_x: f32, spacing: f32) {
    unsafe {
        sys::igSameLine(offset_from_start_x, spacing);
    }
}

pub fn get_mouse_cursor() -> MouseCursor {
    unsafe {
        MouseCursor::from_bits(sys::igGetMouseCursor()).expect("invalid mouse cursor value")
    }
}

pub fn get_text_line_height() -> f32 {
    unsafe {
        sys::igGetTextLineHeight()
    }
}

pub fn get_text_line_height_with_spacing() -> f32 {
    unsafe {
        sys::igGetTextLineHeightWithSpacing()
    }
}

pub fn calc_text_size(text: &ImStr, hide_text_after_double_hash: Option<bool>, wrap_width: Option<f32>) -> ImVec2 {
    unsafe {
        sys::igCalcTextSize_nonUDT2(
            text.begin(),
            text.end(),
            hide_text_after_double_hash.unwrap_or(true),
            wrap_width.unwrap_or(-1.0f32),
        ).into()
    }
}

pub fn image_with_colors(user_texture_id: sys::ImTextureID, size: ImVec2, uv0: Option<ImVec2>, uv1: Option<ImVec2>, tint_col: Option<ImVec4>, border_col: Option<ImVec4>) {
    unsafe {
        sys::igImage(user_texture_id, size,
            uv0.unwrap_or(vec2(0.0, 0.0)),
            uv1.unwrap_or(vec2(1.0, 1.0)),
            tint_col.unwrap_or(vec4(1.0, 1.0, 1.0, 1.0)),
            border_col.unwrap_or(vec4(0.0, 0.0, 0.0, 0.0)))
    }
}

pub fn image(user_texture_id: sys::ImTextureID, size: ImVec2) {
    image_with_colors(user_texture_id, size, None, None, None, None)
}

pub fn get_window_content_region_max() -> ImVec2 {
    unsafe {
        sys::igGetContentRegionAvail_nonUDT2().into()
    }
}

pub fn set_next_window_size_constraints(size_min: ImVec2, size_max: ImVec2, custom_callback: Option<fn(data: &mut sys::ImGuiSizeCallbackData)>) {
    unsafe {
        // @NOTE I don't even know, dude
        sys::igSetNextWindowSizeConstraints(size_min, size_max, Some(imgui_size_callback_trampoline),
            opt_mut_ptr(custom_callback.map(|cb| std::mem::transmute::<*mut std::ffi::c_void, &mut std::ffi::c_void>(cb as *mut std::ffi::c_void))));
    }
}

// @TODO make this use a closure instead at some point :P
unsafe extern "C" fn imgui_size_callback_trampoline(data: *mut sys::ImGuiSizeCallbackData) {
    if !(*data).UserData.is_null() {
        let real_callback = std::mem::transmute::<_, fn(data: &mut sys::ImGuiSizeCallbackData)>((*data).UserData);
        (*data).UserData = std::ptr::null_mut(); // temporarily remove it
        real_callback(std::mem::transmute(data));
        (*data).UserData = std::mem::transmute::<*mut std::ffi::c_void, &mut std::ffi::c_void>(real_callback as *mut std::ffi::c_void);
    }
}

pub fn push_style_var_float(idx: StyleVar, val: f32) {
    unsafe {
        sys::igPushStyleVarFloat(idx.bits() as _, val);
    }
}

pub fn push_style_var_vec2(idx: StyleVar, val: ImVec2) {
    unsafe {
        sys::igPushStyleVarVec2(idx.bits() as _, val);
    }
}

pub fn pop_style_var(count: i32) {
    unsafe {
        sys::igPopStyleVar(count)
    }
}

pub fn set_next_window_content_size(size: ImVec2) {
    unsafe {
        sys::igSetNextWindowContentSize(size)
    }
}

pub fn is_window_focused(flags: FocusedFlags) -> bool {
    unsafe {
        sys::igIsWindowFocused(flags.bits() as _)
    }
}

pub fn begin_main_menu_bar() -> bool {
    unsafe {
        sys::igBeginMainMenuBar()
    }
}

pub fn end_main_menu_bar() {
    unsafe {
        sys::igEndMainMenuBar()
    }
}

pub fn begin_menu_bar() -> bool {
    unsafe {
        sys::igBeginMenuBar()
    }
}

pub fn end_menu_bar() {
    unsafe {
        sys::igEndMenuBar()
    }
}

pub fn begin_menu(label: &ImStr, enabled: bool) -> bool {
    unsafe {
        sys::igBeginMenu(label.as_ptr(), enabled)
    }
}

pub fn end_menu() {
    unsafe {
        sys::igEndMenu()
    }
}

pub fn menu_item(label: &ImStr) -> bool {
    unsafe {
        sys::igMenuItemBool(label.as_ptr(), std::ptr::null(), false, true)
    }
}

pub fn menu_item_ex(label: &ImStr, shortcut: Option<&ImStr>, selected: bool, enabled: bool) -> bool {
    unsafe {
        sys::igMenuItemBool(label.as_ptr(), opt_str_ptr(shortcut), selected, enabled)
    }
}

pub fn plot_histogram(label: &ImStr, values: &[f32], offset: i32) {
    plot_histogram_ex(label, values, offset, None, std::f32::MAX, std::f32::MAX, vec2(0.0, 0.0), -1);
}

pub fn plot_histogram_ex(label: &ImStr, values: &[f32], offset: i32, overlay_text: Option<&ImStr>, scale_min: f32, scale_max: f32, graph_size: ImVec2, stride: i32) {
    unsafe {
        sys::igPlotHistogramFloatPtr(
            label.as_ptr(), values.as_ptr(), values.len() as i32, offset,
            opt_str_ptr(overlay_text), scale_min, scale_max, graph_size,
            if stride < 0 { std::mem::size_of::<f32>() as i32 } else { stride })
    }
}

#[inline]
pub fn text(s: &ImStr) {
    unsafe {
        sys::igText(s.as_ptr())
    }
}

pub fn get_scroll_x() -> f32 {
    unsafe {
        sys::igGetScrollX()
    }
}

pub fn get_scroll_y() -> f32 {
    unsafe {
        sys::igGetScrollY()
    }
}

pub fn get_scroll_max_x() -> f32 {
    unsafe {
        sys::igGetScrollMaxX()
    }
}

pub fn get_scroll_max_y() -> f32 {
    unsafe {
        sys::igGetScrollMaxY()
    }
}

pub fn set_scroll_x(scroll_x: f32) {
    unsafe {
        sys::igSetScrollX(scroll_x)
    }
}

pub fn set_scroll_y(scroll_y: f32) {
    unsafe {
        sys::igSetScrollY(scroll_y)
    }
}

#[inline]
pub fn set_cursor_pos_x(cursor_x: f32) {
    unsafe {
        sys::igSetCursorPosX(cursor_x)
    }
}

#[inline]
pub fn set_cursor_pos_y(cursor_y: f32) {
    unsafe {
        sys::igSetCursorPosY(cursor_y)
    }
}

#[inline]
pub fn set_cursor_pos(cursor_pos: ImVec2) {
    unsafe {
        sys::igSetCursorPos(cursor_pos)
    }
}

#[inline]
pub fn get_cursor_pos_x() -> f32 {
    unsafe {
        sys::igGetCursorPosX()
    }
}


#[inline]
pub fn get_cursor_pos_y() -> f32 {
    unsafe {
        sys::igGetCursorPosY()
    }
}

#[inline]
pub fn get_cursor_pos() -> ImVec2 {
    unsafe {
        sys::igGetCursorPos_nonUDT2().into()
    }
}

#[inline]
pub fn get_cursor_screen_pos() -> ImVec2 {
    unsafe {
        sys::igGetCursorScreenPos_nonUDT2().into()
    }
}

pub fn get_window_pos() -> ImVec2 {
    unsafe {
        sys::igGetWindowPos_nonUDT2().into()
    }
}

pub fn show_demo_window(open: &mut bool) {
    unsafe {
        sys::igShowDemoWindow(open)
    }
}

// @TODO implement input text callback
pub struct CannotConstruct( /*private*/ u32 );
pub fn input_text(label: &ImStr, buf: &mut ImStrBuf, flags: InputTextFlags, _callback: Option<CannotConstruct>) -> bool {
    unsafe {
        sys::igInputText(
            label.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as _,
            flags.bits() as _,
            None,
            std::ptr::null_mut(),
        )
    }
}

#[inline]
pub fn button(label: &ImStr) -> bool {
    unsafe {
        sys::igButton(label.as_ptr(), vec2(0.0, 0.0))
    }
}

pub fn button_with_size(label: &ImStr, size: ImVec2) -> bool {
    unsafe {
        sys::igButton(label.as_ptr(), size)
    }
}

pub fn set_next_item_width(item_width: f32) {
    unsafe {
        sys::igSetNextItemWidth(item_width)
    }
}

#[inline]
pub fn get_key_index(key: Key) -> i32 {
    unsafe {
        sys::igGetKeyIndex(key.bits())
    }
}

#[inline]
pub fn is_key_pressed(key_index: i32, repeat: bool) -> bool {
    unsafe {
        sys::igIsKeyPressed(key_index, repeat)
    }
}

#[inline]
pub fn is_key_released(key_index: i32) -> bool {
    unsafe {
        sys::igIsKeyReleased(key_index)
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
    };

    ($OwnedType:ident, $TargetType:ty, $Destructor:path) => {
        pub struct $OwnedType(*mut $TargetType);

        impl $OwnedType {
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
    };
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

impl ListClipper {
    pub fn new(items_count: i32, items_height: f32) -> ListClipper {
        ListClipper(unsafe {
            sys::ImGuiListClipper_ImGuiListClipper(items_count, items_height)
        })
    }
}
create_owned_impl!(ListClipper, sys::ImGuiListClipper, sys::ImGuiListClipper_destroy);

impl sys::ImGuiListClipper {
    pub fn step(&mut self) -> bool {
        unsafe {
            sys::ImGuiListClipper_Step(self)
        }
    }

    pub fn begin(&mut self, items_count: i32, items_height: f32) {
        unsafe {
            sys::ImGuiListClipper_Begin(self, items_count, items_height)
        }
    }

    pub fn end(&mut self) {
        unsafe {
            sys::ImGuiListClipper_End(self)
        }
    }
}

impl sys::ImDrawList {
    pub fn add_line(&mut self, p1: ImVec2, p2: ImVec2, col: u32, thickness: f32) {
        unsafe {
            sys::ImDrawList_AddLine(self, p1, p2, col, thickness)
        }
    }

    pub fn add_rect(&mut self, p_min: ImVec2, p_max: ImVec2, col: u32, thickness: f32) {
        self.add_round_rect(p_min, p_max, col, 1.0, DrawCornerFlags::All, thickness)
    }

    pub fn add_round_rect(&mut self, p_min: ImVec2, p_max: ImVec2, col: u32, rounding: f32, rounding_corners: DrawCornerFlags, thickness: f32) {
        unsafe {
            sys::ImDrawList_AddRect(self, p_min, p_max, col, rounding, rounding_corners.bits() as _, thickness)
        }
    }

    pub fn add_rect_filled(&mut self, p_min: ImVec2, p_max: ImVec2, col: u32) {
        self.add_round_rect_filled(p_min, p_max, col, 1.0, DrawCornerFlags::All)
    }

    pub fn add_round_rect_filled(&mut self, p_min: ImVec2, p_max: ImVec2, col: u32, rounding: f32, rounding_corners: DrawCornerFlags) {
        unsafe {
            sys::ImDrawList_AddRectFilled(self, p_min, p_max, col, rounding, rounding_corners.bits() as _)
        }
    }

    pub fn add_triangle(&mut self, p1: ImVec2, p2: ImVec2, p3: ImVec2, col: u32, thickness: f32) {
        unsafe {
            sys::ImDrawList_AddTriangle(self, p1, p2, p3, col, thickness)
        }
    }

    pub fn add_circle(&mut self, center: ImVec2, radius: f32, col: u32, segments: Option<i32>, thickness: f32) {
        unsafe {
            sys::ImDrawList_AddCircle(self, center, radius, col, segments.unwrap_or(12), thickness)
        }
    }

    pub fn add_triangle_filled(&mut self, p1: ImVec2, p2: ImVec2, p3: ImVec2, col: u32) {
        unsafe {
            sys::ImDrawList_AddTriangleFilled(self, p1, p2, p3, col)
        }
    }

    pub fn add_circle_filled(&mut self, center: ImVec2, radius: f32, col: u32, segments: Option<i32>) {
        unsafe {
            sys::ImDrawList_AddCircleFilled(self, center, radius, col, segments.unwrap_or(12))
        }
    }
}
/*
    IMGUI_API void  AddLine(const ImVec2& p1, const ImVec2& p2, ImU32 col, float thickness = 1.0f);
    IMGUI_API void  AddRect(const ImVec2& p_min, const ImVec2& p_max, ImU32 col, float rounding = 0.0f, ImDrawCornerFlags rounding_corners = ImDrawCornerFlags_All, float thickness = 1.0f);   // a: upper-left, b: lower-right (== upper-left + size), rounding_corners_flags: 4-bits corresponding to which corner to round
    IMGUI_API void  AddRectFilled(const ImVec2& p_min, const ImVec2& p_max, ImU32 col, float rounding = 0.0f, ImDrawCornerFlags rounding_corners = ImDrawCornerFlags_All);                     // a: upper-left, b: lower-right (== upper-left + size)
    IMGUI_API void  AddRectFilledMultiColor(const ImVec2& p_min, const ImVec2& p_max, ImU32 col_upr_left, ImU32 col_upr_right, ImU32 col_bot_right, ImU32 col_bot_left);
    IMGUI_API void  AddQuad(const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, const ImVec2& p4, ImU32 col, float thickness = 1.0f);
    IMGUI_API void  AddQuadFilled(const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, const ImVec2& p4, ImU32 col);
    IMGUI_API void  AddTriangle(const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, ImU32 col, float thickness = 1.0f);
    IMGUI_API void  AddTriangleFilled(const ImVec2& p1, const ImVec2& p2, const ImVec2& p3, ImU32 col);
    IMGUI_API void  AddCircle(const ImVec2& center, float radius, ImU32 col, int num_segments = 12, float thickness = 1.0f);
    IMGUI_API void  AddCircleFilled(const ImVec2& center, float radius, ImU32 col, int num_segments = 12);
    IMGUI_API void  AddText(const ImVec2& pos, ImU32 col, const char* text_begin, const char* text_end = NULL);
    IMGUI_API void  AddText(const ImFont* font, float font_size, const ImVec2& pos, ImU32 col, const char* text_begin, const char* text_end = NULL, float wrap_width = 0.0f, const ImVec4* cpu_fine_clip_rect = NULL);
    IMGUI_API void  AddPolyline(const ImVec2* points, int num_points, ImU32 col, bool closed, float thickness);
    IMGUI_API void  AddConvexPolyFilled(const ImVec2* points, int num_points, ImU32 col); // Note: Anti-aliased filling requires points to be in clockwise order.
    IMGUI_API void  AddBezierCurve(const ImVec2& pos0, const ImVec2& cp0, const ImVec2& cp1, const ImVec2& pos1, ImU32 col, float thickness, int num_segments = 0);
*/

/////////////////////////////////////////////
//
//             UTILITY FUNCTIONS
//
////////////////////////////////////////////

pub const R_SHIFT: u32 =  0;
pub const G_SHIFT: u32 =  8;
pub const B_SHIFT: u32 = 16;
pub const A_SHIFT: u32 = 24;

/// Converts 0xRRGGBB to 0xAABBGGRR placing 0xFF in AA.
#[inline]
pub const fn rgb(value: u32) -> u32 {
    value.swap_bytes() | (0xFF << A_SHIFT)
}

/// Converts 0xAARRGGBB to 0xAABBGGRR
#[inline]
pub const fn rgba(value: u32) -> u32 {
    value.swap_bytes().rotate_right(8)
}

#[inline]
pub const fn rgb8(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << R_SHIFT) |
    ((g as u32) << G_SHIFT) |
    ((b as u32) << B_SHIFT) |
    (0xFF << A_SHIFT)
}

#[inline]
pub const fn rgba8(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((r as u32) << R_SHIFT) |
    ((g as u32) << G_SHIFT) |
    ((b as u32) << B_SHIFT) |
    ((a as u32) << A_SHIFT)
}

#[inline]
pub const fn vec2(x: f32, y: f32) -> ImVec2 {
    ImVec2 { x, y }
}

#[inline]
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

fn opt_str_ptr(opt: Option<&ImStr>) -> *const i8 {
    if let Some(s) = opt {
        s.as_ptr()
    } else {
        std::ptr::null()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn check_version_and_data_layout() {
        use std::mem::size_of;
        assert_eq!(super::debug_version_and_data_layout(
            super::get_version(),
            size_of::<super::sys::ImGuiIO>(),
            size_of::<super::sys::ImGuiStyle>(),
            size_of::<super::sys::ImVec2>(),
            size_of::<super::sys::ImVec4>(),
            size_of::<super::sys::ImDrawVert>(),
            size_of::<super::sys::ImDrawIdx>()
        ), true);
    }
}
