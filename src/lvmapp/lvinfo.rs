use Constraint::{Length, Min};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style, Stylize},
    text::Text,
    widgets::{
        Block, Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
};

use crate::{
    lvm::{self, LvmLvData},
    lvmapp::{
        View, ViewType,
        res::{self, Colors},
    },
};

pub struct LvInfoView {
    state: TableState,
    lv_items: Option<Vec<LvmLvData>>,
    scroll_state: ScrollbarState,
    colors: Colors,
}

impl View for LvInfoView {
    fn refresh_data(&mut self) {
        self.lv_items = Some(lvm::get_lvs());
    }

    fn view_type(&self) -> ViewType {
        return ViewType::LvInfo;
    }

    //
    // handle events related to this view. If done here return true, e.g if "back" or "save".
    //
    fn handle_events(&mut self, key: &KeyEvent) -> core::result::Result<bool, &'static str> {
        if key.kind == KeyEventKind::Press {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc => {
                        return Ok(true); // Done in this view.                        
                    }
                    KeyCode::Down => self.next_lvrow(),
                    KeyCode::Up => self.previous_lvrow(),
                    _ => {}
                }
            }
        }

        return Ok(false);
    }
}

impl LvInfoView {
    pub fn new() -> Self {
        Self {
            state: TableState::default()
                .with_selected(0)
                .with_selected_cell((0, 0)),
            scroll_state: ScrollbarState::new(15),
            colors: Colors::new(&res::PALETTES[0]),
            lv_items: None,
        }
    }

    pub fn next_lvrow(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.lv_items.as_ref().unwrap().len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * res::ITEM_HEIGHT);
    }

    pub fn previous_lvrow(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.lv_items.as_ref().unwrap().len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * res::ITEM_HEIGHT);
    }

    pub fn render(&mut self, frame: &mut Frame, area: &Rect) {
        let v_outer_layout = &Layout::vertical([Min(8), Length(8)]);
        let [v_area0, v_area1] = v_outer_layout.areas(*area);

        let h_layout = Layout::horizontal([Length(30), Min(0)]).horizontal_margin(1);

        let [h_area0, h_area1] = h_layout.areas(v_area0);

        self.render_lvs_table(frame, v_area0);
        self.render_scrollbar(frame, v_area0);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    fn render_lvs_table(&mut self, frame: &mut Frame, area: Rect) {
        // sort lv data,
        self.lv_items
            .as_deref_mut()
            .unwrap()
            .sort_by_key(|item| (item.lv_name).clone());

        let sb = Block::default().border_style(Style::new().fg(self.colors.block_border));
        //.border_type(BorderType::Rounded)
        //.borders(Borders::ALL);
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);

        let header = ["LV", "VG", "size(g)", "attr", "segtype"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);
        let rows = self
            .lv_items
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, data)| {
                let color = match i % 2 {
                    0 => self.colors.normal_row_color,
                    _ => self.colors.alt_row_color,
                };
                let gb_conv = 1000.0 * 1000.0 * 1000.0;
                let size_gb = (data.size as f64) / gb_conv;
                let size_gb = format!("{:.2}", size_gb);
                let item: [&str; 5] = [
                    &data.lv_name,
                    &data.vg_name,
                    &size_gb,
                    &data.attr,
                    &data.segtype,                    
                ];
                item.into_iter()
                    .map(|content| Cell::from(Text::from(format!("{content}"))))
                    .collect::<Row>()
                    .style(Style::new().fg(self.colors.row_fg).bg(color))
                    .height(1)
            });
        let bar = " █ ";

        let t = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Min(20),
                Constraint::Length(15),
                Constraint::Length(8),
                Constraint::Length(11),
                Constraint::Min(11),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style)
        .highlight_symbol(Text::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .bg(self.colors.buffer_bg)
        .block(sb)
        .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(t, area, &mut self.state);
    }
}
