use std::rc::Rc;

use Constraint::{Length, Min};
use color_eyre::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{self, Constraint, Direction, Layout, Margin, Rect},
    style::{self, Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Gauge, HighlightSpacing, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
};
use style::palette::tailwind;
use unicode_width::UnicodeWidthStr;

use crate::lvm::{self, LvmLvData, LvmVgData};

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::CYAN,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::SLATE,
];
const INFO_TEXT: [&str; 1] =
    ["(Esc) quit | (↑) move up | (↓) move down | (←) move left | (→) move right"];

const TITLE: &str = " LVM-TUI ";

const ITEM_HEIGHT: usize = 1;

struct TableColors {
    block_border: Color,
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            block_border: color.c400,
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

struct VgTableData {
    vg_name: String,
    pv_name: String,
    lv_name: String,
}

impl VgTableData {
    const fn ref_array(&self) -> [&String; 3] {
        [&self.vg_name, &self.pv_name, &self.lv_name]
    }
    fn vg_name(&self) -> &str {
        &self.vg_name
    }
    fn pv_name(&self) -> &str {
        &self.pv_name
    }
}

#[derive(PartialEq)]
enum ViewType {
    VgOverview,
    VgInfo,
}

struct VgInfoView {
    state: TableState,
    vg_name: String,
    vg_item: Option<LvmVgData>,
    lv_items: Option<Vec<LvmLvData>>,
    scroll_state: ScrollbarState,
    colors: TableColors,
}

impl VgInfoView {
    fn new(vg_name: &String) -> Self {
        Self {
            vg_name: vg_name.clone(),
            state: TableState::default()
                .with_selected(0)
                .with_selected_cell((0, 0)),
            scroll_state: ScrollbarState::new(15),
            colors: TableColors::new(&PALETTES[0]),
            vg_item: None,
            lv_items: None,
        }
    }

    fn fetch_data(&mut self) {
        self.vg_item = Some(lvm::get_vg_info(&self.vg_name));
        self.lv_items = Some(lvm::get_lvinfo_by_vg(&self.vg_name, &lvm::get_lvs()));
    }

    fn render(&mut self, frame: &mut Frame, inner_layout: &[Rect; 3]) {
        let vg_info_layout = Layout::horizontal([Length(30), Min(0)]).horizontal_margin(1);

        let [vg_info_area, gbar_area] = vg_info_layout.areas(inner_layout[0]);
        self.render_vginfo(frame, vg_info_area);
        self.render_vginfo_usagebar(frame, gbar_area);

        self.render_lvs_table(frame, inner_layout[1]);
        self.render_scrollbar(frame, inner_layout[1]);
        let sb = Block::default()
            .border_style(Style::new().fg(self.colors.block_border))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);
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
        for seg in &sel_lv_item.lvSegs {
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
                    &data.segtype,
                    &data.attr,
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
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
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
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }
}

pub struct LvmApp {
    state: TableState,
    items: Vec<VgTableData>,
    vgd_longest_item_lens: (u16, u16, u16), // order is (vg_name, pv_name_ lv_name)
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
    view_type: ViewType,
    sel_vg_name: String,
    title: String,
    vg_info_view: Option<VgInfoView>,
}

impl LvmApp {
    pub fn new() -> Self {
        let vg_list = lvm::get_vgs();
        let pv_list = lvm::get_pvs();
        let lv_list = lvm::get_lvs();

        let mut vgs = Vec::<VgTableData>::new();

        for vg_name in vg_list {
            let pvs_in_vg: Vec<String> = lvm::find_pvs_by_vg(&vg_name, &pv_list);
            let mut rows = Vec::<VgTableData>::new();

            for pv_name in pvs_in_vg {
                let vg_table_item: VgTableData = VgTableData {
                    vg_name: vg_name.clone(),
                    pv_name: pv_name.clone(),
                    lv_name: String::from(""),
                };
                rows.push(vg_table_item);
            }

            let lvs_in_vg: Vec<String> = lvm::find_lvs_by_vg(&vg_name, &lv_list);
            for lv_name in lvs_in_vg {
                // Go though existing rows, if find space i.e. "", update row,
                // if no empty lv_names ramaining, add new row.
                if !rows.last().unwrap().lv_name.eq("") {
                    // Add new
                    let row: VgTableData = VgTableData {
                        vg_name: vg_name.clone(),
                        pv_name: String::from(""),
                        lv_name: lv_name.clone(),
                    };
                    rows.push(row);
                } else {
                    // Update existing
                    for row in rows.iter_mut() {
                        if row.lv_name.eq("") {
                            row.lv_name = lv_name.clone();
                            break;
                        }
                    }
                }
            }

            // If no match, put row with vgname only
            if rows.len() < 1 {
                let row: VgTableData = VgTableData {
                    vg_name: vg_name.clone(),
                    pv_name: String::from(""),
                    lv_name: String::from(""),
                };
                vgs.push(row);
            } else {
                vgs.append(&mut rows);
            }
        }

        let initial_cnt_len = match vgs.len() {
            // dont * with 0
            0 => ITEM_HEIGHT,
            1 => ITEM_HEIGHT,
            _ => (vgs.len() - 1) * ITEM_HEIGHT,
        };

        Self {
            state: TableState::default()
                .with_selected(0)
                .with_selected_cell((0, 0)),
            vgd_longest_item_lens: constraint_len_calculator(&vgs),
            scroll_state: ScrollbarState::new(initial_cnt_len),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            items: vgs,
            view_type: ViewType::VgOverview,
            sel_vg_name: String::new(),
            title: String::from(TITLE),
            vg_info_view: None,
        }
    }

    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }

