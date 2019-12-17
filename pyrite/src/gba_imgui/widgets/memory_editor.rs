use pyrite_arm::ArmMemory;

pub type AddrType = u32;
pub const ADDR_MAX: u32 = 0xFFFFFFFF;
// pub const ADDR_MIN: u32 = 0x0;

pub struct MemoryEditorWindow {
    pub open: bool,
    // readonly: bool,
    columns: u32,
    highlight_color: u32,
    opts: MemoryEditorOptions,

    // state
    contents_width_changed: bool,
    cursor_addr: AddrType,
    first_addr: AddrType,
    // data_editing_take_focus: bool,

    // data_input_buf: [u8; 16],
    addr_input_buf: [u8; 16],

    goto_addr: AddrType,
    highlight_min: AddrType,
    highlight_max: AddrType,
    preview_endianess: Endian,
    preview_data_type: DataType,
}

impl MemoryEditorWindow {
    pub fn new() -> MemoryEditorWindow {
        MemoryEditorWindow {
            open: false,
            // readonly: true,
            columns: 16,
            opts: MemoryEditorOptions::default(),
            highlight_color: imgui::rgba8(255, 255, 255, 50),

            cursor_addr: 0,
            first_addr: 0,
            contents_width_changed: false,
            // data_editing_take_focus: false,
            // data_input_buf: [0; 16],
            addr_input_buf: [0; 16],
            goto_addr: 0,
            highlight_min: ADDR_MAX,
            highlight_max: ADDR_MAX,
            preview_endianess: Endian::Little,
            preview_data_type: DataType::S32,
        }
    }
}

impl MemoryEditorWindow {
    // pub fn goto_addr_and_highlight(&mut self, addr_min: AddrType, addr_max: AddrType) {
    //     self.goto_addr = addr_min;
    //     self.highlight_min = addr_min;
    //     self.highlight_max = addr_max;
    // }

    fn calc_sizes(&mut self, sizes: &mut Sizes, mem_size: AddrType, base_display_addr: AddrType) {
        let style = imgui::get_style().expect("failed to get style");
        sizes.addr_digits_count = self.opts.addr_digits_count;
        if sizes.addr_digits_count == 0 {
            let mut n = base_display_addr + mem_size - 1;
            while n > 0 {
                sizes.addr_digits_count += 1;
                n >>= 4;
            }
        }
        sizes.line_height = imgui::get_text_line_height();
        sizes.glyph_width = imgui::calc_text_size(imgui::str!("F"), None, None).x + 1.0; // this assumes the font is mono-space
        sizes.hex_cell_width = (sizes.glyph_width * 2.5).floor();
        sizes.spacing_between_mid_cols = (sizes.hex_cell_width * 0.25).floor();
        sizes.pos_hex_start = (sizes.addr_digits_count + 2) as f32 * sizes.glyph_width;
        sizes.pos_hex_end = sizes.pos_hex_start + (sizes.hex_cell_width * self.columns as f32);
        sizes.pos_ascii_start = sizes.pos_hex_end;
        sizes.pos_ascii_end = sizes.pos_hex_end;

        if self.opts.show_ascii {
            sizes.pos_ascii_start = sizes.pos_hex_end + sizes.glyph_width * 1.0;
            if self.opts.mid_cols_count > 0 {
                sizes.pos_ascii_start += ((self.columns + self.opts.mid_cols_count - 1)
                    / self.opts.mid_cols_count) as f32
                    * sizes.spacing_between_mid_cols;
            }
            sizes.pos_ascii_end = sizes.pos_ascii_start + self.columns as f32 * sizes.glyph_width;
        }

        sizes.window_width = sizes.pos_ascii_end
            + style.ScrollbarSize
            + style.WindowPadding.x * 2.0
            + sizes.glyph_width;
    }

