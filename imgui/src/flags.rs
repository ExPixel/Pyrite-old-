use crate::sys;

pub type FlagType = i32;
pub trait ZeroFlag {
    fn get_zero_flag() -> Self;
}

macro_rules! create_zero_flag_impl {
    ($FlagType:ident, $FlagZero:ident) => (
        impl ZeroFlag for $FlagType {
            fn get_zero_flag() -> $FlagType {
                $FlagType::$FlagZero
            }
        }
    )
}

bitflags::bitflags! {
    pub struct BackendFlags: sys::ImGuiBackendFlags {
        const None = sys::ImGuiBackendFlags__ImGuiBackendFlags_None as FlagType;
        const HasGamePad = sys::ImGuiBackendFlags__ImGuiBackendFlags_HasGamepad as FlagType;
        const HasSetMousePos = sys::ImGuiBackendFlags__ImGuiBackendFlags_HasSetMousePos as FlagType;
        const HasMouseCursors = sys::ImGuiBackendFlags__ImGuiBackendFlags_HasMouseCursors as FlagType;
    }
}
create_zero_flag_impl!(BackendFlags, None);

bitflags::bitflags! {
    pub struct WindowFlags: sys::ImGuiWindowFlags {
        const None = sys::ImGuiWindowFlags__ImGuiWindowFlags_None as FlagType;
        const NoTitleBar = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoTitleBar as FlagType;
        const NoResize = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoResize as FlagType;
        const NoMove = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoMove as FlagType;
        const NoScrollbar = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoScrollbar as FlagType;
        const NoScrollWithMouse = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoScrollWithMouse as FlagType;
        const NoCollapse = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoCollapse as FlagType;
        const AlwaysAutoResize = sys::ImGuiWindowFlags__ImGuiWindowFlags_AlwaysAutoResize as FlagType;
        const NoBackground = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoBackground as FlagType;
        const NoSavedSettings = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoSavedSettings as FlagType;
        const NoMouseInputs = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoMouseInputs as FlagType;
        const MenuBar = sys::ImGuiWindowFlags__ImGuiWindowFlags_MenuBar as FlagType;
        const HorizontalScrollbar = sys::ImGuiWindowFlags__ImGuiWindowFlags_HorizontalScrollbar as FlagType;
        const NoFocusOnAppearing = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoFocusOnAppearing as FlagType;
        const NoBringToFrontOnFocus = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoBringToFrontOnFocus as FlagType;
        const AlwaysVerticalScrollbar = sys::ImGuiWindowFlags__ImGuiWindowFlags_AlwaysVerticalScrollbar as FlagType;
        const AlwaysHorizontalScrollbar = sys::ImGuiWindowFlags__ImGuiWindowFlags_AlwaysHorizontalScrollbar as FlagType as FlagType;
        const AlwaysUseWindowPadding = sys::ImGuiWindowFlags__ImGuiWindowFlags_AlwaysUseWindowPadding as FlagType;
        const NoNavInputs = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoNavInputs as FlagType;
        const NoNavFocus = sys::ImGuiWindowFlags__ImGuiWindowFlags_NoNavFocus as FlagType;
    }
}
create_zero_flag_impl!(WindowFlags, None);

bitflags::bitflags! {
    pub struct Key: sys::ImGuiKey {
        const A = sys::ImGuiKey__ImGuiKey_A as FlagType;
        const C = sys::ImGuiKey__ImGuiKey_C as FlagType;
        const V = sys::ImGuiKey__ImGuiKey_V as FlagType;
        const X = sys::ImGuiKey__ImGuiKey_X as FlagType;
        const Y = sys::ImGuiKey__ImGuiKey_Y as FlagType;
        const Z = sys::ImGuiKey__ImGuiKey_Z as FlagType;
        const End = sys::ImGuiKey__ImGuiKey_End as FlagType;
        const Tab = sys::ImGuiKey__ImGuiKey_Tab as FlagType;
        const Home = sys::ImGuiKey__ImGuiKey_Home as FlagType;
        const Enter = sys::ImGuiKey__ImGuiKey_Enter as FlagType;
        const Space = sys::ImGuiKey__ImGuiKey_Space as FlagType;
        const Delete = sys::ImGuiKey__ImGuiKey_Delete as FlagType;
        const Escape = sys::ImGuiKey__ImGuiKey_Escape as FlagType;
        const Insert = sys::ImGuiKey__ImGuiKey_Insert as FlagType;
        const PageUp = sys::ImGuiKey__ImGuiKey_PageUp as FlagType;
        const UpArrow = sys::ImGuiKey__ImGuiKey_UpArrow as FlagType;
        const PageDown = sys::ImGuiKey__ImGuiKey_PageDown as FlagType;
        const Backspace = sys::ImGuiKey__ImGuiKey_Backspace as FlagType;
        const DownArrow = sys::ImGuiKey__ImGuiKey_DownArrow as FlagType;
        const LeftArrow = sys::ImGuiKey__ImGuiKey_LeftArrow as FlagType;
        const RightArrow = sys::ImGuiKey__ImGuiKey_RightArrow as FlagType;
        const COUNT = sys::ImGuiKey__ImGuiKey_COUNT as FlagType;
    }
}

