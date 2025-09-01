use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    crossterm::event::{ KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Position, Rect},
    style::{Modifier, Style, Stylize},
 
    widgets::{
        Block, BorderType, Borders, Paragraph,
      
    },
};

use Constraint::{Length, Max};

use crate::lvmapp::res::{self, Colors};

#[derive(PartialEq)]
enum Focus {
    LvName = 0,
    LvSize,
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

    lvsegtype: String,
    pv_devs: Vec<String>,
    colors: Colors,
}

impl LvNewView {
    pub fn new(vg_name: &String) -> Self {
        Self {
            focus: Focus::LvName,
            colors: Colors::new(&res::PALETTES[0]),
            lvname: InputField {
                len_max: 14,
                value: String::from(""),
                pos: 0,
            },
            lvsize: InputField {
                len_max: 14,
                value: String::from(""),
                pos: 0,
            },
            pv_devs: Vec::<String>::new(),
            vg_name: vg_name.clone(),
            lvsegtype: String::from(""),
        }
    }

    fn next_focus(&mut self) {
        match self.focus {
            Focus::LvName => self.focus = Focus::LvSize,
            Focus::LvSize => self.focus = Focus::LvName,
        }
    }

    fn prev_focus(&mut self) {
        match self.focus {
            Focus::LvName => self.focus = Focus::LvSize,
            Focus::LvSize => self.focus = Focus::LvName,
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
                        if (c >= 'a' && c <= 'z')
                            || (c >= 'A' && c <= 'Z')
                            || c == '_'
                            || c == '-'
                            || (c >= '0' && c <= '9')
                        {
                            self.insert(&to_insert);
                        }
                    }
                    KeyCode::Right => self.right(),
                    KeyCode::Left => self.left(),
                    _ => {}
                }
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame, rect: &Rect) {
        let inner_layout = &Layout::vertical([Max(1), Max(1), Max(1)]).margin(2);
        let [lvname_area, lvsize_area, yyy] = inner_layout.areas(*rect);
        let h_layout = &Layout::horizontal([Length(10), Max(15)]).horizontal_margin(1);

        let title = self.vg_name.clone() + ": new Logical Volumne ";
        let sb = Block::default()
            .border_style(Style::new().fg(self.colors.block_border))
            .border_type(BorderType::Rounded)
            .title(title)
            .borders(Borders::ALL);

        let [label_area, input_area] = h_layout.areas(lvname_area);
        let para_label = Paragraph::new("lvname:")
            .centered()
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let para_input = Paragraph::new(
            self.lvname
                .value
                .clone()
                .fg(self.colors.selected_column_style_fg),
        )
        .centered()
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

        let [label_area, input_area] = h_layout.areas(lvsize_area);
        let para_label = Paragraph::new("size:")
            .centered()
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::new().fg(self.colors.row_fg));
        let para_input = Paragraph::new(
            self.lvsize
                .value
                .clone()
                .fg(self.colors.selected_column_style_fg),
        )
        .centered()
        .alignment(ratatui::layout::Alignment::Left)
        .style(self.style_input());
        frame.render_widget(para_label, label_area);
        frame.render_widget(para_input, input_area);
        if self.focus == Focus::LvSize {
            frame.set_cursor_position(Position::new(
                input_area.x + (self.lvsize.pos as u16),
                input_area.y,
            ));
        }
    }
}