    pub fn draw(
        &mut self,
        memory: &dyn ArmMemory,
        mem_size: AddrType,
        base_display_addr: AddrType,
    ) {
        let mut sizes = Sizes::default();
        self.calc_sizes(&mut sizes, mem_size, base_display_addr);

        imgui::set_next_window_size_constraints(
            imgui::vec2(0.0, 0.0),
            imgui::vec2(sizes.window_width, std::f32::MAX),
            None,
        );

        if imgui::begin(
            imgui::str!("Memory Editor"),
            &mut self.open,
            imgui::WindowFlags::NoScrollbar,
        ) {
            // @TODO implement imgui::is_window_hovered and context menu
            self.draw_contents(memory, mem_size, base_display_addr, &sizes);
            if self.contents_width_changed {
                self.calc_sizes(&mut sizes, mem_size, base_display_addr);
                imgui::set_window_size(
                    imgui::vec2(sizes.window_width, imgui::get_window_size().y),
                    imgui::none(),
                );
            }
        }
        imgui::end();
    }

    fn draw_contents(
        &mut self,
        memory: &dyn ArmMemory,
        mem_size: AddrType,
        base_display_addr: AddrType,
        sizes: &Sizes,
    ) {
        let style = imgui::get_style().expect("failed to get style");

        let height_separator = style.ItemSpacing.x;
        let mut footer_height = 0.0;

        if self.opts.show_options {
            footer_height += height_separator + imgui::get_frame_height_with_spacing() * 1.0;
        }

        if self.opts.show_data_preview {
            footer_height += height_separator
                + imgui::get_frame_height_with_spacing() * 1.0
                + imgui::get_text_line_height_with_spacing() * 3.0;
        }

        if imgui::is_window_focused(imgui::none()) && !imgui::is_any_item_focused() {
            if imgui::is_key_pressed(imgui::get_key_index(imgui::Key::DownArrow), true) {
                self.cursor_addr = self.cursor_addr.wrapping_add(self.columns);
            }
            if imgui::is_key_pressed(imgui::get_key_index(imgui::Key::UpArrow), true) {
                self.cursor_addr = self.cursor_addr.wrapping_sub(self.columns);
            }
            if imgui::is_key_pressed(imgui::get_key_index(imgui::Key::RightArrow), true) {
                self.cursor_addr = self.cursor_addr.wrapping_add(1);
            }
            if imgui::is_key_pressed(imgui::get_key_index(imgui::Key::LeftArrow), true) {
                self.cursor_addr = self.cursor_addr.wrapping_sub(1);
            }

            if self.cursor_addr >= mem_size {
                self.cursor_addr = base_display_addr;
            }
        }

        imgui::begin_child(
            imgui::str!("##memory"),
            imgui::vec2(0.0, -footer_height),
            false,
            imgui::WindowFlags::NoMove | imgui::WindowFlags::NoScrollbar,
        );
        let draw_list = imgui::get_window_draw_list().expect("failed to get window draw list");

        imgui::push_style_var_vec2(imgui::StyleVar::FramePadding, imgui::vec2(0.0, 0.0));
        imgui::push_style_var_vec2(imgui::StyleVar::ItemSpacing, imgui::vec2(0.0, 0.0));

        let mut available_height = imgui::get_window_size().y - sizes.line_height; // not using footer height in here for now
        if available_height < 0.0 {
            available_height = 0.0;
        }
        let visible_rows = (available_height / sizes.line_height).floor() as u32;
        let mut visible_start_addr = self.first_addr.wrapping_sub(self.first_addr % self.columns);
        let mut visible_end_addr = visible_start_addr.wrapping_add(visible_rows * self.columns);

        if self.cursor_addr < visible_start_addr {
            visible_start_addr = self
                .cursor_addr
                .wrapping_sub(self.cursor_addr % self.columns);
            visible_end_addr = visible_start_addr.wrapping_add(visible_rows * self.columns);
            self.first_addr = visible_start_addr;
        } else if self.cursor_addr >= visible_end_addr {
            visible_end_addr = self
                .cursor_addr
                .wrapping_sub(self.cursor_addr % self.columns);
            visible_start_addr = visible_end_addr.wrapping_sub(visible_rows * self.columns);
            self.first_addr = visible_start_addr;
        }

        // let mut data_next = false;

        if self.cursor_addr >= mem_size {
            self.cursor_addr = base_display_addr;
        }

        // let preview_data_type_size = if self.opts.show_data_preview {
        //     self.preview_data_type.size()
        // } else {
        //     0
        // };

        let window_size = imgui::get_window_size();
        let window_pos = imgui::get_window_pos();

        if self.opts.show_ascii {
            // dbg!( window_pos.x + sizes.pos_ascii_start - sizes.glyph_width, window_pos.y );
            // dbg!( window_pos.x + sizes.pos_ascii_start - sizes.glyph_width, window_pos.y + window_size.y );
            // println!("color: {:08X}\n", imgui::get_color_u32(imgui::Col::Border, 1.0));

            draw_list.add_line(
                imgui::vec2(
                    window_pos.x + sizes.pos_ascii_start - sizes.glyph_width,
                    window_pos.y,
                ),
                imgui::vec2(
                    window_pos.x + sizes.pos_ascii_start - sizes.glyph_width,
                    window_pos.y + window_size.y + 16.0,
                ),
                imgui::get_color_u32(imgui::Col::Border, 1.0),
                1.0,
            );
        }

        let color_text = imgui::get_color_u32(imgui::Col::Text, 1.0);
        let color_disabled = if self.opts.grey_out_zeroes {
            imgui::get_color_u32(imgui::Col::TextDisabled, 1.0)
        } else {
            color_text
        };

        let mut row_start_addr = visible_start_addr;
        for _row in 0..=visible_rows {
            if self.opts.uppercase_hex {
                imgui::text(imgui::str_gbuf!(
                    "{:0digits$X}",
                    row_start_addr,
                    digits = sizes.addr_digits_count as usize
                ));
            } else {
                imgui::text(imgui::str_gbuf!(
                    "{:0digits$x}",
                    row_start_addr,
                    digits = sizes.addr_digits_count as usize
                ));
            }

            // Draw Values
            for col in 0..self.columns {
                let addr = row_start_addr.wrapping_add(col);
                let mut byte_pos_x = sizes.pos_hex_start + sizes.hex_cell_width * col as f32;
                if self.opts.mid_cols_count > 0 {
                    byte_pos_x +=
                        (col / self.opts.mid_cols_count) as f32 * sizes.spacing_between_mid_cols;
                }
                imgui::same_line(byte_pos_x);

                // draw hilight
                let is_highlight_from_user_range =
                    addr >= self.highlight_min && addr < self.highlight_max;
                let is_highlight_from_cursor = addr == self.cursor_addr;

                if is_highlight_from_user_range || is_highlight_from_cursor {
                    let pos = imgui::get_cursor_screen_pos();
                    let mut highlight_width = sizes.glyph_width * 2.0;
                    let is_next_byte_highlighted = (addr + 1 < mem_size)
                        && (self.highlight_max != ADDR_MAX && addr + 1 < self.highlight_max);

                    if is_next_byte_highlighted || (col + 1 == self.columns) {
                        highlight_width += sizes.spacing_between_mid_cols;
                    }

                    draw_list.add_rect_filled(
                        pos,
                        imgui::vec2(pos.x + highlight_width, pos.y + sizes.line_height),
                        self.highlight_color,
                    );
                }

                // @TODO implement data editing
                let b = memory.view_byte(addr);
                if self.opts.show_hex_ii {
                    if b >= 32 || b < 128 {
                        imgui::text(imgui::str_gbuf!(".{} ", b as char));
                    } else if b == 0xFF && self.opts.grey_out_zeroes {
                        imgui::text_disabled(imgui::str!("## "));
                    } else if b == 0x00 && self.opts.grey_out_zeroes {
                        imgui::text_disabled(imgui::str!("   "));
                    } else {
                        if self.opts.uppercase_hex {
                            imgui::text(imgui::str_gbuf!("{:02X} ", b));
                        } else {
                            imgui::text(imgui::str_gbuf!("{:02x} ", b));
                        }
                    }
                } else {
                    if b == 0x00 && self.opts.grey_out_zeroes {
                        imgui::text_disabled(imgui::str!("00 "));
                    } else {
                        if self.opts.uppercase_hex {
                            imgui::text(imgui::str_gbuf!("{:02X} ", b));
                        } else {
                            imgui::text(imgui::str_gbuf!("{:02x} ", b));
                        }
                    }
                }
            }

            if self.opts.show_ascii {
                imgui::same_line(sizes.pos_ascii_start);
                let mut pos = imgui::get_cursor_screen_pos();
                imgui::new_line();

                for col in 0..self.columns {
                    let addr = row_start_addr.wrapping_add(col);
                    let mut buf = [memory.view_byte(addr), 0];
                    let color;
                    if buf[0] < 32 || buf[0] >= 128 {
                        color = color_disabled;
                        buf[0] = b'.';
                    } else {
                        color = color_text;
                    }

                    draw_list.add_text(pos, color, unsafe {
                        imgui::imstr::ImStr::from_bytes_with_nul_unchecked(&buf)
                    });
                    pos.x += sizes.glyph_width;
                }
            }
            row_start_addr = row_start_addr.wrapping_add(self.columns);
        }

        imgui::pop_style_var(2);
        imgui::end_child();

        let mut next_show_data_preview = self.opts.show_data_preview;
        if self.opts.show_options {
            imgui::separator();

            if imgui::button(imgui::str!("Options")) {
                imgui::open_popup(imgui::str!("context"))
            }

            if imgui::begin_popup(imgui::str!("context"), imgui::none()) {
                imgui::push_item_width(56.0);
                let mut columns = self.columns as i32;
                if imgui::drag_int(
                    imgui::str!("##cols"),
                    &mut columns,
                    Some(0.2),
                    Some(4),
                    Some(32),
                    Some(imgui::str!("%d cols")),
                ) {
                    self.columns = std::cmp::max(columns, 0) as u32;
                    self.contents_width_changed = true;
                }
                imgui::pop_item_width();
                imgui::checkbox(
                    imgui::str!("Show Data Preview"),
                    &mut next_show_data_preview,
                );
                imgui::checkbox(imgui::str!("Show HexII"), &mut self.opts.show_hex_ii);
                if imgui::checkbox(imgui::str!("Show ASCII"), &mut self.opts.show_ascii) {
                    self.contents_width_changed = true;
                }
                imgui::checkbox(
                    imgui::str!("Grey out zeroes"),
                    &mut self.opts.grey_out_zeroes,
                );
                imgui::checkbox(imgui::str!("Uppercase Hex"), &mut self.opts.uppercase_hex);
                imgui::end_popup();
            }

            imgui::same_line(0.0);

            if self.opts.uppercase_hex {
                imgui::text(imgui::str_gbuf!(
                    "{:0digits$X} ... {:0digits$X}",
                    visible_start_addr,
                    visible_end_addr,
                    digits = sizes.addr_digits_count as usize
                ));
            } else {
                imgui::text(imgui::str_gbuf!(
                    "{:0digits$x} ... {:0digits$x}",
                    visible_start_addr,
                    visible_end_addr,
                    digits = sizes.addr_digits_count as usize
                ));
            }

            imgui::same_line(0.0);
            imgui::push_item_width(
                (sizes.addr_digits_count + 1) as f32 * sizes.glyph_width
                    + style.FramePadding.x * 2.0,
            );
            {
                let addr_input_buf = imgui::imstr::ImStrBuf::from_bytes(&mut self.addr_input_buf);
                if imgui::input_text(
                    imgui::str!("##addr"),
                    addr_input_buf,
                    imgui::InputTextFlags::CharsHexadecimal
                        | imgui::InputTextFlags::EnterReturnsTrue,
                    None,
                ) {
                    if let Ok(Ok(goto_addr)) =
                        addr_input_buf.to_str().map(|s| u32::from_str_radix(s, 16))
                    {
                        self.goto_addr = goto_addr;
                        self.highlight_min = ADDR_MAX;
                        self.highlight_max = ADDR_MAX;
                    }
                }
            }
            imgui::pop_item_width();

            if self.goto_addr != ADDR_MAX {
                if self.goto_addr < mem_size {
                    self.cursor_addr = self.goto_addr;
                }
                self.goto_addr = ADDR_MAX;
            }
        }

        if self.opts.show_data_preview {
            imgui::separator();
            imgui::align_text_to_frame_padding();
            imgui::text(imgui::str!("Preview as:"));
            imgui::same_line(0.0);
            imgui::push_item_width(
                (sizes.glyph_width * 10.0) + style.FramePadding.x * 2.0 + style.ItemInnerSpacing.x,
            );

            if imgui::begin_combo(
                imgui::str!("##combo_type"),
                self.preview_data_type.name(),
                imgui::ComboFlags::HeightLargest,
            ) {
                for data_type in DataType::VARIANTS.iter() {
                    if imgui::selectable(
                        data_type.name(),
                        self.preview_data_type == *data_type,
                        imgui::none(),
                        None,
                    ) {
                        self.preview_data_type = *data_type;
                    }
                }
                imgui::end_combo();
            }
            imgui::pop_item_width();

            imgui::same_line(0.0);
            imgui::push_item_width(
                (sizes.glyph_width * 6.0) + style.FramePadding.x * 2.0 + style.ItemInnerSpacing.x,
            );
            if imgui::begin_combo(
                imgui::str!("##combo_endianess"),
                self.preview_endianess.name(),
                imgui::ComboFlags::HeightLargest,
            ) {
                if imgui::selectable(
                    Endian::Little.name(),
                    self.preview_endianess == Endian::Little,
                    imgui::none(),
                    None,
                ) {
                    self.preview_endianess = Endian::Little;
                }
                if imgui::selectable(
                    Endian::Big.name(),
                    self.preview_endianess == Endian::Big,
                    imgui::none(),
                    None,
                ) {
                    self.preview_endianess = Endian::Big;
                }
                imgui::end_combo();
            }
            imgui::pop_item_width();

            let x = sizes.glyph_width * 6.0;

            imgui::text(imgui::str!("Dec"));
            imgui::same_line(x);
            Self::text_data_as_dec(
                self.cursor_addr,
                self.preview_data_type,
                self.preview_endianess == Endian::Big,
                memory,
            );

            imgui::text(imgui::str!("Hex"));
            imgui::same_line(x);
            Self::text_data_as_hex(
                self.cursor_addr,
                self.preview_data_type,
                self.preview_endianess == Endian::Big,
                memory,
            );

            imgui::text(imgui::str!("Bin"));
            imgui::same_line(x);
            Self::text_data_as_bin(
                self.cursor_addr,
                self.preview_data_type,
                self.preview_endianess == Endian::Big,
                memory,
            );
        }

        self.opts.show_data_preview = next_show_data_preview;
        imgui::set_cursor_pos_x(sizes.window_width);
    }