bitflags::bitflags! {
    pub struct ConfigFlags: sys::ImGuiConfigFlags {
        const None = sys::ImGuiConfigFlags__ImGuiConfigFlags_None as FlagType;
        const IsSRGB = sys::ImGuiConfigFlags__ImGuiConfigFlags_IsSRGB as FlagType;
        const NoMouse = sys::ImGuiConfigFlags__ImGuiConfigFlags_NoMouse as FlagType;
        const IsTouchScreen = sys::ImGuiConfigFlags__ImGuiConfigFlags_IsTouchScreen as FlagType;
        const NavEnableGamepad = sys::ImGuiConfigFlags__ImGuiConfigFlags_NavEnableGamepad as FlagType;
        const NavEnableKeyboard = sys::ImGuiConfigFlags__ImGuiConfigFlags_NavEnableKeyboard as FlagType;
        const NoMouseCursorChange = sys::ImGuiConfigFlags__ImGuiConfigFlags_NoMouseCursorChange as FlagType;
        const NavEnableSetMousePos = sys::ImGuiConfigFlags__ImGuiConfigFlags_NavEnableSetMousePos as FlagType;
        const NavNoCaptureKeyboard = sys::ImGuiConfigFlags__ImGuiConfigFlags_NavNoCaptureKeyboard as FlagType;
    }
}
create_zero_flag_impl!(ConfigFlags, None);

bitflags::bitflags! {
    pub struct MouseCursor: sys::ImGuiMouseCursor {
        const None = sys::ImGuiMouseCursor__ImGuiMouseCursor_None as FlagType;
        const Hand = sys::ImGuiMouseCursor__ImGuiMouseCursor_Hand as FlagType;
        const Arrow = sys::ImGuiMouseCursor__ImGuiMouseCursor_Arrow as FlagType;
        const ResizeEW = sys::ImGuiMouseCursor__ImGuiMouseCursor_ResizeEW as FlagType;
        const ResizeNS = sys::ImGuiMouseCursor__ImGuiMouseCursor_ResizeNS as FlagType;
        const ResizeNESW = sys::ImGuiMouseCursor__ImGuiMouseCursor_ResizeNESW as FlagType;
        const ResizeNWSE = sys::ImGuiMouseCursor__ImGuiMouseCursor_ResizeNWSE as FlagType;
        const ResizeAll = sys::ImGuiMouseCursor__ImGuiMouseCursor_ResizeAll as FlagType;
        const TextInput = sys::ImGuiMouseCursor__ImGuiMouseCursor_TextInput as FlagType;
    }
}
create_zero_flag_impl!(MouseCursor, None);

bitflags::bitflags! {
    pub struct TreeNodeFlags: sys::ImGuiTreeNodeFlags {
        const None = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_None as FlagType;
        const Leaf = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_Leaf as FlagType;
        const Bullet = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_Bullet as FlagType;
        const Framed = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_Framed as FlagType;
        const Selected = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_Selected as FlagType;
        const DefaultOpen = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_DefaultOpen as FlagType;
        const OpenOnArrow = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_OpenOnArrow as FlagType;
        const FramePadding = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_FramePadding as FlagType;
        const NoAutoOpenOnLog = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_NoAutoOpenOnLog as FlagType;
        const AllowItemOverlap = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_AllowItemOverlap as FlagType;
        const CollapsingHeader = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_CollapsingHeader as FlagType;
        const NoTreePushOnOpen = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_NoTreePushOnOpen as FlagType;
        const OpenOnDoubleClick = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_OpenOnDoubleClick as FlagType;
        const NavLeftJumpsBackHere = sys::ImGuiTreeNodeFlags__ImGuiTreeNodeFlags_NavLeftJumpsBackHere as FlagType;
    }
}
create_zero_flag_impl!(TreeNodeFlags, None);

