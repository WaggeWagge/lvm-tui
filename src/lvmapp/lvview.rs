use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Position, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Padding, Paragraph, Widget},
};
use tui_widget_list::{ListBuilder, ListState, ListView, ScrollAxis};

use Constraint::{Length, Max};

use crate::{
    lvm::LvmExtraArg,
    lvmapp::{
        STATUS, View, ViewType,
        popup::ConfPopup,
        res::{self, Colors},
    },
};

use crate::lvm::{self};

pub struct ListItem {
    text: String,
    style: Style,
}

impl ListItem {
    pub fn new<T: Into<String>>(text: T) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
        }
    }
}

impl Widget for ListItem {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Line::from(self.text).style(self.style).render(area, buf);
    }
}

impl View for LvNewView<'_> {
    fn refresh_data(&mut self) {
        // noop
    }

    fn view_type(&self) -> ViewType {
        return ViewType::LvNew;
    }

    //
    // handle events related to this view. If done here return true, e.g if "back" or "save".
    //
    fn handle_events(&mut self, key: &KeyEvent) -> core::result::Result<bool, &'static str> {
        if key.kind == KeyEventKind::Press {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Tab => self.next_focus(),
                    KeyCode::BackTab => self.prev_focus(),
                    KeyCode::Backspace => self.remove(),
                    KeyCode::Char(' ') => match self.focus {
                        Focus::LvPvAv => self.move_availpv(),
                        Focus::LvPvSel => self.move_selpv(),
                        _ => {}
                    },
                    KeyCode::Char(to_insert) => {
                        let c = to_insert;
                        self.handle_keycode_char(c);
                    }
                    KeyCode::Right => self.right(),
                    KeyCode::Left => self.left(),
                    KeyCode::Down => self.down(),
                    KeyCode::Up => self.up(),
                    KeyCode::F(6) => {
                        self.save_popup();
                    }
                    KeyCode::Esc => {
                        if self.popup_save {
                            // do nothing e.g stay in this view, reset popup flag.
                            STATUS.lock().unwrap().set_status("Lv creation cancelled.");
                            self.popup_save = false;
                        } else {
                            return Ok(true); // Done in this view.
                        }
                    }
                    KeyCode::Enter => {
                        if self.popup_save {
                            self.popup_save = false;
                            // well save/create lv
                            let lv_c_res = self.handle_create_lv();
                            match lv_c_res {
                                Ok(_) => {
                                    self.lvm_changed_flag = true;
                                    let str = String::from("Created LV: ") + &self.lvname.value;
                                    STATUS.lock().unwrap().set_status(str.as_str());
                                    return Ok(true); // Done here
                                }
                                Err(e) => {
                                    STATUS.lock().unwrap().set_status(e);
                                    // Nothing to do but indicate error and press on...
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        return Ok(false);
    }
}

#[derive(PartialEq)]
enum Focus {
    LvName = 0,
    LvSize,
    LvSizeOpt,
    LvSegType,
    SegTypeStripes,
    SegTypeMirrors,
    SegTypeSsize,
    LvPvAv,
    LvPvSel,
}

struct InputField {
    len_max: usize,
    value: String,
    pos: usize,
}
pub struct LvNewView<'a> {
    focus: Focus,
    popup_save: bool,
    vg_name: String,
    lvname: InputField,
    lvsize: InputField,
    lvsize_opt_state: ListState,
    lvsize_opts: [&'a str; 5], // TODO Get rid of this, use constant, forces 'a...
    lvsegtype_state: ListState,
    lvsegtype_opts: [&'a str; 6], // TODO Get rid of this, use constant, forces 'a...
    mirrors: InputField,
    stripes: InputField,
    strips_size: InputField,
    pv_devs_avail: Vec<String>,
    pv_devs_selected: Vec<String>,
    sel_list_state: ListState,
    avail_list_state: ListState,
    colors: Colors,
    lvm_changed_flag: bool,
}

impl<'a> LvNewView<'a> {
    pub fn new(vg_name: &String, pvdev_names: &Vec<String>) -> Self {
        Self {
            focus: Focus::LvName,
            popup_save: false,
            colors: Colors::new(&res::PALETTES[0]),
            lvname: InputField {
                len_max: 25,
                value: String::from(""),
                pos: 0,
            },
            lvsize: InputField {
                len_max: 5,
                value: String::from(""),
                pos: 0,
            },
            lvsize_opt_state: ListState::default(),
            lvsize_opts: ["M", "G", "%FREE", "%VG", "T"],
            sel_list_state: ListState::default(),
            avail_list_state: ListState::default(),
            vg_name: vg_name.clone(),
            lvsegtype_state: ListState::default(),
            pv_devs_avail: pvdev_names.to_vec(),
            pv_devs_selected: Vec::new(),
            lvsegtype_opts: ["linear", "raid0", "raid1", "raid10", "raid5", "raid6"],
            mirrors: InputField {
                len_max: 2,
                value: String::from(""),
                pos: 0,
            },
            stripes: InputField {
                len_max: 6,
                value: String::from(""),
                pos: 0,
            },
            strips_size: InputField {
                len_max: 6,
                value: String::from(""),
                pos: 0,
            },
            lvm_changed_flag: false,
        }
    }

    pub fn lvm_changed(&mut self) -> bool {
        self.lvm_changed_flag
    }

    fn handle_nfocus_pvsel(&mut self) {
        // if nothing selected, no point ...
        if self.pv_devs_selected.len() < 1 {
            self.focus = Focus::LvName;
        } else {
            self.focus = Focus::LvPvSel;
        }
    }

    fn handle_next_pv_selection(&mut self) {
        if self.pv_devs_avail.len() > 0 {
            self.focus = Focus::LvPvAv;
        } else if self.pv_devs_selected.len() > 0 {
            self.focus = Focus::LvPvSel;
        } else {
            self.focus = Focus::LvName;
        }
    }

    fn handle_prev_pv_selection(&mut self) {
        // if nothing select in PvSel no point
        if self.pv_devs_selected.len() > 0 {
            self.focus = Focus::LvPvSel;
        } else if self.pv_devs_avail.len() > 0 {
            self.focus = Focus::LvPvAv;
        } else {
            self.focus = Focus::LvSegType;
        }
    }

    fn prev_segtype_focus(&mut self) {
        let segtype = self.lvsegtype_opts[self.lvsegtype_state.selected.unwrap()];

        match segtype {
            "linear" => self.focus = Focus::LvSegType,
            "raid0" | "raid5" | "raid6" => self.focus = Focus::SegTypeSsize,
            "raid10" => self.focus = Focus::SegTypeMirrors,
            "raid1" => self.focus = Focus::SegTypeMirrors,
            _ => self.focus = Focus::LvSegType,
        }
    }

    fn handle_pfocus_pvsel(&mut self) {
        // if nothing to select in PvAl, no point
        if self.pv_devs_avail.len() < 1 {
            self.prev_segtype_focus();
        } else {
            self.focus = Focus::LvPvAv;
        }
    }

    fn next_focus(&mut self) {
        match self.focus {
            Focus::LvName => self.focus = Focus::LvSize,
            Focus::LvSize => self.focus = Focus::LvSizeOpt,
            Focus::LvSizeOpt => self.focus = Focus::LvSegType,
            Focus::LvSegType => {
                let segtype = self.lvsegtype_opts[self.lvsegtype_state.selected.unwrap()];
                match segtype {
                    "linear" => self.handle_next_pv_selection(),
                    "raid0" | "raid10" | "raid5" | "raid6" => self.focus = Focus::SegTypeStripes,
                    "raid1" => self.focus = Focus::SegTypeMirrors,
                    _ => self.handle_next_pv_selection(),
                }
            }
            Focus::SegTypeStripes => self.focus = Focus::SegTypeSsize,
            Focus::SegTypeSsize => {
                let segtype = self.lvsegtype_opts[self.lvsegtype_state.selected.unwrap()];
                match segtype {
                    "raid10" => self.focus = Focus::SegTypeMirrors,
                    _ => self.handle_next_pv_selection(),
                }
            }
            Focus::SegTypeMirrors => self.handle_next_pv_selection(),
            Focus::LvPvAv => self.handle_nfocus_pvsel(),
            Focus::LvPvSel => self.focus = Focus::LvName,
        }
    }

    fn prev_focus(&mut self) {
        match self.focus {
            Focus::LvName => self.handle_prev_pv_selection(),
            Focus::LvSize => self.focus = Focus::LvName,
            Focus::LvSizeOpt => self.focus = Focus::LvSize,
            Focus::LvSegType => self.focus = Focus::LvSizeOpt,
            Focus::SegTypeStripes => self.focus = Focus::LvSegType,
            Focus::SegTypeSsize => self.focus = Focus::SegTypeStripes,
            Focus::SegTypeMirrors => {
                let segtype = self.lvsegtype_opts[self.lvsegtype_state.selected.unwrap()];
                match segtype {
                    "raid10" => self.focus = Focus::SegTypeSsize,
                    _ => self.focus = Focus::LvSegType,
                }
            }
            Focus::LvPvAv => self.prev_segtype_focus(),
            Focus::LvPvSel => self.handle_pfocus_pvsel(),
        }
    }

    fn insert(&mut self, char: &char) {
        match self.focus {
            Focus::LvName => {
                if self.lvname.value.len() < self.lvname.len_max {
                    self.lvname.value.insert(self.lvname.pos, *char);
                    self.lvname.pos += 1;
                }
            }
            Focus::LvSize => {
                if self.lvsize.value.len() < self.lvsize.len_max {
                    self.lvsize.value.insert(self.lvsize.pos, *char);
                    self.lvsize.pos += 1;
                }
            }
            Focus::SegTypeStripes => {
                if self.stripes.value.len() < self.stripes.len_max {
                    self.stripes.value.insert(self.stripes.pos, *char);
                    self.stripes.pos += 1;
                }
            }
            Focus::SegTypeMirrors => {
                if self.mirrors.value.len() < self.mirrors.len_max {
                    self.mirrors.value.insert(self.mirrors.pos, *char);
                    self.mirrors.pos += 1;
                }
            }
            Focus::SegTypeSsize => {
                if self.strips_size.value.len() < self.strips_size.len_max {
                    self.strips_size.value.insert(self.strips_size.pos, *char);
                    self.strips_size.pos += 1;
                }
            }
            _ => {}
        }
    }

    fn remove(&mut self) {
        match self.focus {
            Focus::LvName => {
                if self.lvname.pos > 0 {
                    self.lvname.value.remove(self.lvname.pos - 1);
                    self.lvname.pos -= 1;
                }
            }
            Focus::LvSize => {
                if self.lvsize.pos > 0 {
                    self.lvsize.value.remove(self.lvsize.pos - 1);
                    self.lvsize.pos -= 1;
                }
            }
            Focus::SegTypeStripes => {
                if self.stripes.pos > 0 {
                    self.stripes.value.remove(self.stripes.pos - 1);
                    self.stripes.pos -= 1;
                }
            }
            Focus::SegTypeSsize => {
                if self.strips_size.pos > 0 {
                    self.strips_size.value.remove(self.strips_size.pos - 1);
                    self.strips_size.pos -= 1;
                }
            }
            Focus::SegTypeMirrors => {
                if self.mirrors.pos > 0 {
                    self.mirrors.value.remove(self.mirrors.pos - 1);
                    self.mirrors.pos -= 1;
                }
            }
            _ => {}
        }
    }

    fn left(&mut self) {
        match self.focus {
            Focus::LvName => {
                if self.lvname.pos > 0 {
                    self.lvname.pos -= 1;
                }
            }
            Focus::LvSize => {
                if self.lvsize.pos > 0 {
                    self.lvsize.pos -= 1;
                }
            }
            Focus::SegTypeStripes => {
                if self.stripes.pos > 0 {
                    self.stripes.pos -= 1;
                }
            }
            Focus::SegTypeMirrors => {
                if self.mirrors.pos > 0 {
                    self.mirrors.pos -= 1;
                }
            }
            Focus::SegTypeSsize => {
                if self.strips_size.pos > 0 {
                    self.strips_size.pos -= 1;
                }
            }
            _ => {}
        }
    }

    fn right(&mut self) {
        match self.focus {
            Focus::LvName => {
                if self.lvname.pos > 0 && self.lvname.pos < (self.lvname.value.len()) {
                    self.lvname.pos += 1;
                }
            }
            Focus::LvSize => {
                if self.lvsize.pos > 0 && self.lvsize.pos < (self.lvsize.value.len()) {
                    self.lvsize.pos += 1;
                }
            }
            Focus::SegTypeStripes => {
                if self.stripes.pos > 0 && self.stripes.pos < (self.stripes.value.len()) {
                    self.stripes.pos += 1;
                }
            }
            Focus::SegTypeMirrors => {
                if self.mirrors.pos > 0 && self.mirrors.pos < (self.mirrors.value.len()) {
                    self.mirrors.pos += 1;
                }
            }
            Focus::SegTypeSsize => {
                if self.strips_size.pos > 0 && self.strips_size.pos < (self.strips_size.value.len())
                {
                    self.strips_size.pos += 1;
                }
            }
            _ => {}
        }
    }

    fn up(&mut self) {
        match self.focus {
            Focus::LvSizeOpt => {
                self.lvsize_opt_state.previous();
            }
            Focus::LvSegType => {
                self.lvsegtype_state.previous();
            }
            _ => {}
        }
    }

    fn down(&mut self) {
        match self.focus {
            Focus::LvSizeOpt => {
                self.lvsize_opt_state.next();
            }
            Focus::LvSegType => {
                self.lvsegtype_state.next();
            }
            _ => {}
        }
    }

    fn save_popup(&mut self) {
        // cause "popup" confim dialog
        self.popup_save = true;
    }

    fn move_availpv(&mut self) {
        if self.avail_list_state.selected.is_none() || self.pv_devs_avail.len() < 1 {
            return;
        }
        let select_index = self.avail_list_state.selected.unwrap();
        // pop/remove from avail list, push to selected.
        let pv_item = self.pv_devs_avail.remove(select_index);
        self.pv_devs_selected.push(pv_item);
    }

    fn move_selpv(&mut self) {
        if self.sel_list_state.selected.is_none() || self.pv_devs_selected.len() < 1 {
            return;
        }
        let select_index = self.sel_list_state.selected.unwrap();
        // pop/remove from avail list, push to selected.
        let pv_item = self.pv_devs_selected.remove(select_index);
        self.pv_devs_avail.push(pv_item);
    }

    fn style_input(&mut self) -> Style {
        Style::new()
            .fg(self.colors.header_bg)
            .underline_color(self.colors.header_bg)
            .add_modifier(Modifier::UNDERLINED)
    }

    fn handle_keycode_char(&mut self, c: char) {
        match self.focus {
            Focus::LvSize => {
                if c >= '0' && c <= '9' || c == '.' {
                    self.insert(&c);
                }
            }
            Focus::SegTypeMirrors | Focus::SegTypeStripes | Focus::SegTypeSsize => {
                if c >= '0' && c <= '9' {
                    self.insert(&c);
                }
            }
            _ => {
                if (c >= 'a' && c <= 'z')
                    || (c >= 'A' && c <= 'Z')
                    || c == '_'
                    || c == '-'
                    || (c >= '0' && c <= '9')
                {
                    self.insert(&c);
                }
            }
        }
    }

    fn handle_create_lv(&mut self) -> Result<String, &'static str> {
        let size = self.lvsize.value.parse::<u64>().unwrap();
        let size_opt = self.lvsize_opts[self.lvsize_opt_state.selected.unwrap()];
        match size_opt {
            "%FREE" => todo!("%FREE"),
            "%VG" => todo!("%VG"),
            _ => (),
        }
        let calc_size_multi = calc_size_multi(&size_opt.to_string());
        let size = size * calc_size_multi;

        let segtype = &self.lvsegtype_opts[self.lvsegtype_state.selected.unwrap()].to_string();
        let lv_name = &self.lvname.value;
        let vg_name = &self.vg_name;
        let mut lvm_extra_args = Vec::<lvm::LvmExtraArg>::new();
        self.populate_extra_opts(segtype, &mut lvm_extra_args);

        return lvm::create_lv(
            lv_name,
            vg_name,
            size,
            segtype,
            &self.pv_devs_selected,
            &lvm_extra_args,
        );
    }

    fn populate_extra_opts(&self, segtype: &String, extra_opts: &mut Vec<lvm::LvmExtraArg>) {
        match segtype.as_str() {
            "linear" => (),
            "raid0" | "raid5" | "raid6" => {
                // --stripes,  --stripesize
                if self.stripes.value.len() > 0 {
                    let stripes = LvmExtraArg {
                        opt: "stripes".to_string(),
                        value: self.stripes.value.clone(),
                    };
                    extra_opts.push(stripes);
                }
                if self.strips_size.value.len() > 0 {
                    let ss = LvmExtraArg {
                        opt: "stripesize".to_string(),
                        value: self.strips_size.value.clone(),
                    };
                    extra_opts.push(ss);
                }
            }
            "raid10" => {
                // --stripes,  --stripesize , --mirrors
                if self.mirrors.value.len() > 0 {
                    let mirrors = LvmExtraArg {
                        opt: "mirrors".to_string(),
                        value: self.mirrors.value.clone(),
                    };
                    extra_opts.push(mirrors);
                }
                if self.strips_size.value.len() > 0 {
                    let ss = LvmExtraArg {
                        opt: "stripesize".to_string(),
                        value: self.strips_size.value.clone(),
                    };
                    extra_opts.push(ss);
                }
                if self.stripes.value.len() > 0 {
                    let stripes = LvmExtraArg {
                        opt: "stripes".to_string(),
                        value: self.stripes.value.clone(),
                    };
                    extra_opts.push(stripes);
                }
            }
            "raid1" => {
                // mirrors,
                if self.mirrors.value.len() > 0 {
                    let mirrors = LvmExtraArg {
                        opt: "mirrors".to_string(),
                        value: self.mirrors.value.clone(),
                    };
                    extra_opts.push(mirrors);
                }
            }
            _ => (),
        }
    }

    fn render_size_opt(&mut self, frame: &mut Frame, rect: &mut Rect) {
        rect.height = 1;
        let list_style = match self.focus {
            // IF we have focus, highlight
            Focus::LvSizeOpt => Style::new()
                .bg(self.colors.header_bg)
                .fg(self.colors.selected_column_style_fg),
            _ => {
                // set rect v size to 1,
                rect.height = 1;
                Style::new()
                    .bg(self.colors.alt_row_color)
                    .fg(self.colors.selected_column_style_fg)
            }
        };

        let builder = ListBuilder::new(|context| {
            let mut item = ListItem::new(self.lvsize_opts[context.index]);
            if context.is_selected {
                item.style = Style::new().fg(self.colors.selected_cell_style_fg);
            }
            let main_axis_size = 1;
            (item, main_axis_size)
        });

        let block = Block::default().padding(Padding::horizontal(1));
        let item_count = self.lvsize_opts.len();
        let list = ListView::new(builder, item_count)
            .scroll_axis(ScrollAxis::Vertical)
            .block(block)
            .infinite_scrolling(true)
            .style(list_style);
        let state = &mut self.lvsize_opt_state;
        if state.selected.is_none() {
            // Default select G
            state.select(Some(1));
        }

        frame.render_stateful_widget(list, *rect, state);
        frame.render_widget(Text::from("▾").style(list_style).right_aligned(), *rect);
    }

    fn render_segtype_opt(&mut self, frame: &mut Frame, rect: &mut Rect) {
        rect.height = 1;
        let list_style = match self.focus {
            // IF we have focus, highlight
            Focus::LvSegType => Style::new()
                .bg(self.colors.header_bg)
                .fg(self.colors.selected_column_style_fg),
            _ => {
                // set rect v size to 1,
                rect.height = 1;
                Style::new()
                    .bg(self.colors.alt_row_color)
                    .fg(self.colors.selected_column_style_fg)
            }
        };

        let builder = ListBuilder::new(|context| {
            let mut item = ListItem::new(self.lvsegtype_opts[context.index]);
            if context.is_selected {
                item.style = Style::new().fg(self.colors.selected_cell_style_fg);
            }
            let main_axis_size = 1;
            (item, main_axis_size)
        });

        let block = Block::default().padding(Padding::horizontal(1));
        let item_count = self.lvsegtype_opts.len();
        let list = ListView::new(builder, item_count)
            .scroll_axis(ScrollAxis::Vertical)
            .block(block)
            .infinite_scrolling(true)
            .style(list_style);
        let state = &mut self.lvsegtype_state;
        if state.selected.is_none() {
            // Default select linear
            state.select(Some(0));
        }

        frame.render_stateful_widget(list, *rect, state);
        frame.render_widget(Text::from("▾").style(list_style).right_aligned(), *rect);
    }

    pub fn render(&mut self, frame: &mut Frame, rect: &Rect) {
        let inner_layout = &Layout::vertical([
            Length(1),
            Length(1),
            Length(1),
            Length(1),
            Length(1),
            Length(3),
            Length(1),
            Length(10),
        ])
        .margin(2);
        let [
            header_area,
            vgname_area,
            lvname_area,
            lvsize_area,
            lvtype_area,
            mut lvtype_options_area,
            pv_sel_label,
            mut pv_sel_area,
        ] = inner_layout.areas(*rect);
        let para_heading = Paragraph::new("CREATE LOGICAL VOLUMNE")
            .alignment(ratatui::layout::Alignment::Left)
            .style(
                Style::new().fg(self.colors.block_border), //.underline_color(self.colors.header_bg)
                                                           //.add_modifier(Modifier::UNDERLINED)
            );
        frame.render_widget(para_heading, header_area);

        let h_layout = &Layout::horizontal([Length(8), Max(26)])
            .horizontal_margin(1)
            .spacing(1);
        let [label_area, val_area] = h_layout.areas(vgname_area);
        let para_vgl = Paragraph::new("vgname:")
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let para_vgv = Paragraph::new(self.vg_name.clone())
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.header_bg));
        frame.render_widget(para_vgl, label_area);
        frame.render_widget(para_vgv, val_area);

        let h_layout = &Layout::horizontal([Length(8), Max(26), Length(4)])
            .horizontal_margin(1)
            .spacing(1);

        let [label_area, input_area, _option_area] = h_layout.areas(lvname_area);
        let para_label = Paragraph::new("lvname:")
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let para_input = Paragraph::new(
            self.lvname
                .value
                .clone()
                .fg(self.colors.selected_column_style_fg),
        )
        .alignment(ratatui::layout::Alignment::Left)
        .style(self.style_input());

        frame.render_widget(para_label, label_area);
        frame.render_widget(para_input, input_area);
        if self.focus == Focus::LvName {
            frame.set_cursor_position(Position::new(
                input_area.x + (self.lvname.pos as u16),
                input_area.y,
            ));
        }

        // Redefine layout for next input row
        let h_layout = &Layout::horizontal([Length(8), Max(8), Length(7)])
            .horizontal_margin(1)
            .spacing(1);
        let [mut label_area, mut input_area, mut option_area] = h_layout.areas(lvsize_area);
        let para_label = Paragraph::new("size:")
            .centered()
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let size_input_text =
            Text::from(self.lvsize.value.clone()).fg(self.colors.selected_column_style_fg);
        let para_input = Paragraph::new(size_input_text)
            .centered()
            .style(self.style_input())
            .alignment(ratatui::layout::Alignment::Left);
        label_area.height = 1;
        input_area.height = 1;
        frame.render_widget(para_label, label_area);
        frame.render_widget(para_input, input_area);
        self.render_size_opt(frame, &mut option_area);
        if self.focus == Focus::LvSize {
            frame.set_cursor_position(Position::new(
                input_area.x + (self.lvsize.pos as u16),
                input_area.y,
            ));
        }

        // Volumne type, linear, raid etc.
        // Redefine layout for next input row
        let h_layout = &Layout::horizontal([Length(8), Length(9)])
            .horizontal_margin(1)
            .spacing(1);
        let [mut label_area, mut option_area] = h_layout.areas(lvtype_area);
        let para_label = Paragraph::new("type:")
            .centered()
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        label_area.height = 1;
        input_area.height = 1;
        frame.render_widget(para_label, label_area);
        self.render_segtype_opt(frame, &mut option_area);

        // Number of devices, depending of selected seg type, stripe/raid0, mirror/raid1, raid5.
        self.draw_segtype_opts(frame, &mut lvtype_options_area);

        let para_sel = Paragraph::new("Select PVs new LV will use (Optional):")
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.block_border));
        frame.render_widget(para_sel, pv_sel_label);
        self.render_pvsel(frame, &mut pv_sel_area);

        if self.popup_save {
            self.render_popup_save(frame, rect);
        }
    }

    fn render_popup_save(&mut self, frame: &mut Frame, rect: &Rect) {
        let popup_area = Rect {
            x: rect.width / 4,
            y: rect.height / 3,
            width: rect.width / 2,
            height: rect.height / 2,
        };
        let title = format!("Create {}", self.lvname.value);
        let s1 = Style::new().white().bold();
        let lvtxt = Span::from(&self.lvname.value).style(s1);
        let vgtxt = Span::from(&self.vg_name).style(s1);
        let segtype = &self.lvsegtype_opts[self.lvsegtype_state.selected.unwrap()].to_string();

        let mut lvm_extra_args = Vec::<lvm::LvmExtraArg>::new();
        self.populate_extra_opts(segtype, &mut lvm_extra_args);
        let mut extra_args_msg = String::new();
        for arg in lvm_extra_args.iter() {
            let msg = format!("{}={} ", arg.opt, arg.value);
            extra_args_msg.push_str(&msg);
        }

        let mut line = Line::from(vec![
            Span::from("You are about to create a new logical volumn '"),
            lvtxt,
            Span::from("' in volumn group '"),
            vgtxt,
            Span::from("'. Segtype '"),
            Span::from(segtype),
        ]);

        if lvm_extra_args.len() > 0 {
            line.push_span(Span::from("'. Extra args: "));
            line.push_span(Span::from(extra_args_msg));
            line.push_span(Span::from(""));
        }

        let popup = ConfPopup::new(Colors::new(&res::PALETTES[0]))
            .content(line.into())
            .title(title);
        frame.render_widget(popup, popup_area);
    }

    //
    // raid0, 5, 6
    //  --stripes Number
    //  --stripesize Size
    //
    fn render_segtype_raid0_5_6(&mut self, frame: &mut Frame, rect: &Rect) {
        let v_layout = &Layout::vertical([Length(1), Length(1)]);

        let [nr_str_area, str_size_area] = v_layout.areas(*rect);

        let h_layout = &Layout::horizontal([
            Length(("stripes/PVs:".len() + 1).try_into().unwrap()),
            Length(5),
        ])
        .horizontal_margin(1)
        .spacing(1);
        let [label_area, input_area] = h_layout.areas(nr_str_area);
        let para_label = Paragraph::new("stripes/PVs:")
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let para_input = Paragraph::new(
            self.stripes
                .value
                .clone()
                .fg(self.colors.selected_column_style_fg),
        )
        .alignment(ratatui::layout::Alignment::Left)
        .style(self.style_input());
        frame.render_widget(para_label, label_area);
        frame.render_widget(para_input, input_area);
        if self.focus == Focus::SegTypeStripes {
            frame.set_cursor_position(Position::new(
                input_area.x + (self.stripes.pos as u16),
                input_area.y,
            ));
        }

        let [label_area, input_area] = h_layout.areas(str_size_area);
        let para_label = Paragraph::new("stripe size:")
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let para_input = Paragraph::new(
            self.strips_size
                .value
                .clone()
                .fg(self.colors.selected_column_style_fg),
        )
        .alignment(ratatui::layout::Alignment::Left)
        .style(self.style_input());
        frame.render_widget(para_label, label_area);
        frame.render_widget(para_input, input_area);
        if self.focus == Focus::SegTypeSsize {
            frame.set_cursor_position(Position::new(
                input_area.x + (self.strips_size.pos as u16),
                input_area.y,
            ));
        }
    }

    // Raid10
    // --mirrors
    // --stripes Number
    // --stripesize Size
    //
    fn render_segtype_raid10(&mut self, frame: &mut Frame, rect: &Rect) {
        let v_layout = &Layout::vertical([Length(1), Length(1), Length(1)]);

        let [nr_str_area, str_size_area, nr_mirrors_area] = v_layout.areas(*rect);

        let h_layout = &Layout::horizontal([
            Length(("stripes/PVs:".len() + 1).try_into().unwrap()),
            Length(5),
        ])
        .horizontal_margin(1)
        .spacing(1);
        let [label_area, input_area] = h_layout.areas(nr_str_area);
        let para_label = Paragraph::new("stripes/PVs:")
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let para_input = Paragraph::new(
            self.stripes
                .value
                .clone()
                .fg(self.colors.selected_column_style_fg),
        )
        .alignment(ratatui::layout::Alignment::Left)
        .style(self.style_input());
        frame.render_widget(para_label, label_area);
        frame.render_widget(para_input, input_area);
        if self.focus == Focus::SegTypeStripes {
            frame.set_cursor_position(Position::new(
                input_area.x + (self.stripes.pos as u16),
                input_area.y,
            ));
        }

        let [label_area, input_area] = h_layout.areas(str_size_area);
        let para_label = Paragraph::new("stripe size:")
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let para_input = Paragraph::new(
            self.strips_size
                .value
                .clone()
                .fg(self.colors.selected_column_style_fg),
        )
        .alignment(ratatui::layout::Alignment::Left)
        .style(self.style_input());
        frame.render_widget(para_label, label_area);
        frame.render_widget(para_input, input_area);
        if self.focus == Focus::SegTypeSsize {
            frame.set_cursor_position(Position::new(
                input_area.x + (self.strips_size.pos as u16),
                input_area.y,
            ));
        }

        let [label_area, input_area] = h_layout.areas(nr_mirrors_area);
        let para_label = Paragraph::new("mirrors:")
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let para_input = Paragraph::new(
            self.mirrors
                .value
                .clone()
                .fg(self.colors.selected_column_style_fg),
        )
        .alignment(ratatui::layout::Alignment::Left)
        .style(self.style_input());
        frame.render_widget(para_label, label_area);
        frame.render_widget(para_input, input_area);
        if self.focus == Focus::SegTypeMirrors {
            frame.set_cursor_position(Position::new(
                input_area.x + (self.mirrors.pos as u16),
                input_area.y,
            ));
        }
    }

    //
    // Mirrors --mirrors Number
    //
    fn render_segtype_mirror(&mut self, frame: &mut Frame, rect: &Rect) {
        let v_layout = &Layout::vertical([Length(1)]);
        let [nr_str_area] = v_layout.areas(*rect);
        let h_layout = &Layout::horizontal([
            Length(("mirrors:".len() + 1).try_into().unwrap()),
            Length(5),
        ])
        .horizontal_margin(1)
        .spacing(1);
        let [label_area, input_area] = h_layout.areas(nr_str_area);

        let para_label = Paragraph::new("mirrors:")
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let para_input = Paragraph::new(
            self.mirrors
                .value
                .clone()
                .fg(self.colors.selected_column_style_fg),
        )
        .alignment(ratatui::layout::Alignment::Left)
        .style(self.style_input());

        frame.render_widget(para_label, label_area);
        frame.render_widget(para_input, input_area);

        if self.focus == Focus::SegTypeMirrors {
            frame.set_cursor_position(Position::new(
                input_area.x + (self.mirrors.pos as u16),
                input_area.y,
            ));
        }
    }

    fn draw_segtype_opts(&mut self, frame: &mut Frame, rect: &mut Rect) {
        let segtype = self.lvsegtype_opts[self.lvsegtype_state.selected.unwrap()];

        match segtype {
            "linear" => rect.height = 0,
            "raid0" => self.render_segtype_raid0_5_6(frame, rect),
            "raid10" => self.render_segtype_raid10(frame, rect),
            "raid5" => self.render_segtype_raid0_5_6(frame, rect),
            "raid6" => self.render_segtype_raid0_5_6(frame, rect),
            "raid1" => self.render_segtype_mirror(frame, rect),
            _ => rect.height = 0,
        }
    }

    // List are rendered based on pv_devs_avail and self.pv_devs_avail.
    fn render_pvsel(&mut self, frame: &mut Frame, rect: &Rect) {
        let h_layout = &Layout::horizontal([Max(15), Max(15)])
            .horizontal_margin(1)
            .spacing(1);
        let [avail_pv_area, sel_pv_area] = h_layout.areas(*rect);

        // Render available list
        let avail_builder = ListBuilder::new(|context| {
            let mut item = ListItem::new(self.pv_devs_avail[context.index].clone());
            let main_axis_size = 1;
            if context.is_selected && self.focus == Focus::LvPvAv {
                item.style = Style::new()
                    .bg(self.colors.header_bg)
                    .fg(self.colors.selected_column_style_fg);
            } else {
                item.style = Style::new().fg(self.colors.selected_column_style_fg);
            }
            (item, main_axis_size)
        });

        let border_style = match self.focus {
            Focus::LvPvAv => Style::new().fg(self.colors.block_border),
            _ => Style::new().fg(self.colors.header_bg),
        };
        let block = Block::bordered()
            .padding(Padding::horizontal(1))
            .border_type(BorderType::Plain)
            .title("available")
            .border_style(border_style);
        let avail_count = self.pv_devs_avail.len();
        let avail_list = ListView::new(avail_builder, avail_count)
            .scroll_axis(ScrollAxis::Vertical)
            .block(block)
            .infinite_scrolling(true);
        let avail_state = &mut self.avail_list_state;
        if avail_state.selected.is_none() {
            avail_state.select(Some(0));
        }

        frame.render_stateful_widget(avail_list, avail_pv_area, avail_state);

        // Render selected pvs.
        let sel_builder = ListBuilder::new(|context| {
            let mut item = ListItem::new(self.pv_devs_selected[context.index].clone());
            if context.is_selected && self.focus == Focus::LvPvSel {
                item.style = Style::new()
                    .bg(self.colors.header_bg)
                    .fg(self.colors.selected_column_style_fg);
            } else {
                item.style = Style::new().fg(self.colors.selected_column_style_fg);
            }
            let main_axis_size = 1;
            (item, main_axis_size)
        });
        let border_style = match self.focus {
            Focus::LvPvSel => Style::new().fg(self.colors.block_border),
            _ => Style::new().fg(self.colors.header_bg),
        };
        let block = Block::bordered()
            .padding(Padding::horizontal(1))
            .border_type(BorderType::Plain)
            .title("selected")
            .border_style(border_style);
        let sel_count = self.pv_devs_selected.len();
        let sel_list = ListView::new(sel_builder, sel_count)
            .scroll_axis(ScrollAxis::Vertical)
            .block(block)
            .infinite_scrolling(true);
        let sel_state = &mut self.sel_list_state;
        if sel_state.selected.is_none() {
            sel_state.select(Some(0));
        }

        frame.render_stateful_widget(sel_list, sel_pv_area, sel_state);
    }
}

//
//  lvsize_opts: ["M", "G", "%FREE", "%VG", "T"],
//
fn calc_size_multi(size_opt: &String) -> u64 {
    match size_opt.as_str() {
        "M" => 1000 * 1000,
        "G" => 1000 * 1000 * 1000,
        "T" => 1000 * 1000 * 1000 * 1000,
        _ => 1000000, // M
    }
}
