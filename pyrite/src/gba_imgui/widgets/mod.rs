pub mod disassembly;
pub mod memory_editor;
use crate::debugger::GbaDebugger;

use crate::platform::opengl::GbaTexture;
use disassembly::DisassemblyWindow;
use memory_editor::MemoryEditorWindow;
use pyrite_gba::Gba;

/// Main emulator GUI struct.
pub struct EmulatorGUI {
    gba_display: GbaDisplayWindow,
    stats_window: EmulatorStatsWindow,
    disassembly_window: DisassemblyWindow,
    memory_editor_window: MemoryEditorWindow,

    /// @TODO remove this later. For now I use it because I'm not very familiar with all of ImGui's
    /// features.
    show_demo_window: bool,
}

impl EmulatorGUI {
    pub fn new() -> EmulatorGUI {
        EmulatorGUI {
            gba_display: GbaDisplayWindow::new(),
            stats_window: EmulatorStatsWindow::new(),
            disassembly_window: DisassemblyWindow::new(),
            memory_editor_window: MemoryEditorWindow::new(),
            show_demo_window: false,
        }
    }

    pub fn draw(&mut self, gba: &mut Gba, gba_texture: &GbaTexture, debugger: &mut GbaDebugger) {
        self.draw_menu_bar();
        if self.gba_display.open {
            self.gba_display.draw(&gba_texture);
        }
        if self.stats_window.open {
            self.stats_window.draw();
        }
        if self.disassembly_window.open {
            self.disassembly_window
                .draw(debugger, &gba.cpu, &gba.hardware);
        }
        if self.memory_editor_window.open {
            self.memory_editor_window
                .draw(&gba.hardware, 0x0FFFFFFF, 0x0);
        }
        if self.show_demo_window {
            imgui::show_demo_window(&mut self.show_demo_window);
        }
    }

    fn draw_menu_bar(&mut self) {
        if imgui::begin_main_menu_bar() {
            if imgui::begin_menu(imgui::str!("File"), true) {
                if imgui::menu_item(imgui::str!("Load ROM...")) { /* NOP */ }
                imgui::end_menu();
            }

            if imgui::begin_menu(imgui::str!("View"), true) {
                if imgui::menu_item_ex(
                    imgui::str!("Memory Editor"),
                    None,
                    self.memory_editor_window.open,
                    true,
                ) {
                    self.memory_editor_window.open = !self.memory_editor_window.open;
                }

                if imgui::menu_item_ex(
                    imgui::str!("Disassembly"),
                    None,
                    self.disassembly_window.open,
                    true,
                ) {
                    self.disassembly_window.open = !self.disassembly_window.open;
                }

                if imgui::menu_item_ex(
                    imgui::str!("GBA Display"),
                    None,
                    self.gba_display.open,
                    true,
                ) {
                    self.gba_display.open = !self.gba_display.open;
                }

                if imgui::begin_menu(imgui::str!("GBA Display Size"), self.gba_display.open) {
                    let display_sizes = [
                        (1.0f32, imgui::str!("1x (240 x 160)")),
                        (2.0f32, imgui::str!("2x (480 x 320)")),
                        (3.0f32, imgui::str!("3x (720 x 480)")),
                        (4.0f32, imgui::str!("4x (960 x 640)")),
                    ];

                    for &(scale, label) in display_sizes.iter() {
                        if imgui::menu_item_ex(
                            label,
                            None,
                            (self.gba_display.scale - scale).abs() < std::f32::EPSILON,
                            true,
                        ) {
                            self.gba_display.scale = scale;
                        }
                    }

                    imgui::end_menu();
                }

                if imgui::menu_item_ex(imgui::str!("Stats"), None, self.stats_window.open, true) {
                    self.stats_window.open = !self.stats_window.open;
                }

                if imgui::menu_item_ex(
                    imgui::str!("Demo Window"),
                    None,
                    self.show_demo_window,
                    true,
                ) {
                    self.show_demo_window = !self.show_demo_window;
                }

                imgui::end_menu();
            }

            imgui::end_main_menu_bar();
        }
    }

    pub fn is_gba_display_focused(&self) -> bool {
        self.gba_display.is_focused()
    }