bitflags::bitflags! {
    pub struct Cond: sys::ImGuiCond {
        const None = 0 as FlagType;
        const Once = sys::ImGuiCond__ImGuiCond_Once as FlagType;
        const Always = sys::ImGuiCond__ImGuiCond_Always as FlagType;
        const Appearing = sys::ImGuiCond__ImGuiCond_Appearing as FlagType;
        const FirstEverUse = sys::ImGuiCond__ImGuiCond_FirstUseEver as FlagType;
    }
}
create_zero_flag_impl!(Cond, None);

bitflags::bitflags! {
    pub struct StyleVar: sys::ImGuiStyleVar {
        const None = 0 as FlagType;
        const Alpha = sys::ImGuiStyleVar__ImGuiStyleVar_Alpha as FlagType;
        const GrabMinSize = sys::ImGuiStyleVar__ImGuiStyleVar_GrabMinSize as FlagType;
        const ItemSpacing = sys::ImGuiStyleVar__ImGuiStyleVar_ItemSpacing as FlagType;
        const FramePadding = sys::ImGuiStyleVar__ImGuiStyleVar_FramePadding as FlagType;
        const GrabRounding = sys::ImGuiStyleVar__ImGuiStyleVar_GrabRounding as FlagType;
        const ChildRounding = sys::ImGuiStyleVar__ImGuiStyleVar_ChildRounding as FlagType;
        const FrameRounding = sys::ImGuiStyleVar__ImGuiStyleVar_FrameRounding as FlagType;
        const IndentSpacing = sys::ImGuiStyleVar__ImGuiStyleVar_IndentSpacing as FlagType;
        const PopupRounding = sys::ImGuiStyleVar__ImGuiStyleVar_PopupRounding as FlagType;
        const ScrollbarSize = sys::ImGuiStyleVar__ImGuiStyleVar_ScrollbarSize as FlagType;
        const WindowMinSize = sys::ImGuiStyleVar__ImGuiStyleVar_WindowMinSize as FlagType;
        const WindowPadding = sys::ImGuiStyleVar__ImGuiStyleVar_WindowPadding as FlagType;
        const WindowRounding = sys::ImGuiStyleVar__ImGuiStyleVar_WindowRounding as FlagType;
        const ButtonTextAlign = sys::ImGuiStyleVar__ImGuiStyleVar_ButtonTextAlign as FlagType;
        const ChildBorderSize = sys::ImGuiStyleVar__ImGuiStyleVar_ChildBorderSize as FlagType;
        const FrameBorderSize = sys::ImGuiStyleVar__ImGuiStyleVar_FrameBorderSize as FlagType;
        const PopupBorderSize = sys::ImGuiStyleVar__ImGuiStyleVar_PopupBorderSize as FlagType;
        const ItemInnerSpacing = sys::ImGuiStyleVar__ImGuiStyleVar_ItemInnerSpacing as FlagType;
        const WindowBorderSize = sys::ImGuiStyleVar__ImGuiStyleVar_WindowBorderSize as FlagType;
        const WindowTitleAlign = sys::ImGuiStyleVar__ImGuiStyleVar_WindowTitleAlign as FlagType;
        const ScrollbarRounding = sys::ImGuiStyleVar__ImGuiStyleVar_ScrollbarRounding as FlagType;
    }
}
create_zero_flag_impl!(StyleVar, None);

