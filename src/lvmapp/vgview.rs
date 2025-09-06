use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Gauge, HighlightSpacing, Paragraph, Row,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
};
use Constraint::{Length, Min};

use crate::{
    lvm::{self, LvmLvData, LvmVgData},
    lvmapp::res::{self, Colors},
};

pub struct VgInfoView {
    state: TableState,
    vg_name: String,
    vg_item: Option<LvmVgData>,
    lv_items: Option<Vec<LvmLvData>>,
    scroll_state: ScrollbarState,
    colors: Colors,
}

impl VgInfoView {
    pub fn new(vg_name: &String) -> Self {
        Self {
            vg_name: vg_name.clone(),
            state: TableState::default()
                .with_selected(0)
                .with_selected_cell((0, 0)),
            scroll_state: ScrollbarState::new(15),
            colors: Colors::new(&res::PALETTES[0]),
            vg_item: None,
            lv_items: None,
        }
    }

    pub fn fetch_data(&mut self) {
        self.vg_item = Some(lvm::get_vg_info(&self.vg_name));
        self.lv_items = Some(lvm::get_lvinfo_by_vg(&self.vg_name, &lvm::get_lvs()));
    }

    pub fn render(&mut self, frame: &mut Frame, inner_layout: &[Rect; 3]) {
        let vg_info_layout = Layout::horizontal([Length(30), Min(0)]).horizontal_margin(1);

        let [vg_info_area, gbar_area] = vg_info_layout.areas(inner_layout[0]);
        self.render_vginfo(frame, vg_info_area);
        self.render_vginfo_usagebar(frame, gbar_area);

        self.render_lvs_table(frame, inner_layout[1]);
        self.render_scrollbar(frame, inner_layout[1]);

        self.render_lvs_pvs(frame, inner_layout[2]);
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

    fn render_lvs_pvs(&mut self, frame: &mut Frame, area: Rect) {
        // get the selected lv.
        let i = match self.state.selected() {
            Some(i) => i,
            None => 0,
        };

        let sel_lv_item = self.lv_items.as_ref().unwrap().get(i).unwrap();
        let mut lines = Vec::<Line>::new();
        for seg in &sel_lv_item.lv_segs {
            let line = format!(
                "pvdev={:<10} start_seg={:<10} seg_size={:<10}",
                seg.pvdev, seg.pv_start_pe, seg.size_pe
            );
            lines.push(
                Line::raw(line)
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            );
        }
        // Render a paragraph with details of vg
        let para = Paragraph::new(lines)
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .block(
                Block::default()
                    .title(&*sel_lv_item.lv_name)
                    .border_style(Style::new().fg(self.colors.block_border))
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL),
            );

        frame.render_widget(para, area);
    }

    fn render_vginfo_usagebar(&mut self, frame: &mut Frame, area: Rect) {
        let lvm_vg_data = self.vg_item.as_ref().unwrap();
        let used = lvm_vg_data.size - lvm_vg_data.free;
        let used = (used as f64) / (lvm_vg_data.size as f64) * 100.0;
        let used = used as u16;

        use Constraint::Length;
        let layout = Layout::vertical([Length(5)]).vertical_margin(1);
        let [gbar_area] = layout.areas(area);

        let bar_title = format!(" VG: {} ", self.vg_name);
        // Render a paragraph
        let bar = Gauge::default()
            .block(
                Block::default()
                    .border_style(Style::new().fg(self.colors.block_border))
                    .title(bar_title)
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL),
            )
            .gauge_style(
                Style::new()
                    .fg(self.colors.selected_row_style_fg)
                    .on_black()
                    .italic(),
            )
            .percent(used);
        frame.render_widget(bar, gbar_area);
    }

    fn render_vginfo(&mut self, frame: &mut Frame, area: Rect) {
        let lvm_vg_data = self.vg_item.as_ref().unwrap();
        let header_str = format!("{:<10} {:<20}", "Name", "Value");

        let mut lines = vec![
            Line::raw(header_str)
                .fg(self.colors.header_fg)
                .bg(self.colors.header_bg),
        ];

        let line = format!("{:<10} {:<20}", "VG", lvm_vg_data.name);
        lines.push(Line::raw(line).fg(self.colors.header_fg));
        let line = format!(
            "{:<10} {:<20}",
            "size (g)",
            lvm_vg_data.size / 1000 / 1000 / 1000
        );
        lines.push(Line::raw(line).fg(self.colors.header_fg));
        let line = format!(
            "{:<10} {:<20}",
            "free (g)",
            lvm_vg_data.free / 1000 / 1000 / 1000
        );
        lines.push(Line::raw(line).fg(self.colors.header_fg));

        let used = lvm_vg_data.size - lvm_vg_data.free;
        let used = (used as f64) / (lvm_vg_data.size as f64) * 100.0;

        let line = format!("{:<10} {:<20.1}", "%used", used);
        lines.push(Line::raw(line).fg(self.colors.header_fg));

        let line = format!("{:<10} {:<20}", "pv_count", lvm_vg_data.pv_count);
        lines.push(Line::raw(line).fg(self.colors.header_fg));

        // Render a paragraph with details of vg
        let para = Paragraph::new(lines).style(
            Style::new()
                .fg(self.colors.row_fg)
                .bg(self.colors.buffer_bg),
        );

        frame.render_widget(para, area);
    }

    fn render_lvs_table(&mut self, frame: &mut Frame, area: Rect) {
        // sort lv data,
        self.lv_items
            .as_deref_mut()
            .unwrap()
            .sort_by_key(|item| (item.lv_name).clone());

        let sb = Block::default()
            .border_style(Style::new().fg(self.colors.block_border))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let header = ["LV", "size(g)", "attr", "segtype", "uuid"]
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
                    &size_gb,
                    &data.attr,
                    &data.segtype,
                    &data.uuid,
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
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(11),
                Constraint::Min(40),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style)
        .column_highlight_style(selected_col_style)
        .cell_highlight_style(selected_cell_style)
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
}