    fn text_data_as_dec(addr: u32, data_type: DataType, big_endian: bool, memory: &dyn ArmMemory) {
        macro_rules! set_endian {
            ($Value:expr) => {
                if big_endian {
                    $Value.swap_bytes()
                } else {
                    $Value
                }
            };
        }

        match data_type {
            DataType::S8 => imgui::text_unformatted(imgui::str_gbuf!(
                "{}",
                set_endian!(memory.view_byte(addr) as i8)
            )),
            DataType::U8 => {
                imgui::text_unformatted(imgui::str_gbuf!("{}", set_endian!(memory.view_byte(addr))))
            }
            DataType::S16 => imgui::text_unformatted(imgui::str_gbuf!(
                "{}",
                set_endian!(memory.view_halfword(addr) as i16)
            )),
            DataType::U16 => imgui::text_unformatted(imgui::str_gbuf!(
                "{}",
                set_endian!(memory.view_halfword(addr))
            )),
            DataType::S32 => imgui::text_unformatted(imgui::str_gbuf!(
                "{}",
                set_endian!(memory.view_word(addr) as i32)
            )),
            DataType::U32 => {
                imgui::text_unformatted(imgui::str_gbuf!("{}", set_endian!(memory.view_word(addr))))
            }

            DataType::S64 => {
                let lo = memory.view_word(addr);
                let hi = memory.view_word(addr.wrapping_add(4));
                let v = set_endian!((lo as i64) | ((hi as i64) << 32));
                imgui::text_unformatted(imgui::str_gbuf!("{}", v))
            }
            DataType::U64 => {
                let lo = memory.view_word(addr);
                let hi = memory.view_word(addr.wrapping_add(4));
                let v = set_endian!((lo as u64) | ((hi as u64) << 32));
                imgui::text_unformatted(imgui::str_gbuf!("{}", v))
            }
            DataType::Float => imgui::text_unformatted(imgui::str_gbuf!("{:+e}", unsafe {
                std::mem::transmute::<u32, f32>(set_endian!(memory.view_word(addr)))
            })),
            DataType::Double => {
                let lo = memory.view_word(addr);
                let hi = memory.view_word(addr.wrapping_add(4));
                let v = unsafe {
                    std::mem::transmute::<u64, f64>(set_endian!((lo as u64) | ((hi as u64) << 32)))
                };
                imgui::text_unformatted(imgui::str_gbuf!("{:+e}", v))
            }
        }
    }

