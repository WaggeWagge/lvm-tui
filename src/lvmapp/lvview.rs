use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Position, Rect},
    style::{Modifier, Style, Stylize},    
    text::{Line, Text},
    widgets::{Block, BorderType, Padding, Paragraph, Widget, Wrap},
};
use tui_widget_list::{ListBuilder, ListState, ListView, ScrollAxis};

use Constraint::{Length, Max, Min};

use crate::lvmapp::res::{self, Colors};

pub const C_LVM_INFO_TEXT: [&str; 1] =
    ["info: (TAB) toggle fields | (Enter) create | (ESQ|q) cancel"];

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

#[derive(PartialEq)]
enum Focus {
    LvName = 0,
    LvSize,
    LvSizeOpt,
    LvSegType,
}

struct InputField {
    len_max: usize,
    value: String,
    pos: usize,
}
pub struct LvNewView {
    focus: Focus,
    vg_name: String,
    lvname: InputField,
    lvsize: InputField,
    lvsize_opt_state: ListState,
    lvsegtype_state: ListState,    
    pv_devs: Vec<String>,
    colors: Colors,
}

impl LvNewView {
    pub fn new(vg_name: &String) -> Self {
        Self {
            focus: Focus::LvName,
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
            pv_devs: Vec::<String>::new(),
            vg_name: vg_name.clone(),
            lvsegtype_state: ListState::default(),
        }
    }

    fn next_focus(&mut self) {
        match self.focus {
            Focus::LvName => self.focus = Focus::LvSize,
            Focus::LvSize => self.focus = Focus::LvSizeOpt,
            Focus::LvSizeOpt => self.focus = Focus::LvSegType,
            Focus::LvSegType => self.focus = Focus::LvName,
        }
    }

    fn prev_focus(&mut self) {
        match self.focus {
            Focus::LvName => self.focus = Focus::LvSegType,
            Focus::LvSize => self.focus = Focus::LvName,
            Focus::LvSizeOpt => self.focus = Focus::LvSize,
            Focus::LvSegType => self.focus = Focus::LvSizeOpt,
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

    fn style_input(&mut self) -> Style {
        Style::new()
            .fg(self.colors.header_bg)
            .underline_color(self.colors.header_bg)
            .add_modifier(Modifier::UNDERLINED)
    }

    pub fn handle_events(&mut self, key: &KeyEvent) {
        if key.kind == KeyEventKind::Press {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Tab => self.next_focus(),
                    KeyCode::BackTab => self.prev_focus(),
                    KeyCode::Backspace => self.remove(),
                    KeyCode::Char(to_insert) => {
                        let c = to_insert;
                        match self.focus {
                            Focus::LvSize => {
                                if c >= '0' && c <= '9' {
                                    self.insert(&to_insert);
                                }
                            }
                            _ => {
                                if (c >= 'a' && c <= 'z')
                                    || (c >= 'A' && c <= 'Z')
                                    || c == '_'
                                    || c == '-'
                                    || (c >= '0' && c <= '9')
                                {
                                    self.insert(&to_insert);
                                }
                            }
                        }
                    }
                    KeyCode::Right => self.right(),
                    KeyCode::Left => self.left(),
                    KeyCode::Down => self.down(),
                    KeyCode::Up => self.up(),
                    _ => {}
                }
            }
        }
    }

