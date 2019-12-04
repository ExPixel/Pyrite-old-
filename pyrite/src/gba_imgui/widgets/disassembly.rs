use pyrite_arm::{ ArmCpu, ArmMemory };
use pyrite_arm::disasm::{ disassemble_arm, disassemble_thumb };

const MAX_OPCODE_SIZE: u32 = 48;

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
        sizes.opcode_begin  = sizes.addr_begin + (sizes.addr_digits + 2.0) * sizes.glyph_width;
        sizes.opcode_end    = sizes.opcode_begin + (MAX_OPCODE_SIZE as f32 * sizes.glyph_width);
        sizes.disasm_width  = sizes.opcode_end;

        sizes.scrollbar_size = style.ScrollbarSize;
        sizes.window_padding = style.WindowPadding.x;

        return sizes;
    }

    pub fn draw(&mut self, cpu: &ArmCpu, memory: &dyn ArmMemory) {
        let sizes = self.calc_sizes();
        imgui::set_next_window_size_constraints(
            imgui::vec2(sizes.reg_width + sizes.disasm_width + sizes.scrollbar_size + sizes.window_padding, 0.0),
            imgui::vec2(sizes.reg_width + sizes.disasm_width + sizes.scrollbar_size + sizes.window_padding, std::f32::MAX),
            None
        );

        if imgui::begin(imgui::str!("Disassembly"), &mut self.open, imgui::WindowFlags::NoScrollbar) {
            self.draw_disassembly_window(&sizes, cpu, memory);
        }
        imgui::end();
    }

    fn draw_disassembly_window(&mut self, sizes: &Sizes, cpu: &ArmCpu, memory: &dyn ArmMemory) {
        const SCROLLBAR_STEPS: i32 = 10000;

        let addr_buffer = imgui::imstr::ImStrBuf::from_bytes(&mut self.address_buffer);
        imgui::set_next_item_width(sizes.glyph_width * 18.0);
        imgui::input_text(imgui::str!(""), addr_buffer, imgui::none(), None);
        imgui::same_line(0.0);
        imgui::button(imgui::str!("Goto Address"));

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

        imgui::begin_child(imgui::str!("##scrolling_disasm"), imgui::vec2(sizes.disasm_width, 0.0), true, imgui::WindowFlags::NoMove);

        let draw_list = imgui::get_window_draw_list().expect("failed to get window draw list");

        imgui::push_style_var_vec2(imgui::StyleVar::FramePadding, imgui::vec2(0.0, 0.0));
        imgui::push_style_var_vec2(imgui::StyleVar::ItemSpacing, imgui::vec2(0.0, 0.0));

        let mut clipper = imgui::ListClipper::new(SCROLLBAR_STEPS, sizes.line_height);

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
            self.first_address = self.min_address + address_offset;
            self.last_scroll_y = current_scroll_y;
        }

        let thumb_mode = false;
        let instr_size = 4;

        for clipper_row in clipper.DisplayStart..clipper.DisplayEnd {
            let row = (clipper_row - clipper.DisplayStart) as u32;
            let address = self.first_address + (row * instr_size);

            self.disasm_buffer.clear();
            if thumb_mode {
                disassemble_thumb(&mut self.disasm_buffer, address, memory);
            } else {
                disassemble_arm(&mut self.disasm_buffer, address, memory);
            }
            self.disasm_buffer.push('\0');

            let cursor_display_x = sizes.cursor_begin;
            let cursor_display_y = imgui::get_cursor_pos_y();
            imgui::set_cursor_pos_x(sizes.addr_begin);
            imgui::text(imgui::str_gbuf!("{:08X}", address));
            imgui::same_line(sizes.opcode_begin);
            imgui::text(unsafe {
                imgui::imstr::ImStr::from_bytes_with_nul_unchecked(self.disasm_buffer.as_str().as_bytes())
            });

            if address == self.cursor_address {
                let window_pos = imgui::get_window_pos();
                let cursor_pad_x = sizes.line_height / 8.0;
                let cursor_pad_y = sizes.glyph_width / 8.0;

                let cursor_width    = sizes.glyph_width - (cursor_pad_x * 2.0);
                let cursor_height   = sizes.line_height - (cursor_pad_y * 2.0);

                let cursor_left     = window_pos.x + cursor_display_x + cursor_pad_x;
                let cursor_right    = cursor_left + cursor_width;
                let cursor_top      = window_pos.y + cursor_display_y + cursor_pad_y;
                let cursor_bottom   = cursor_top + cursor_height;
                let cursor_center_v = cursor_top + (cursor_height / 2.0);

                draw_list.add_triangle_filled(
                    imgui::vec2(cursor_left, cursor_top),
                    imgui::vec2(cursor_left, cursor_bottom),
                    imgui::vec2(cursor_right, cursor_center_v),
                    imgui::rgb(252, 196, 25)
                );
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
    opcode_end:     f32,
    disasm_width:   f32,
    scrollbar_size: f32,
    window_padding: f32,
    cursor_begin:   f32,
}