bitflags::bitflags! {
    pub struct Col: sys::ImGuiCol {
        const None = 0 as FlagType;
        const Text = sys::ImGuiCol__ImGuiCol_Text as FlagType;
        const Border = sys::ImGuiCol__ImGuiCol_Border as FlagType;
        const Button = sys::ImGuiCol__ImGuiCol_Button as FlagType;
        const Header = sys::ImGuiCol__ImGuiCol_Header as FlagType;
        const ChildBg = sys::ImGuiCol__ImGuiCol_ChildBg as FlagType;
        const FrameBg = sys::ImGuiCol__ImGuiCol_FrameBg as FlagType;
        const PopupBg = sys::ImGuiCol__ImGuiCol_PopupBg as FlagType;
        const TitleBg = sys::ImGuiCol__ImGuiCol_TitleBg as FlagType;
        const WindowBg = sys::ImGuiCol__ImGuiCol_WindowBg as FlagType;
        const CheckMark = sys::ImGuiCol__ImGuiCol_CheckMark as FlagType;
        const MenuBarBg = sys::ImGuiCol__ImGuiCol_MenuBarBg as FlagType;
        const PlotLines = sys::ImGuiCol__ImGuiCol_PlotLines as FlagType;
        const Separator = sys::ImGuiCol__ImGuiCol_Separator as FlagType;
        const ResizeGrip = sys::ImGuiCol__ImGuiCol_ResizeGrip as FlagType;
        const SliderGrab = sys::ImGuiCol__ImGuiCol_SliderGrab as FlagType;
        const ScrollbarBg = sys::ImGuiCol__ImGuiCol_ScrollbarBg as FlagType;
        const BorderShadow = sys::ImGuiCol__ImGuiCol_BorderShadow as FlagType;
        const ButtonActive = sys::ImGuiCol__ImGuiCol_ButtonActive as FlagType;
        const HeaderActive = sys::ImGuiCol__ImGuiCol_HeaderActive as FlagType;
        const NavHighlight = sys::ImGuiCol__ImGuiCol_NavHighlight as FlagType;
        const TextDisabled = sys::ImGuiCol__ImGuiCol_TextDisabled as FlagType;
        const ButtonHovered = sys::ImGuiCol__ImGuiCol_ButtonHovered as FlagType;
        const FrameBgActive = sys::ImGuiCol__ImGuiCol_FrameBgActive as FlagType;
        const HeaderHovered = sys::ImGuiCol__ImGuiCol_HeaderHovered as FlagType;
        const PlotHistogram = sys::ImGuiCol__ImGuiCol_PlotHistogram as FlagType;
        const ScrollbarGrab = sys::ImGuiCol__ImGuiCol_ScrollbarGrab as FlagType;
        const TitleBgActive = sys::ImGuiCol__ImGuiCol_TitleBgActive as FlagType;
        const DragDropTarget = sys::ImGuiCol__ImGuiCol_DragDropTarget as FlagType;
        const FrameBgHovered = sys::ImGuiCol__ImGuiCol_FrameBgHovered as FlagType;
        const TextSelectedBg = sys::ImGuiCol__ImGuiCol_TextSelectedBg as FlagType;
        const SeparatorActive = sys::ImGuiCol__ImGuiCol_SeparatorActive as FlagType;
        const ModalWindowDimBg = sys::ImGuiCol__ImGuiCol_ModalWindowDimBg as FlagType;
        const PlotLinesHovered = sys::ImGuiCol__ImGuiCol_PlotLinesHovered as FlagType;
        const ResizeGripActive = sys::ImGuiCol__ImGuiCol_ResizeGripActive as FlagType;
        const SeparatorHovered = sys::ImGuiCol__ImGuiCol_SeparatorHovered as FlagType;
        const SliderGrabActive = sys::ImGuiCol__ImGuiCol_SliderGrabActive as FlagType;
        const TitleBgCollapsed = sys::ImGuiCol__ImGuiCol_TitleBgCollapsed as FlagType;
        const NavWindowingDimBg = sys::ImGuiCol__ImGuiCol_NavWindowingDimBg as FlagType;
        const ResizeGripHovered = sys::ImGuiCol__ImGuiCol_ResizeGripHovered as FlagType;
        const ScrollbarGrabActive = sys::ImGuiCol__ImGuiCol_ScrollbarGrabActive as FlagType;
        const PlotHistogramHovered = sys::ImGuiCol__ImGuiCol_PlotHistogramHovered as FlagType;
        const ScrollbarGrabHovered = sys::ImGuiCol__ImGuiCol_ScrollbarGrabHovered as FlagType;
        const NavWindowingHighlight = sys::ImGuiCol__ImGuiCol_NavWindowingHighlight as FlagType;
    }
}
create_zero_flag_impl!(Col, None);