    fn render_size_opt(&mut self, frame: &mut Frame, rect: &mut Rect) {
        let size_opts = ["M", "G", "T"];
        rect.height = 1;
        let list_style = match self.focus {
            // IF we have focus, highlight
            Focus::LvSizeOpt => Style::new().bg(self.colors.header_bg).fg(self.colors.selected_column_style_fg),
            _ => {
                // set rect v size to 1,
                rect.height = 1;
                Style::new()
                    .bg(self.colors.alt_row_color)
                    .fg(self.colors.selected_column_style_fg)
            }
        };

        let builder = ListBuilder::new(|context| {
            let mut item = ListItem::new(size_opts[context.index]);
            if context.is_selected {
                item.style = Style::new().fg(self.colors.selected_cell_style_fg);
            }
            let main_axis_size = 1;
            (item, main_axis_size)
        });

        let block = Block::default().padding(Padding::horizontal(1));
        let item_count = size_opts.len();
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
        let segtype_opts = ["linear", "raid0", "raid1", "raid5"];
        rect.height = 1;
        let list_style = match self.focus {
            // IF we have focus, highlight
            Focus::LvSegType => {                
                Style::new().bg(self.colors.header_bg).fg(self.colors.selected_column_style_fg)              
            }
            _ => {
                // set rect v size to 1,
                rect.height = 1;
                Style::new()
                    .bg(self.colors.alt_row_color)
                    .fg(self.colors.selected_column_style_fg)
            }
        };

        let builder = ListBuilder::new(|context| {
            let mut item = ListItem::new(segtype_opts[context.index]);
            if context.is_selected {
                item.style = Style::new().fg(self.colors.selected_cell_style_fg);
            }
            let main_axis_size = 1;
            (item, main_axis_size)
        });

        let block = Block::default().padding(Padding::horizontal(1));
        let item_count = segtype_opts.len();
        let list = ListView::new(builder, item_count)
            .scroll_axis(ScrollAxis::Vertical)
            .block(block)
            .infinite_scrolling(true)
            .style(list_style);
        let state = &mut self.lvsegtype_state;
        if state.selected.is_none() {
            // Default select G
            state.select(Some(1));
        }

        frame.render_stateful_widget(list, *rect, state);
        frame.render_widget(Text::from("▾").style(list_style).right_aligned(), *rect);
    }


    fn render_info(&mut self, frame: &mut Frame, rect: &Rect) {
        let block = Block::default();
        let para_label = Paragraph::new(Text::from_iter(C_LVM_INFO_TEXT))
            .block(block)
            .alignment(ratatui::layout::Alignment::Left)
            .wrap(Wrap { trim: false })
            .style(Style::new().fg(self.colors.row_fg));

        frame.render_widget(para_label, *rect);
    }

    pub fn render(&mut self, frame: &mut Frame, rect: &Rect) {
        let inner_layout = &Layout::vertical([
            Length(1),
            Max(1),
            Max(1),
            Max(2),
            Length(1),
            Length(10),
            Max(2),
        ])        
        .margin(2);
        let [
            header_area,
            lvname_area,
            lvsize_area,
            lvtype_area,
            pv_sel_label,
            mut pv_sel_area,
            info_area,
        ] = inner_layout.areas(*rect);
        let para_heading = Paragraph::new("CREATE LOGICAL VOLUMNE")
            .alignment(ratatui::layout::Alignment::Left)
            .style(
                Style::new().fg(self.colors.block_border), //.underline_color(self.colors.header_bg)
                                                           //.add_modifier(Modifier::UNDERLINED)
            );
        frame.render_widget(para_heading, header_area);

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
        let h_layout = &Layout::horizontal([Length(8), Max(8), Length(4)])
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

        // Volumne type, linear, raid etc.
        // Redefine layout for next input row
        let h_layout = &Layout::horizontal([Length(8), Length(10)])
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

        let para_sel = Paragraph::new("Select PVs (O):")
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.block_border));
        frame.render_widget(para_sel, pv_sel_label);
        self.render_pvsel(frame, &mut pv_sel_area);

        self.render_info(frame, &info_area);
        if self.focus == Focus::LvSize {
            frame.set_cursor_position(Position::new(
                input_area.x + (self.lvsize.pos as u16),
                input_area.y,
            ));
        }
    }

    fn render_pvsel(&mut self, frame: &mut Frame, rect: &Rect) {
        let h_layout = &Layout::horizontal([Max(15), Max(15)])
            .horizontal_margin(1)
            .spacing(1);
        let [sel_pv_area, avail_pv_area] = h_layout.areas(*rect);

        let para_sel = Paragraph::new("xxx1:")
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(self.colors.header_bg)),
            )
            .style(Style::new().fg(self.colors.row_fg));
        frame.render_widget(para_sel, sel_pv_area);

        let para_avail = Paragraph::new("xxx2:")
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(self.colors.header_bg)),
            )
            .style(Style::new().fg(self.colors.row_fg));
        frame.render_widget(para_avail, avail_pv_area);
    }
}
