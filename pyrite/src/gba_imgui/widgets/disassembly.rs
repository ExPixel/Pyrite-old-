use pyrite_arm::{ ArmCpu, ArmMemory };
use pyrite_arm::disasm::{ disassemble_arm, disassemble_thumb };

const MAX_OPCODE_SIZE: u32 = 44;

#[derive(PartialEq, Eq)]
pub enum DisasmMode {
    Arm,
    Thumb,
    Auto,
}

pub struct DisassemblyWindow {
    /// The number of rows displayed by the widget.
    visible_rows: u32,

    /// The first address being disassembled:
    first_address: u32,

    /// The address that is currently being hilighted.
    cursor_address: u32,

    min_address: u32,
    max_address: u32,
    disasm_buffer: String,
    last_scroll_y: f32,
    address_buffer: [u8; 16],
    ignore_next_scroll_event: bool,
    mode: DisasmMode,
    pub open: bool,
}

impl DisassemblyWindow {
    pub fn new() -> DisassemblyWindow {
        DisassemblyWindow {
            visible_rows: 16,
            first_address: 0,
            cursor_address: 0,
            min_address: 0,
            max_address: 0x0FFFFFFF,
            disasm_buffer: String::new(),
            last_scroll_y: 0.0,
            address_buffer: [0u8; 16],
            ignore_next_scroll_event: false,
            mode: DisasmMode::Auto,
            open: false,
        }
    }

    fn calc_sizes(&self) -> Sizes {
        let mut sizes = Sizes::default();
        let style = imgui::get_style().expect("failed to get imgui style");
        sizes.line_height   = imgui::get_text_line_height();
        sizes.glyph_width   = imgui::calc_text_size(imgui::str!("F"), None, None).x + 1.0;

        sizes.reg_begin     = 0.0;
        sizes.reg_val_begin = sizes.reg_begin + (6.0 * sizes.glyph_width); // CPSR is the longest register name
        sizes.reg_end       = sizes.reg_val_begin + (8.0 * sizes.glyph_width);
        sizes.reg_width     = sizes.reg_end + (style.FramePadding.x + style.ChildBorderSize + style.WindowPadding.x) * 2.0 + style.ScrollbarSize;

        sizes.cursor_begin  = 1.0 * sizes.glyph_width;
        sizes.addr_digits   = 8.0;
        sizes.addr_begin    = sizes.cursor_begin + (sizes.glyph_width * 1.5);
        sizes.opcode_begin  = sizes.addr_begin + (sizes.addr_digits + 1.0) * sizes.glyph_width;
        sizes.disasm_begin  = sizes.opcode_begin + (sizes.addr_digits + 1.0) * sizes.glyph_width;
        sizes.disasm_end    = sizes.disasm_begin + (MAX_OPCODE_SIZE as f32 * sizes.glyph_width);

        sizes.scrollbar_size = style.ScrollbarSize;
        sizes.window_padding = style.WindowPadding.x;

        return sizes;
    }

    pub fn draw(&mut self, cpu: &ArmCpu, memory: &dyn ArmMemory) {
        let sizes = self.calc_sizes();
        imgui::set_next_window_size_constraints(
            imgui::vec2(sizes.reg_width + sizes.disasm_end + sizes.scrollbar_size + sizes.window_padding, 0.0),
            imgui::vec2(sizes.reg_width + sizes.disasm_end + sizes.scrollbar_size + sizes.window_padding, std::f32::MAX),
            None
        );

        if imgui::begin(imgui::str!("Disassembly"), &mut self.open, imgui::WindowFlags::NoScrollbar) {
            self.draw_disassembly_window(&sizes, cpu, memory);
        }
        imgui::end();
    }