bitflags::bitflags! {
    pub struct ColorEditFlags: sys::ImGuiColorEditFlags {
        const None = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_None as FlagType;
        const HDR = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_HDR as FlagType;
        const Float = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_Float as FlagType;
        const Uint8 = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_Uint8 as FlagType;
        const NoAlpha = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_NoAlpha as FlagType;
        const NoLabel = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_NoLabel as FlagType;
        const AlphaBar = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_AlphaBar as FlagType;
        const NoInputs = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_NoInputs as FlagType;
        const NoPicker = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_NoPicker as FlagType;
        const NoOptions = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_NoOptions as FlagType;
        const NoTooltip = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_NoTooltip as FlagType;
        const NoDragDrop = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_NoDragDrop as FlagType;
        const _InputsMask = sys::ImGuiColorEditFlags__ImGuiColorEditFlags__InputMask as FlagType;
        const _PickerMask = sys::ImGuiColorEditFlags__ImGuiColorEditFlags__PickerMask as FlagType;
        const AlphaPreview = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_AlphaPreview as FlagType;
        const PickerHueBar = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_PickerHueBar as FlagType;
        const _DataTypeMask = sys::ImGuiColorEditFlags__ImGuiColorEditFlags__DataTypeMask as FlagType;
        const NoSidePreview = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_NoSidePreview as FlagType;
        const NoSmallPreview = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_NoSmallPreview as FlagType;
        const PickerHueWheel = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_PickerHueWheel as FlagType;
        const _OptionsDefault = sys::ImGuiColorEditFlags__ImGuiColorEditFlags__OptionsDefault as FlagType;
        const AlphaPreviewHalf = sys::ImGuiColorEditFlags__ImGuiColorEditFlags_AlphaPreviewHalf as FlagType;
    }
}
create_zero_flag_impl!(ColorEditFlags, None);

bitflags::bitflags! {
    pub struct InputTextFlags: sys::ImGuiInputTextFlags {
        const None = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_None as FlagType;
        const Password = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_Password as FlagType;
        const ReadOnly = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_ReadOnly as FlagType;
        const Multiline = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_Multiline as FlagType;
        const NoUndoRedo = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_NoUndoRedo as FlagType;
        const CharsDecimal = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CharsDecimal as FlagType;
        const CharsNoBlank = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CharsNoBlank as FlagType;
        const AllowTabInput = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_AllowTabInput as FlagType;
        const AutoSelectAll = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_AutoSelectAll as FlagType;
        const CallbackAlways = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CallbackAlways as FlagType;
        const CallbackResize = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CallbackResize as FlagType;
        const CharsUppercase = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CharsUppercase as FlagType;
        const CallbackHistory = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CallbackHistory as FlagType;
        const CharsScientific = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CharsScientific as FlagType;
        const AlwaysInsertMode = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_AlwaysInsertMode as FlagType;
        const CharsHexadecimal = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CharsHexadecimal as FlagType;
        const EnterReturnsTrue = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_EnterReturnsTrue as FlagType;
        const CallbackCharFilter = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CallbackCharFilter as FlagType;
        const CallbackCompletion = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CallbackCompletion as FlagType;
        const NoHorizontalScroll = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_NoHorizontalScroll as FlagType;
        const CtrlEnterForNewLine = sys::ImGuiInputTextFlags__ImGuiInputTextFlags_CtrlEnterForNewLine as FlagType;
    }
}
create_zero_flag_impl!(InputTextFlags, None);

bitflags::bitflags! {
    pub struct HoveredFlags: sys::ImGuiHoveredFlags {
        const None = sys::ImGuiHoveredFlags__ImGuiHoveredFlags_None as FlagType;
        const RectOnly = sys::ImGuiHoveredFlags__ImGuiHoveredFlags_RectOnly as FlagType;
        const AnyWindow = sys::ImGuiHoveredFlags__ImGuiHoveredFlags_AnyWindow as FlagType;
        const RootWindow = sys::ImGuiHoveredFlags__ImGuiHoveredFlags_RootWindow as FlagType;
        const ChildWindows = sys::ImGuiHoveredFlags__ImGuiHoveredFlags_ChildWindows as FlagType;
        const AllowWhenDisabled = sys::ImGuiHoveredFlags__ImGuiHoveredFlags_AllowWhenDisabled as FlagType;
        const AllowWhenOverlapped = sys::ImGuiHoveredFlags__ImGuiHoveredFlags_AllowWhenOverlapped as FlagType;
        const RootAndChildWindows = sys::ImGuiHoveredFlags__ImGuiHoveredFlags_RootAndChildWindows as FlagType;
        const AllowWhenBlockedByPopup = sys::ImGuiHoveredFlags__ImGuiHoveredFlags_AllowWhenBlockedByPopup as FlagType;
        const AllowWhenBlockedByActiveItem = sys::ImGuiHoveredFlags__ImGuiHoveredFlags_AllowWhenBlockedByActiveItem as FlagType;
    }
}
create_zero_flag_impl!(HoveredFlags, None);