    pub fn set_gba_frame_delay(&mut self, duration: std::time::Duration) {
        self.stats_window.gba_frame_duration = duration;
    }

    pub fn set_gui_frame_delay(&mut self, duration: std::time::Duration) {
        self.stats_window.gui_frame_duration = duration;
    }
}

pub struct GbaDisplayWindow {
    pub open: bool,
    focused: bool,
    pub scale: f32,
}

impl GbaDisplayWindow {
    pub fn new() -> GbaDisplayWindow {
        GbaDisplayWindow {
            open: true,
            focused: false,
            scale: 2.0f32,
        }
    }

    pub fn draw(&mut self, texture: &GbaTexture) {
        let content_size = imgui::vec2(240.0 * self.scale, 160.0 * self.scale);
        imgui::set_next_window_content_size(content_size);
        imgui::push_style_var_vec2(imgui::StyleVar::WindowPadding, imgui::vec2(0.0, 0.0));
        imgui::push_style_var_float(imgui::StyleVar::WindowRounding, 0.0);

        if imgui::begin(
            imgui::str!("Emulator Display"),
            &mut self.open,
            imgui::WindowFlags::AlwaysAutoResize | imgui::WindowFlags::NoScrollbar,
        ) {
            let texture_id: imgui::ImTextureID = texture.get_texture_handle() as _;
            imgui::image(texture_id, content_size);
            self.focused = imgui::is_window_focused(imgui::FocusedFlags::ChildWindows);
        }

        imgui::pop_style_var(2);
        imgui::end();
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }
}

pub struct EmulatorStatsWindow {
    open: bool,

    emu_delay_history: crate::util::circular_buffer::CircularBuffer32<f32>,
    gba_delay_history: crate::util::circular_buffer::CircularBuffer32<f32>,
    gui_delay_history: crate::util::circular_buffer::CircularBuffer32<f32>,

    gba_frame_duration: std::time::Duration,
    gui_frame_duration: std::time::Duration,
}

impl EmulatorStatsWindow {
    pub fn new() -> EmulatorStatsWindow {
        EmulatorStatsWindow {
            open: false,

            emu_delay_history: crate::util::circular_buffer::CircularBuffer32::new(),
            gba_delay_history: crate::util::circular_buffer::CircularBuffer32::new(),
            gui_delay_history: crate::util::circular_buffer::CircularBuffer32::new(),

            gba_frame_duration: std::time::Duration::from_millis(0),
            gui_frame_duration: std::time::Duration::from_millis(0),
        }
    }