    fn text_data_as_hex(addr: u32, data_type: DataType, big_endian: bool, memory: &dyn ArmMemory) {
        macro_rules! set_endian {
            ($Value:expr) => {
                if big_endian {
                    $Value.swap_bytes()
                } else {
                    $Value
                }
            };
        }

        match data_type {
            DataType::S8 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:02X}",
                set_endian!(memory.view_byte(addr) as i8)
            )),
            DataType::U8 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:02X}",
                set_endian!(memory.view_byte(addr))
            )),
            DataType::S16 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:04X}",
                set_endian!(memory.view_halfword(addr) as i16)
            )),
            DataType::U16 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:04X}",
                set_endian!(memory.view_halfword(addr))
            )),
            DataType::S32 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:08X}",
                set_endian!(memory.view_word(addr) as i32)
            )),
            DataType::U32 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:08X}",
                set_endian!(memory.view_word(addr))
            )),

            DataType::S64 => {
                let lo = memory.view_word(addr);
                let hi = memory.view_word(addr.wrapping_add(4));
                let v = set_endian!((lo as i64) | ((hi as i64) << 32));
                imgui::text_unformatted(imgui::str_gbuf!("{:016X}", v))
            }
            DataType::U64 => {
                let lo = memory.view_word(addr);
                let hi = memory.view_word(addr.wrapping_add(4));
                let v = set_endian!((lo as u64) | ((hi as u64) << 32));
                imgui::text_unformatted(imgui::str_gbuf!("{:016X}", v))
            }
            DataType::Float => imgui::text_unformatted(imgui::str_gbuf!(
                "{:08X}",
                set_endian!(memory.view_word(addr) as i32)
            )),
            DataType::Double => {
                let lo = memory.view_word(addr);
                let hi = memory.view_word(addr.wrapping_add(4));
                let v = set_endian!((lo as u64) | ((hi as u64) << 32));
                imgui::text_unformatted(imgui::str_gbuf!("{:016X}", v))
            }
        }
    }

    fn text_data_as_bin(addr: u32, data_type: DataType, big_endian: bool, memory: &dyn ArmMemory) {
        macro_rules! set_endian {
            ($Value:expr) => {
                if big_endian {
                    $Value.swap_bytes()
                } else {
                    $Value
                }
            };
        }

        match data_type {
            DataType::S8 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:08b}",
                set_endian!(memory.view_byte(addr) as i8)
            )),
            DataType::U8 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:08b}",
                set_endian!(memory.view_byte(addr))
            )),
            DataType::S16 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:016b}",
                set_endian!(memory.view_halfword(addr) as i16)
            )),
            DataType::U16 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:016b}",
                set_endian!(memory.view_halfword(addr))
            )),
            DataType::S32 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:032b}",
                set_endian!(memory.view_word(addr) as i32)
            )),
            DataType::U32 => imgui::text_unformatted(imgui::str_gbuf!(
                "{:032b}",
                set_endian!(memory.view_word(addr))
            )),

            DataType::S64 => {
                let lo = memory.view_word(addr);
                let hi = memory.view_word(addr.wrapping_add(4));
                let v = set_endian!((lo as i64) | ((hi as i64) << 32));
                imgui::text_unformatted(imgui::str_gbuf!("{:064b}", v))
            }
            DataType::U64 => {
                let lo = memory.view_word(addr);
                let hi = memory.view_word(addr.wrapping_add(4));
                let v = set_endian!((lo as u64) | ((hi as u64) << 32));
                imgui::text_unformatted(imgui::str_gbuf!("{:064b}", v))
            }
            DataType::Float => imgui::text_unformatted(imgui::str_gbuf!(
                "{:032b}",
                set_endian!(memory.view_word(addr) as i32)
            )),
            DataType::Double => {
                let lo = memory.view_word(addr);
                let hi = memory.view_word(addr.wrapping_add(4));
                let v = set_endian!((lo as u64) | ((hi as u64) << 32));
                imgui::text_unformatted(imgui::str_gbuf!("{:064b}", v))
            }
        }
    }
}

