pub mod lvview;
pub mod res;
pub mod vgview;

use color_eyre::Result;

use Constraint::{Length, Min};
use ratatui::style::Stylize;
use ratatui::text::Span;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Text},
    widgets::{
        Block, Borders, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
};

use unicode_width::UnicodeWidthStr;

use crate::{
    lvm::{self},
    lvmapp::{res::Colors, vgview::VgInfoView},
};

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
    LvNew,
}

pub struct LvmApp<'a> {
    state: TableState,
    items: Vec<VgTableData>,
    vgd_longest_item_lens: (u16, u16, u16), // order is (vg_name, pv_name_ lv_name)
    scroll_state: ScrollbarState,
    colors: Colors,
    color_index: usize,
    view_type: ViewType,
    sel_vg_name: String,
    title: String,
    vg_info_view: Option<VgInfoView>,
    lv_new_view: Option<lvview::LvNewView<'a>>,
}

impl<'a> LvmApp<'a> {
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
                // if no empty lv_names remaining, add new row.
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
            0 => res::ITEM_HEIGHT,
            1 => res::ITEM_HEIGHT,
            _ => (vgs.len() - 1) * res::ITEM_HEIGHT,
        };

        Self {
            state: TableState::default()
                .with_selected(0)
                .with_selected_cell((0, 0)),
            vgd_longest_item_lens: constraint_len_calculator(&vgs),
            scroll_state: ScrollbarState::new(initial_cnt_len),
            colors: Colors::new(&res::PALETTES[0]),
            color_index: 0,
            items: vgs,
            view_type: ViewType::VgOverview,
            sel_vg_name: String::new(),
            title: String::from(res::TITLE),
            vg_info_view: None,
            lv_new_view: None,
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
        self.scroll_state = self.scroll_state.position(i * res::ITEM_HEIGHT);
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
        self.scroll_state = self.scroll_state.position(i * res::ITEM_HEIGHT);
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub fn set_colors(&mut self) {
        self.colors = Colors::new(&res::PALETTES[self.color_index]);
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

    // Handle events for the whole app. Also responsible for init of 'views'.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                if self.view_type == ViewType::VgOverview {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Enter => self.acton_cell(),
                            KeyCode::Esc => {
                                // if in main window, quit
                                self.view_type = ViewType::VgOverview;
                                return Ok(());
                            }
                            KeyCode::Down => self.next_row(),
                            KeyCode::Up => self.previous_row(),
                            KeyCode::Right => self.next_column(),
                            KeyCode::Left => self.previous_column(),
                            _ => {}
                        }
                    }
                } else if self.view_type == ViewType::VgInfo {
                    let vg_info_view = self.vg_info_view.as_mut().unwrap();
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => self.view_type = ViewType::VgOverview,
                            KeyCode::Char('j') | KeyCode::Down => vg_info_view.next_lvrow(),
                            KeyCode::Char('k') | KeyCode::Up => vg_info_view.previous_lvrow(),
                            KeyCode::F(7) => {
                                self.view_type = ViewType::LvNew;
                                self.lv_new_view = Some(lvview::LvNewView::new(
                                    &self.sel_vg_name,
                                    vg_info_view.pvdev_list.as_ref().unwrap(),
                                ));
                            }
                            _ => {}
                        }
                    }
                } else if self.view_type == ViewType::LvNew {
                    let lv_new_view = self.lv_new_view.as_mut().unwrap();
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => self.view_type = ViewType::VgInfo, // "back"
                            _ => {
                                lv_new_view.handle_events(&key);
                            }
                        }
                    }
                }
            }
        }
    }

    // Draws widgets and views depending on view type.
    // Data needed (in self) expected to have been initialized beforhand in e.g. run (handleEvent)
    fn draw(&mut self, frame: &mut Frame) {
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(2)]);
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
            let vg_info_layout: [Rect; 3] = inner_layout.areas(outer_layout[0]);
            frame.render_widget(table_block, outer_layout[0]);
            let vg_view = self.vg_info_view.as_mut().unwrap();
            vg_view.render(frame, &vg_info_layout);
        } else if self.view_type == ViewType::LvNew {
            let lv_new_view = self.lv_new_view.as_mut().unwrap();
            frame.render_widget(table_block, outer_layout[0]);
            lv_new_view.render(frame, &outer_layout[0]);
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
        let s1 = Style::new().white().bold();
        let s2 = Style::new()
            .bg(self.colors.infotxt_bg)
            .fg(self.colors.infotxt_fg);
        let esq = Span::from("ESC").style(s1);
        let quit = Span::from("Quit").style(s2);
        let tab = Span::from(" TAB").style(s1);
        let tabtxt = Span::from("Toggle").style(s2);
        let spc = Span::from(" SPACE").style(s1);
        let msec = Span::from("Mark/sel").style(s2);
        let f6 = Span::from(" F6").style(s1);
        let save = Span::from("Save").style(s2);
        let f7 = Span::from(" F7").style(s1);
        let new = Span::from("New").style(s2);

        let line = Line::from(vec![esq, quit, tab, tabtxt, spc, msec, f6, save, f7, new]);

        let info_footer = Paragraph::new(line)
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(Block::default());
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