    fn draw_disassembly_window(&mut self, sizes: &Sizes, cpu: &ArmCpu, memory: &dyn ArmMemory) {
        let executing_address = cpu.next_exec_address();

        const SCROLLBAR_STEPS: i32 = 10000;

        let addr_buffer = imgui::imstr::ImStrBuf::from_bytes(&mut self.address_buffer);
        imgui::set_next_item_width(sizes.glyph_width * 18.0);
        imgui::input_text(imgui::str!(""), addr_buffer, imgui::none(), None);
        imgui::same_line(0.0);
        imgui::button(imgui::str!("Goto Address"));
        imgui::same_line(0.0);

        let mut cursor_moved = false;
        if imgui::button(imgui::str!("GOTO EXEC")) {
            self.cursor_address = executing_address;
            cursor_moved = true;
        }

        imgui::begin_child(imgui::str!("##scrolling_registers"), imgui::vec2(sizes.reg_width, 0.0), true, imgui::WindowFlags::NoMove);
        for r in 0..10 {
            imgui::text(imgui::str_gbuf!("  R{} = 0x{:08X}", r, cpu.registers.read(r)));
        }
        imgui::text(imgui::str_gbuf!(" R10 = 0x{:08X}", cpu.registers.read(10)));
        imgui::text(imgui::str_gbuf!(" R11 = 0x{:08X}", cpu.registers.read(11)));
        imgui::text(imgui::str_gbuf!(" R12 = 0x{:08X}", cpu.registers.read(12)));
        imgui::text(imgui::str_gbuf!("  SP = 0x{:08X}", cpu.registers.read(13)));
        imgui::text(imgui::str_gbuf!("  LR = 0x{:08X}", cpu.registers.read(14)));
        imgui::text(imgui::str_gbuf!("  PC = 0x{:08X}", cpu.registers.read(15)));
        imgui::text(imgui::str_gbuf!("CPSR = 0x{:08X}", cpu.registers.read_cpsr()));
        imgui::text(imgui::str_gbuf!("SPSR = 0x{:08X}", cpu.registers.read_spsr()));
        imgui::end_child();

        imgui::same_line(0.0);

        imgui::begin_child(imgui::str!("##scrolling_disasm"), imgui::vec2(sizes.disasm_end, 0.0), true, imgui::WindowFlags::NoMove);

        let draw_list = imgui::get_window_draw_list().expect("failed to get window draw list");

        imgui::push_style_var_vec2(imgui::StyleVar::FramePadding, imgui::vec2(0.0, 0.0));
        imgui::push_style_var_vec2(imgui::StyleVar::ItemSpacing, imgui::vec2(0.0, 0.0));

        let mut clipper = imgui::ListClipper::new(SCROLLBAR_STEPS, sizes.line_height);
        let mut clipper_item_start = clipper.DisplayStart;
        let mut clipper_item_end = if clipper.DisplayEnd >= (clipper.DisplayStart.saturating_add(1)) {
            // Truncate the number of rows by one (if there is more than one row) because for some
            // reason ImGui's clipper always displays one row too many and the last one gets cut
            // off.
            clipper.DisplayEnd - 1
        } else {
            clipper.DisplayStart
        };

        let thumb_mode = match self.mode {
            DisasmMode::Arm => false,
            DisasmMode::Thumb => true,
            DisasmMode::Auto => cpu.registers.getf_t(),
        };

        let (instr_size, instr_align) = if thumb_mode {
            (2, 0xFFFFFFFE)
        } else {
            (4, 0xFFFFFFFC)
        };

        if imgui::is_key_pressed(imgui::get_key_index(imgui::Key::DownArrow), true) {
            self.cursor_address = self.cursor_address.wrapping_add(instr_size);

            if self.cursor_address > self.max_address { self.cursor_address = self.min_address; }
            if self.cursor_address < self.min_address { self.cursor_address = self.min_address; }
            self.cursor_address &= instr_align;
            cursor_moved = true;
        } else if imgui::is_key_pressed(imgui::get_key_index(imgui::Key::UpArrow), true) {
            self.cursor_address = self.cursor_address.wrapping_sub(instr_size);

            if self.cursor_address > self.max_address { self.cursor_address = self.min_address; }
            if self.cursor_address < self.min_address { self.cursor_address = self.min_address; }
            self.cursor_address &= instr_align;
            cursor_moved = true;
        }

        self.visible_rows = (clipper_item_end - clipper_item_start) as u32;
        let mut last_address = if self.visible_rows == 0 {
            self.first_address
        } else {
            self.first_address + ((self.visible_rows - 1) * instr_size)
        };

        if cursor_moved { 
            if self.cursor_address <= self.first_address {
                self.first_address = std::cmp::max(self.min_address, self.cursor_address.saturating_sub(instr_size));
            }

            if self.cursor_address >= last_address {
                self.first_address = if self.visible_rows == 0 {
                    std::cmp::max(self.min_address, self.cursor_address.saturating_sub(instr_size))
                } else {
                    let target = self.cursor_address - ((self.visible_rows - 1) * instr_size);
                    std::cmp::min(self.max_address, target.saturating_add(instr_size))
                };
                last_address = self.cursor_address;
            }


            let percentage = self.first_address as f64 / self.max_address as f64;
            let scroll_y = imgui::get_scroll_max_y() as f64 * percentage;
            imgui::set_scroll_y(scroll_y as f32);
            self.ignore_next_scroll_event = true;

            let _ = last_address; // not used for now, but I update it anyway.
        } else if self.ignore_next_scroll_event {
            self.ignore_next_scroll_event = false;
            self.last_scroll_y = imgui::get_scroll_y();
        } else {
            let current_scroll_y = imgui::get_scroll_y();
            if current_scroll_y != self.last_scroll_y {
                let max_scroll_y = imgui::get_scroll_max_y() as f64;
                let address_values = self.max_address - self.min_address;
                let address_values_f = address_values as f64;
                let percentage = current_scroll_y as f64 / max_scroll_y;
                let address_offset_f = address_values_f * percentage;
                let address_offset = if address_offset_f > address_values_f {
                    address_values
                } else {
                    address_offset_f as u32
                };
                clipper_item_start = clipper.DisplayStart;
                clipper_item_end = if clipper.DisplayEnd >= (clipper.DisplayStart.saturating_add(1)) {
                    // Truncate the number of rows by one (if there is more than one row) because for some
                    // reason ImGui's clipper always displays one row too many and the last one gets cut
                    // off.
                    clipper.DisplayEnd - 1
                } else {
                    clipper.DisplayStart
                };
                self.first_address = self.min_address + address_offset;
                last_address = if self.visible_rows == 0 {
                    self.first_address
                } else {
                    self.first_address + ((self.visible_rows - 1) * instr_size)
                };
                self.last_scroll_y = current_scroll_y;

                let _ = last_address; // not used for now, but I update it anyway.
            }
        }

        for clipper_row in clipper_item_start..clipper_item_end {
            let row = (clipper_row - clipper_item_start) as u32;
            let address = self.first_address + (row * instr_size);

            self.disasm_buffer.clear();
            if thumb_mode {
                disassemble_thumb(&mut self.disasm_buffer, address, memory);
            } else {
                disassemble_arm(&mut self.disasm_buffer, address, memory);
            }
            self.disasm_buffer.push('\0');

            let cursor_display_y = imgui::get_cursor_screen_pos().y;

            imgui::set_cursor_pos_x(sizes.addr_begin);
            imgui::text(imgui::str_gbuf!("{:08X}", address));
            imgui::same_line(sizes.opcode_begin);
            if thumb_mode {
                imgui::text(imgui::str_gbuf!("{:04X}", memory.view_halfword(address)));
            } else {
                imgui::text(imgui::str_gbuf!("{:08X}", memory.view_word(address)));
            }
            imgui::same_line(sizes.disasm_begin);
            imgui::text(unsafe {
                imgui::imstr::ImStr::from_bytes_with_nul_unchecked(self.disasm_buffer.as_str().as_bytes())
            });

            if (address == self.cursor_address) | (address == executing_address) {
                let cursor_display_x = imgui::get_window_pos().x + sizes.cursor_begin;

                let cursor_pad_x = sizes.line_height / 8.0;
                let cursor_pad_y = sizes.glyph_width / 8.0;

                let cursor_width    = sizes.glyph_width - (cursor_pad_x * 2.0);
                let cursor_height   = sizes.line_height - (cursor_pad_y * 2.0);

                let cursor_left     = cursor_display_x + cursor_pad_x;
                let cursor_right    = cursor_left + cursor_width;
                let cursor_top      = cursor_display_y + cursor_pad_y;
                let cursor_bottom   = cursor_top + cursor_height;
                let cursor_center_h = cursor_left + (cursor_width / 2.0);
                let cursor_center_v = cursor_top + (cursor_height / 2.0);

                if address == self.cursor_address {
                    draw_list.add_triangle_filled(
                        imgui::vec2(cursor_left, cursor_top),
                        imgui::vec2(cursor_left, cursor_bottom),
                        imgui::vec2(cursor_right, cursor_center_v),
                        imgui::rgb8(252, 196, 25)
                    );
                }

                if address == executing_address {
                    draw_list.add_circle_filled(
                        imgui::vec2(cursor_center_h, cursor_center_v),
                        cursor_width / 2.0,
                        imgui::rgb(0xCC5DE8),
                        None);
                }
            }
        }

        clipper.end();
        imgui::pop_style_var(2);
        imgui::end_child();
    }
}

#[derive(Default)]
struct Sizes {
    reg_begin:      f32,
    reg_val_begin:  f32,
    reg_end:        f32,
    reg_width:      f32,
    addr_digits:    f32,
    line_height:    f32,
    glyph_width:    f32,
    addr_begin:     f32,
    opcode_begin:   f32,
    disasm_begin:   f32,
    disasm_end:     f32,
    scrollbar_size: f32,
    window_padding: f32,
    cursor_begin:   f32,
}