#[derive(Default)]
struct Sizes {
    addr_digits_count: u32,
    line_height: f32,
    glyph_width: f32,
    hex_cell_width: f32,
    pos_hex_start: f32,
    pos_hex_end: f32,
    pos_ascii_start: f32,
    pos_ascii_end: f32,
    window_width: f32,
    spacing_between_mid_cols: f32,
}

pub struct MemoryEditorOptions {
    show_options: bool,
    show_data_preview: bool,
    show_hex_ii: bool,
    show_ascii: bool,
    grey_out_zeroes: bool,
    uppercase_hex: bool,
    mid_cols_count: u32,
    addr_digits_count: u32,
}

impl Default for MemoryEditorOptions {
    fn default() -> MemoryEditorOptions {
        MemoryEditorOptions {
            show_options: true,
            show_data_preview: false,
            show_hex_ii: false,
            show_ascii: true,
            grey_out_zeroes: true,
            uppercase_hex: true,
            mid_cols_count: 8,
            addr_digits_count: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
    pub fn name(self) -> &'static imgui::imstr::ImStr {
        match self {
            Endian::Little => imgui::str!("LE"),
            Endian::Big => imgui::str!("BE"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    U64,
    Float,
    Double,
}

impl DataType {
    pub const COUNT: usize = 10;
    pub const VARIANTS: [DataType; DataType::COUNT] = [
        DataType::S8,
        DataType::U8,
        DataType::S16,
        DataType::U16,
        DataType::S32,
        DataType::U32,
        DataType::S64,
        DataType::U64,
        DataType::Float,
        DataType::Double,
    ];

    // pub fn size(self) -> usize {
    //     match self {
    //         DataType::S8 => 1,
    //         DataType::U8 => 1,
    //         DataType::S16 => 2,
    //         DataType::U16 => 2,
    //         DataType::S32 => 4,
    //         DataType::U32 => 4,
    //         DataType::S64 => 8,
    //         DataType::U64 => 8,
    //         DataType::Float => 4,
    //         DataType::Double => 8,
    //     }
    // }

    pub fn name(self) -> &'static imgui::imstr::ImStr {
        match self {
            DataType::S8 => imgui::str!("Int8"),
            DataType::U8 => imgui::str!("UInt8"),
            DataType::S16 => imgui::str!("Int16"),
            DataType::U16 => imgui::str!("UInt16"),
            DataType::S32 => imgui::str!("Int32"),
            DataType::U32 => imgui::str!("UInt32"),
            DataType::S64 => imgui::str!("Int64"),
            DataType::U64 => imgui::str!("UInt64"),
            DataType::Float => imgui::str!("Float"),
            DataType::Double => imgui::str!("Double"),
        }
    }
}
