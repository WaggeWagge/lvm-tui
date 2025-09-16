pub mod lvview;
pub mod popup;
pub mod res;
pub mod statusbar;
pub mod vgview;

use color_eyre::Result;

use std::collections::HashMap;
use std::sync::Mutex;

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
        ScrollbarState, Table, TableState,
    },
};

use unicode_width::UnicodeWidthStr;

use crate::lvmapp::statusbar::StatusBar;
use crate::{
    lvm::{self},
    lvmapp::{res::Colors, vgview::VgInfoView},
};

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

/// An event type.
#[derive(PartialEq, Eq, Hash, Clone)]
pub enum LvmEvent {
    LVCreated,
    LVDeleted,
    LVMGenUpdate,
}

pub trait Observer {
    //
    // Inform provider we have changed state.
    //
    fn notify_provider(&self)
    where
        Self: Sized;
    //
    // Invoked when provider informs about state change.
    //
    fn state_changed(&self);
 
}

pub trait Provider<'a> {
    //
    // Observers will be notified of state_change, when
    // notify_change is invoked.
    //
    fn register<T: Observer>(&'a mut self, e_type: LvmEvent, o: &'a T)
    where
        T: Observer + 'static,  Self: Sized;
    //
    // Inform of a change.
    //
    fn notify_change(&self, e_type: LvmEvent);

    //
    // Unsub...
    //
    fn unregister<T: Observer>(&mut self, e_type: LvmEvent, o: T)
    where
        T: Observer + 'static,
        Self: Sized;
}

#[derive(Default)]
pub struct LvmMonitorProvider<'a> {    
    events: HashMap<LvmEvent, Vec<Box<&'a dyn Observer>>>,
}

impl <'a>Provider<'a>  for LvmMonitorProvider<'a>  {
    fn register<T: Observer>(&'a mut self, e_type: LvmEvent, obs: &'a T)    
    {
        self.events.entry(e_type.clone()).or_default();
        self.events.get_mut(&e_type).unwrap().push(Box::new(obs));
    }

    fn notify_change(&self, e_type: LvmEvent) {
        let obs = self.events.get(&e_type).unwrap();
        for o in obs.iter() {
            o.state_changed();
        }
    }

    fn unregister<T: Observer>(&mut self, _e_type: LvmEvent, _usub: T)
    where
        T: Observer + 'static,
    {
        //    self.events.get_mut(&e_type)
        //        .unwrap()
        //        .retain(|&x| x != usub);
        // Need to implement/hash the observer somehow.
        todo!("Implement");
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
                } else if self.view_type == ViewType::LvNew {
                    let lv_new_view = self.lv_new_view.as_mut().unwrap();
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            _ => {
                                match lv_new_view.handle_events(&key) {
                                    Ok(true) => {
                                        // done,
                                        self.view_type = ViewType::VgInfo; // "back"
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
                }
            }
        }
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
            // Inner layout for table
            let inner_layout = &Layout::vertical([Min(15)]).margin(1);
            let [table_area] = inner_layout.areas(outer_layout[0]);
            self.render_table(table_block, frame, table_area);
            self.render_scrollbar(frame, table_area);
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
    use crate::lvmapp::{LvmEvent, LvmMonitorProvider, Observer, Provider};

    pub struct TestObserver  {    
        //provider :  &'a dyn Provider,
        pub state : bool,
    }

    impl <'a>TestObserver{
        //  fn register<T: Observer>(&mut self, e_type: LvmEvent, o: T)
        //pub fn  new<T: Provider>(p: &'a T) -> Self
        //where
        //T: Provider + 'static,
        //{ 
        //    Self {
        //        provider: p,
        //        state: false,
        //    }
        //}

         pub fn new() -> Self { 
            Self {                
                state: false,
            }
        }
    }

    impl Observer for TestObserver {
        fn notify_provider(&self)
        where
            Self: Sized {
            //self.provider.notify_change(super::LvmEvent::LVCreated);
        }
    
        fn state_changed(&self) {
            println!("Got state_changed notification");
        }
    }

    #[derive(Default)]
    pub struct App<'a> {
        provider: LvmMonitorProvider<'a>,
    }

    impl <'a> App <'a>{
        pub fn events(&'a mut self) -> &'a mut LvmMonitorProvider<'a> {
            &mut self.provider
        }
    }

    #[test]
    fn test_provider() {
        let mut app = App::default();

        let observer = TestObserver::new();
        let observer2 = TestObserver::new();
        
        app.events().register(LvmEvent::LVCreated, &observer);
        
        {
            app.events().register(LvmEvent::LVCreated, &observer2);
        }
        
        observer.notify_provider();

        //provider.notify_change(LvmEvent::LVCreated);
    }
}