    pub fn acton_cell(&mut self) {
        if self.view_type == ViewType::VgOverview {
            let ic: (usize, usize) = match self.state.selected_cell() {
                Some(ic) => ic,
                None => (0, 0),
            };

            let item: &VgTableData = self.items.get(ic.0).expect("Unexpected error");
            match ic.1 {
                0 => {
                    if !item.vg_name.eq("") {
                        // if cell is "", nothing to act on
                        self.view_type = ViewType::VgInfo;
                        self.sel_vg_name = item.vg_name.clone();
                        self.vg_info_view = Some(VgInfoView::new(&self.sel_vg_name));
                        self.vg_info_view.as_mut().unwrap().fetch_data();
                    }
                    item.vg_name.clone()
                }
                1 => {
                    self.view_type = ViewType::VgOverview;
                    item.pv_name.clone()
                }
                2 => {
                    self.view_type = ViewType::VgOverview;
                    item.lv_name.clone()
                }
                _ => "".to_string(),
            };
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                if self.view_type == ViewType::VgOverview {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Enter => self.acton_cell(),
                            KeyCode::Char('q') | KeyCode::Esc => {
                                // if in main window, quit
                                self.view_type = ViewType::VgOverview;
                                return Ok(());
                            }
                            KeyCode::Char('j') | KeyCode::Down => self.next_row(),
                            KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
                            KeyCode::Char('l') | KeyCode::Right => self.next_column(),
                            KeyCode::Char('h') | KeyCode::Left => self.previous_column(),
                            _ => {}
                        }
                    }
                } else if self.view_type == ViewType::VgInfo {
                    let vg_info_view = self.vg_info_view.as_mut().unwrap();
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                self.view_type = ViewType::VgOverview
                            }
                            KeyCode::Char('j') | KeyCode::Down => vg_info_view.next_lvrow(),
                            KeyCode::Char('k') | KeyCode::Up => vg_info_view.previous_lvrow(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(3)]);
        let outer_layout = vertical.split(frame.area());
        self.set_colors();

        let table_block = Block::default()
            .border_style(Style::new().fg(self.colors.block_border))
            .bg(self.colors.buffer_bg)
            .title_top(Line::raw(self.title.to_string()))
            .borders(Borders::ALL);

        // What to draw, ...
        if self.view_type == ViewType::VgOverview {
            self.render_table(table_block, frame, outer_layout[0]);
            self.render_scrollbar(frame, outer_layout[0]);
        } else if self.view_type == ViewType::VgInfo {
            // inner layout to hold vginfo
            let inner_layout = &Layout::vertical([Length(8), Min(15), Min(5)]).margin(1);
            //let inner_layout = &Layout::default()
            //    .direction(Direction::Vertical)
            //    .constraints(vec![Constraint::Length(8), Constraint::Min(10)])
            //    .margin(1)
            //    .split(outer_layout[0]);
            let vg_info_layout: [Rect; 3] = inner_layout.areas(outer_layout[0]);
            frame.render_widget(table_block, outer_layout[0]);
            let vg_view = self.vg_info_view.as_mut().unwrap();
            vg_view.render(frame, &vg_info_layout);
        }
        self.render_footer(frame, outer_layout[1]);
    }

    fn render_table(&mut self, sb: Block, frame: &mut Frame, area: Rect) {
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

        let header = ["vg_name", "pv_name", "lv_name"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);
        let rows = self.items.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };
            let item = data.ref_array();
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("{content}"))))
                .collect::<Row>()
                .style(Style::new().fg(self.colors.row_fg).bg(color))
                .height(1)
        });
        let bar = " █ ";
        // Set sane min values
        if self.vgd_longest_item_lens.0 < 10 {
            self.vgd_longest_item_lens.0 = 10 + 1;
        } // +1 paddong
        if self.vgd_longest_item_lens.1 < 10 {
            self.vgd_longest_item_lens.0 = 10;
        }
        if self.vgd_longest_item_lens.2 < 10 {
            self.vgd_longest_item_lens.0 = 10;
        }
        let t = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(self.vgd_longest_item_lens.0 + 1),
                Constraint::Min(self.vgd_longest_item_lens.1),
                Constraint::Min(self.vgd_longest_item_lens.2),
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

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Text::from_iter(INFO_TEXT))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            );
        frame.render_widget(info_footer, area);
    }
}

fn constraint_len_calculator(items: &[VgTableData]) -> (u16, u16, u16) {
    // If 0 number of items return sane defaul i.e. 10 for min width
    //if items.len() < 1 {
    //    return (10, 10, 10)
    //}

    let vgname_len = items
        .iter()
        .map(VgTableData::vg_name)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let pvname_len = items
        .iter()
        .map(VgTableData::pv_name)
        .flat_map(str::lines)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let lvname_len = items
        .iter()
        .map(VgTableData::pv_name)
        .flat_map(str::lines)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (vgname_len as u16, pvname_len as u16, lvname_len as u16)
}
