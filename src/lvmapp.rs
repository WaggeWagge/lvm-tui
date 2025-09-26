pub mod lvinfo;
pub mod lvview;
pub mod popup;
pub mod res;
pub mod statusbar;
pub mod vgview;

use core::time;
use crossterm::event::KeyEvent;
use crossterm::style::Print;
use crossterm::{ExecutableCommand, QueueableCommand, terminal};
use ratatui::buffer::Buffer;
use ratatui::widgets::Widget;
use std::io::{self, Write};
use std::sync::Mutex;
use std::thread::{self};
use std::time::Duration;

use Constraint::{Length, Max, Min};
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
        ScrollbarState, Table, TableState, Tabs,
    },
};

use unicode_width::UnicodeWidthStr;

use crate::lvmapp::lvinfo::LvInfoView;
use crate::lvmapp::lvview::LvNewView;
use crate::lvmapp::statusbar::StatusBar;
use crate::{
    lvm::{self},
    lvmapp::{res::Colors, vgview::VgInfoView},
};

const STATUS_RESET_INTERVAL: u64 = 5;

struct Status {
    last_result: Option<String>,
}
impl Status {
    pub fn set_status(&mut self, str: &str) {
        self.last_result = Some(str.to_string());
    }

    pub fn status(&mut self) -> String {
        if self.last_result.is_none() {
            self.last_result = Some(String::from("Ok"));
        }

        self.last_result.clone().unwrap()
    }
}

static STATUS: Mutex<Status> = Mutex::new(Status { last_result: None });

#[derive(Clone, PartialEq)]
#[repr(usize)]
enum SelTabs {
    ALL = 0,
    LV,
    VG,
}

#[derive(Clone)]
struct MainTabs {
    selected: SelTabs,
    colors: Colors,
}

impl MainTabs {
    fn new() -> Self {
        Self {
            colors: Colors::new(&res::PALETTES[0]),
            selected: SelTabs::ALL,
        }
    }

    fn next(&mut self) {
        match self.selected {
            SelTabs::ALL => self.selected = SelTabs::LV,
            SelTabs::LV => self.selected = SelTabs::ALL,
            SelTabs::VG => self.selected = SelTabs::ALL,
        }
    }
}

impl Widget for MainTabs {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let selected: usize = self.selected as usize;
        Tabs::new(vec!["All", "LV", "VG"])
            .highlight_style(
                Style::new()
                    .fg(self.colors.selected_column_style_fg)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .select(Some(selected))
            .padding(" ", " ")
            .divider("|")
            .render(area, buf);
    }
}

pub trait View {
    fn view_type(&self) -> ViewType;
    fn refresh_data(&mut self);
    //
    // handle events related to the view.
    // Return indicates if done in view and can move along, get on with it then...
    // false, stay...
    fn handle_events(&mut self, key: &KeyEvent) -> core::result::Result<bool, &'static str>;
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
pub enum ViewType {
    VgOverview,
    VgInfo,
    LvInfo,
    LvNew,
}

pub struct LvmApp<'a> {
    state: TableState,
    items: Vec<VgTableData>,
    vgd_longest_item_lens: (u16, u16, u16), // order is (vg_name, pv_name_ lv_name)
    scroll_state: ScrollbarState,
    colors: Colors,
    color_index: usize,
    main_tabs: MainTabs,
    view_type: ViewType,
    sel_vg_name: String,
    title: String,
    vg_info_view: Option<VgInfoView>,
    lv_new_view: Option<LvNewView<'a>>,
    refresh_lvm_data: bool,
    lvinfo_view: Option<LvInfoView>,
}

impl View for LvmApp<'_> {
    // For constructed views, refresh data
    // Main view and vgview.
    fn refresh_data(&mut self) {
        let mut vgs = Vec::<VgTableData>::new();
        fetch_data(&mut vgs);
        self.vgd_longest_item_lens = constraint_len_calculator(&vgs);
        self.items = vgs;

        if self.vg_info_view.is_some() {
            self.vg_info_view.as_mut().unwrap().refresh_data();
        }
        if self.lvinfo_view.is_some() {
            self.lvinfo_view.as_mut().unwrap().refresh_data();
        }
        STATUS.lock().unwrap().set_status("Refreshed lvm info.");
    }

    fn view_type(&self) -> ViewType {
        match self.view_type {
            ViewType::VgOverview => ViewType::VgOverview,
            ViewType::VgInfo => ViewType::VgInfo,
            ViewType::LvNew => ViewType::LvNew,
            ViewType::LvInfo => ViewType::LvInfo,
        }
    }

    fn handle_events(&mut self, key: &KeyEvent) -> core::result::Result<bool, &'static str> {
        return self.handle_events(key);
    }
}