bitflags::bitflags! {
    pub struct ComboFlags: sys::ImGuiComboFlags {
        const None = sys::ImGuiComboFlags__ImGuiComboFlags_None as FlagType;
        const NoPreview = sys::ImGuiComboFlags__ImGuiComboFlags_NoPreview as FlagType;
        const HeightLarge = sys::ImGuiComboFlags__ImGuiComboFlags_HeightLarge as FlagType;
        const HeightMask_ = sys::ImGuiComboFlags__ImGuiComboFlags_HeightMask_ as FlagType;
        const HeightSmall = sys::ImGuiComboFlags__ImGuiComboFlags_HeightSmall as FlagType;
        const HeightLargest = sys::ImGuiComboFlags__ImGuiComboFlags_HeightLargest as FlagType;
        const HeightRegular = sys::ImGuiComboFlags__ImGuiComboFlags_HeightRegular as FlagType;
        const NoArrowButton = sys::ImGuiComboFlags__ImGuiComboFlags_NoArrowButton as FlagType;
        const PopupAlignLeft = sys::ImGuiComboFlags__ImGuiComboFlags_PopupAlignLeft as FlagType;
    }
}
create_zero_flag_impl!(ComboFlags, None);

bitflags::bitflags! {
    pub struct DrawCornerFlags: sys::ImDrawCornerFlags {
        const All = sys::ImDrawCornerFlags__ImDrawCornerFlags_All as FlagType;
        const Bot = sys::ImDrawCornerFlags__ImDrawCornerFlags_Bot as FlagType;
        const Top = sys::ImDrawCornerFlags__ImDrawCornerFlags_Top as FlagType;
        const Left = sys::ImDrawCornerFlags__ImDrawCornerFlags_Left as FlagType;
        const Right = sys::ImDrawCornerFlags__ImDrawCornerFlags_Right as FlagType;
        const BotLeft = sys::ImDrawCornerFlags__ImDrawCornerFlags_BotLeft as FlagType;
        const TopLeft = sys::ImDrawCornerFlags__ImDrawCornerFlags_TopLeft as FlagType;
        const BotRight = sys::ImDrawCornerFlags__ImDrawCornerFlags_BotRight as FlagType;
        const TopRight = sys::ImDrawCornerFlags__ImDrawCornerFlags_TopRight as FlagType;
    }
}
create_zero_flag_impl!(DrawCornerFlags, All);

bitflags::bitflags! {
    pub struct FocusedFlags: sys::ImGuiFocusedFlags {
        const None =  sys::ImGuiFocusedFlags__ImGuiFocusedFlags_None as FlagType;
        const ChildWindows =  sys::ImGuiFocusedFlags__ImGuiFocusedFlags_ChildWindows as FlagType;
        const RootWindow =  sys::ImGuiFocusedFlags__ImGuiFocusedFlags_RootWindow as FlagType;
        const AnyWindow =  sys::ImGuiFocusedFlags__ImGuiFocusedFlags_AnyWindow as FlagType;
        const RootAndChildWindows =  sys::ImGuiFocusedFlags__ImGuiFocusedFlags_RootAndChildWindows as FlagType;
    }
}
create_zero_flag_impl!(FocusedFlags, None);

bitflags::bitflags! {
    pub struct SelectableFlags: sys::ImGuiSelectableFlags {
        const None = sys::ImGuiSelectableFlags__ImGuiSelectableFlags_None as FlagType;
        const DontClosePopups = sys::ImGuiSelectableFlags__ImGuiSelectableFlags_DontClosePopups as FlagType;
        const SpanAllColumns = sys::ImGuiSelectableFlags__ImGuiSelectableFlags_SpanAllColumns as FlagType;
        const AllowDoubleClick = sys::ImGuiSelectableFlags__ImGuiSelectableFlags_AllowDoubleClick as FlagType;
        const Disabled = sys::ImGuiSelectableFlags__ImGuiSelectableFlags_Disabled as FlagType;
    }
}
create_zero_flag_impl!(SelectableFlags, None);

#[inline(always)]
pub fn none<T: ZeroFlag>() -> T {
    T::get_zero_flag()
}