    pub fn draw(&mut self) {
        if imgui::begin(
            imgui::str!("Stats"),
            &mut self.open,
            imgui::WindowFlags::AlwaysAutoResize | imgui::WindowFlags::NoScrollbar,
        ) {
            let io = imgui::get_io().expect("NO IO");

            let emu_frame_delay = io.DeltaTime * 1000.0;
            let gba_frame_delay = (self.gba_frame_duration.as_secs_f64() * 1000.0) as f32;
            let gui_frame_delay = (self.gui_frame_duration.as_secs_f64() * 1000.0) as f32;

            // BEGIN EMU FRAME TIMES
            // =====================
            self.emu_delay_history.push_back_overwrite(emu_frame_delay);
            let mut emu_max_frame_delay = std::f32::MIN;
            let mut emu_min_frame_delay = std::f32::MAX;

            let emu_average_frame_delay =
                self.emu_delay_history
                    .get_internal_buffer()
                    .iter()
                    .fold(0.0, |acc, &x| {
                        if x > emu_max_frame_delay {
                            emu_max_frame_delay = x;
                        }
                        if x < emu_min_frame_delay {
                            emu_min_frame_delay = x;
                        }
                        acc + x
                    })
                    / self.emu_delay_history.len() as f32;

            let emu_scale_max = if emu_max_frame_delay > 32.0 {
                emu_max_frame_delay
            } else {
                24.0
            };

            imgui::plot_lines_ex(
                imgui::str!("Emulator Frame Delay"),
                &self.emu_delay_history.get_internal_buffer(),
                self.emu_delay_history.get_internal_head() as i32,
                None,
                0.0,
                emu_scale_max,
                imgui::vec2(0.0, 0.0),
                -1,
            );
            imgui::text(imgui::str_gbuf!(
                "    Average: {:.02} ({:.02} FPS)",
                emu_average_frame_delay,
                1000.0 / emu_average_frame_delay
            ));
            imgui::text(imgui::str_gbuf!(
                "        Min: {:.02} ({:.02} FPS)",
                emu_min_frame_delay,
                1000.0 / emu_min_frame_delay
            ));
            imgui::text(imgui::str_gbuf!(
                "        Max: {:.02} ({:.02} FPS)",
                emu_max_frame_delay,
                1000.0 / emu_max_frame_delay
            ));

            // BEGIN GBA FRAME TIMES
            // =====================
            self.gba_delay_history.push_back_overwrite(gba_frame_delay);
            let mut gba_max_frame_delay = std::f32::MIN;
            let mut gba_min_frame_delay = std::f32::MAX;

            let gba_average_frame_delay =
                self.gba_delay_history
                    .get_internal_buffer()
                    .iter()
                    .fold(0.0, |acc, &x| {
                        if x > gba_max_frame_delay {
                            gba_max_frame_delay = x;
                        }
                        if x < gba_min_frame_delay {
                            gba_min_frame_delay = x;
                        }
                        acc + x
                    })
                    / self.gba_delay_history.len() as f32;

            let gba_scale_max = if emu_max_frame_delay > 32.0 {
                emu_max_frame_delay
            } else {
                18.0
            };

            imgui::plot_lines_ex(
                imgui::str!("GBA Frame Delay"),
                &self.gba_delay_history.get_internal_buffer(),
                self.gba_delay_history.get_internal_head() as i32,
                None,
                0.0,
                gba_scale_max,
                imgui::vec2(0.0, 0.0),
                -1,
            );
            imgui::text(imgui::str_gbuf!(
                "    Average: {:.02} ({:.02} FPS)",
                gba_average_frame_delay,
                1000.0 / gba_average_frame_delay
            ));
            imgui::text(imgui::str_gbuf!(
                "        Min: {:.02} ({:.02} FPS)",
                gba_min_frame_delay,
                1000.0 / gba_min_frame_delay
            ));
            imgui::text(imgui::str_gbuf!(
                "        Max: {:.02} ({:.02} FPS)",
                gba_max_frame_delay,
                1000.0 / gba_max_frame_delay
            ));
            let gba_speed_percentage = ((1000.0 / 60.0) / gba_frame_delay) * 100.0;
            imgui::text(imgui::str_gbuf!(
                "        {:.02}%%  GBA Speed",
                gba_speed_percentage,
            ));

            // BEGIN IMGUI FRAME TIMES
            // =====================
            self.gui_delay_history.push_back_overwrite(gui_frame_delay);
            let mut gui_max_frame_delay = std::f32::MIN;
            let mut gui_min_frame_delay = std::f32::MAX;

            let gui_average_frame_delay =
                self.gui_delay_history
                    .get_internal_buffer()
                    .iter()
                    .fold(0.0, |acc, &x| {
                        if x > gui_max_frame_delay {
                            gui_max_frame_delay = x;
                        }
                        if x < gui_min_frame_delay {
                            gui_min_frame_delay = x;
                        }
                        acc + x
                    })
                    / self.gui_delay_history.len() as f32;

            let gui_scale_max = if emu_max_frame_delay > 32.0 {
                emu_max_frame_delay
            } else {
                18.0
            };

            imgui::plot_lines_ex(
                imgui::str!("ImGUI Frame Delay"),
                &self.gui_delay_history.get_internal_buffer(),
                self.gui_delay_history.get_internal_head() as i32,
                None,
                0.0,
                gui_scale_max,
                imgui::vec2(0.0, 0.0),
                -1,
            );
            imgui::text(imgui::str_gbuf!(
                "    Average: {:.02}",
                gui_average_frame_delay
            ));
            imgui::text(imgui::str_gbuf!("        Min: {:.02}", gui_min_frame_delay));
            imgui::text(imgui::str_gbuf!("        Max: {:.02}", gui_max_frame_delay));
        }
        imgui::end();
    }
}