impl LvmApp<'_> {
    pub fn new() -> Self {
        let mut vgs = Vec::<VgTableData>::new();
        fetch_data(&mut vgs);

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
            main_tabs: MainTabs::new(),
            items: vgs,
            view_type: ViewType::VgOverview,
            sel_vg_name: String::new(),
            title: String::from(res::TITLE),
            vg_info_view: None,
            lv_new_view: None,
            lvinfo_view: None,
            refresh_lvm_data: true,
        }
    }

    // Handle events for the whole app. Also responsible for init of 'views'.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        // "clear" status bar every STATUS_RESET_INTERVALs.
        thread::spawn(move || {
            loop {
                let dur = time::Duration::from_secs(STATUS_RESET_INTERVAL);
                thread::sleep(dur);
                STATUS
                    .lock()
                    .unwrap()
                    .set_status(format!("Ok({})", STATUS_RESET_INTERVAL).as_str());
                // Send any event to trigger refresh.
                //let mut stdout = io::stdout();

                //match stdout.queue(Print(event::KeyCode::F(5))) {
                //    _ => (), // Noop,
                //};
                //match stdout.flush() {
                //    _ => (), // Noop
                //}
            }
        });

        loop {
            terminal.draw(|frame| self.draw(frame))?;
            self.clear_flags();

            // Dont block forever, do some re-draw in between evens. Wait for at most STATUS_RESET_INTERVAL - 1.
            if event::poll(Duration::from_secs(STATUS_RESET_INTERVAL - 1))? {
                if let Event::Key(key) = event::read()? {
                    match self.handle_events(&key) {
                        Ok(true) => return color_eyre::eyre::Ok(()),
                        Ok(false) => (), // keep going
                        Err(_) => todo!("throw back error..."),
                    }
                }

                if self.refresh_lvm_data {
                    self.refresh_data();
                }
            }
        } // loop
    }

    fn trigger_lvm_refresh(&mut self) {
        self.refresh_lvm_data = true;
    }

    fn clear_flags(&mut self) {
        self.refresh_lvm_data = false;
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
                        self.vg_info_view.as_mut().unwrap().refresh_data();
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

    fn he_vg_overview(&mut self, key: &KeyEvent) -> core::result::Result<bool, &'static str> {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Enter => self.acton_cell(),
                KeyCode::Esc => {
                    // if in main window, quit
                    self.view_type = ViewType::VgOverview;
                    return Ok(true);
                }
                KeyCode::Down => match self.main_tabs.selected {
                    SelTabs::ALL => self.next_row(),
                    SelTabs::LV => self.lvinfo_view.as_mut().unwrap().next_lvrow(),
                    SelTabs::VG => todo!(),
                },
                KeyCode::Up => match self.main_tabs.selected {
                    SelTabs::ALL => self.previous_row(),
                    SelTabs::LV => self.lvinfo_view.as_mut().unwrap().previous_lvrow(),
                    SelTabs::VG => todo!(),
                },
                KeyCode::Right => self.next_column(),
                KeyCode::Left => self.previous_column(),
                KeyCode::Tab => {
                    self.main_tabs.next();
                    if self.main_tabs.selected == SelTabs::LV {
                        // Dont re-create unless needed.
                        if self.lvinfo_view.is_none() {
                            self.lvinfo_view = Some(lvinfo::LvInfoView::new());
                            self.lvinfo_view.as_mut().unwrap().refresh_data();
                        }
                    }
                }
                _ => {}
            }
        }
        return Ok(false);
    }

    fn he_vg_info(&mut self, key: &KeyEvent) -> core::result::Result<bool, &'static str> {
        let vg_info_view = self.vg_info_view.as_mut().unwrap();
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Esc => {
                    self.view_type = ViewType::VgOverview;
                    self.vg_info_view = None;
                    return Ok(false);
                }
                KeyCode::Down => vg_info_view.next_lvrow(),
                KeyCode::Up => vg_info_view.previous_lvrow(),
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
        return Ok(false);
    }

    fn he_lv_new(&mut self, key: &KeyEvent) -> core::result::Result<bool, &'static str> {
        let lv_new_view = self.lv_new_view.as_mut().unwrap();
        if key.kind == KeyEventKind::Press {
            match key.code {
                _ => {
                    match lv_new_view.handle_events(&key) {
                        Ok(true) => {
                            // done,
                            self.view_type = ViewType::VgInfo; // "back"
                            if lv_new_view.lvm_changed() {
                                self.trigger_lvm_refresh();
                            }
                            self.lv_new_view = None;
                        }
                        Ok(false) => (),
                        Err(e) => {
                            // Error handled in view, panic if for some reason get here
                            panic!("{e}");
                        }
                    }
                }
            }
        }

        return Ok(false);
    }

    fn handle_events(&mut self, key: &KeyEvent) -> core::result::Result<bool, &'static str> {
        let mut result = false;

        if self.view_type == ViewType::VgOverview {
            result = self.he_vg_overview(key)?;
        } else if self.view_type == ViewType::VgInfo {
            result = self.he_vg_info(key)?;
        } else if self.view_type == ViewType::LvNew {
            result = self.he_lv_new(key)?;
        }

        return Ok(result);
    }

    // Draws widgets and views depending on view type.
    // Data needed (in self) expected to have been initialized beforhand in e.g. run (handleEvent)
    fn draw(&mut self, frame: &mut Frame) {
        let app_area = frame.area();
        let aab = Block::default()
            .border_style(Style::new().fg(self.colors.block_border))
            .bg(self.colors.buffer_bg)
            .title_top(Line::raw(self.title.to_string()))
            .borders(Borders::ALL);
        frame.render_widget(aab, app_area);

        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(2)])
            .horizontal_margin(1)
            .vertical_margin(1);
        let outer_layout = vertical.split(frame.area());
        self.set_colors();

        let table_block = Block::default().bg(self.colors.buffer_bg);

        // What to draw, ...
        if self.view_type == ViewType::VgOverview {
            // Main View
            // Inner layout for tabs and table
            let inner_layout = &Layout::vertical([Max(1), Min(15)]).margin(1).spacing(1);
            let [tab_area, table_area] = inner_layout.areas(outer_layout[0]);
            frame.render_widget(self.main_tabs.clone(), tab_area);

            // Check tab selected
            match self.main_tabs.selected {
                SelTabs::ALL | SelTabs::VG => {
                    self.render_table(table_block, frame, table_area);
                    self.render_scrollbar(frame, table_area);
                }
                SelTabs::LV => {
                    let lv_info_view = self.lvinfo_view.as_mut().unwrap();
                    frame.render_widget(table_block, table_area);
                    lv_info_view.render(frame, &table_area);
                }
            }
        } else if self.view_type == ViewType::VgInfo {
            // inner layout to hold vginfo
            let inner_layout = &Layout::vertical([Length(8), Min(15), Max(10)]).margin(1);
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
        let w = line.width() as u16;

        let info_footer = Paragraph::new(line)
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(Block::default());

        let layout = Layout::horizontal([Length(w), Min(10)])
            .horizontal_margin(1)
            .spacing(2);

        let [action_area, status_area] = layout.areas(area);

        frame.render_widget(info_footer, action_area);

        let status = STATUS.lock().unwrap().status().clone();
        let sb = StatusBar::new(self.colors.clone()).content(status);
        frame.render_widget(sb, status_area);
    }
}

fn fetch_data(vgs: &mut Vec<VgTableData>) {
    let vg_list = lvm::get_vgs();
    let pv_list = lvm::get_pvs();
    let lv_list = lvm::get_lvs();

    for vg in vg_list {
        let pvs_in_vg: Vec<String> = lvm::find_pvs_by_vg(&vg.name, &pv_list);
        let mut rows = Vec::<VgTableData>::new();

        for pv_name in pvs_in_vg {
            let vg_table_item: VgTableData = VgTableData {
                vg_name: vg.name.clone(),
                pv_name: pv_name.clone(),
                lv_name: String::from(""),
            };
            rows.push(vg_table_item);
        }

        let lvs_in_vg: Vec<String> = lvm::find_lvs_by_vg(&vg.name, &lv_list);
        for lv_name in lvs_in_vg {
            // Go though existing rows, if find space i.e. "", update row,
            // if no empty lv_names remaining, add new row.
            if rows.last().is_none() || !rows.last().unwrap().lv_name.eq("") {
                // Add new
                let row: VgTableData = VgTableData {
                    vg_name: vg.name.clone(),
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
                vg_name: vg.name.clone(),
                pv_name: String::from(""),
                lv_name: String::from(""),
            };
            vgs.push(row);
        } else {
            vgs.append(&mut rows);
        }
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

#[cfg(test)]
mod tests {

    #[test]
    fn something() {
        ////////////////////
    }
}
