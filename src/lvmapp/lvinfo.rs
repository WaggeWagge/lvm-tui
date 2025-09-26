use Constraint::{Length, Min};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect}, style::{Style, Stylize}, widgets::{
        Block, BorderType, Borders, Gauge, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, TableState
    }, Frame
};

use crate::{
    lvm::{self, LvmLvData, LvmVgData},
    lvmapp::{
        View, ViewType,
        res::{self, Colors},
    },
};

pub struct LvInfoView {
    state: TableState,
    lv_selected_index: usize,
    lvs_items_rndr_start: usize, // what items to render and scrollbar
    lvs_item_rndr_end: usize,    // what items to render and scrollbar
    lv_items: Option<Vec<LvmLvData>>,
    vg_items: Option<Vec<LvmVgData>>,
    scroll_state: ScrollbarState,
    colors: Colors,
}

const ITEM_HEIGHT: u16 = 3;

impl View for LvInfoView {
    fn refresh_data(&mut self) {
        self.lv_items = Some(lvm::get_lvs());
        self.vg_items = Some(lvm::get_vgs());
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
            vg_items: None,
            lvs_items_rndr_start: 0,
            lvs_item_rndr_end: 0,
            lv_selected_index: 0,
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
        self.lv_selected_index = i;

        if self.lvs_item_rndr_end < self.lv_items.as_ref().unwrap().len() {
            // we did not render all....
            self.lvs_items_rndr_start += 1;
          
        }
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT as usize);
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

        self.lv_selected_index = i;

        if self.lvs_items_rndr_start > 0 {
            self.lvs_items_rndr_start -= 1;
          
        }

        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT as usize);
        self.state.select(Some(i));
    }

    pub fn render(&mut self, frame: &mut Frame, area: &Rect) {
        let v_outer_layout = &Layout::vertical([ Length(1), Min(8)]);
        let [v_area0, v_area1] = v_outer_layout.areas(*area);

        let h_layout = Layout::horizontal([Min(20), Length(2)]).horizontal_margin(1);

        let [h_area0, _h_area1] = h_layout.areas(v_area0);

        self.render_lvs_list(frame, v_area1);
        self.render_scrollbar(frame, v_area1);

        let p = Paragraph::new(format!("sorted by: vgname"));
        frame.render_widget(p,h_area0);
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

    fn lv_index_is_sel(&self, index: usize) -> bool {
        if index == self.lv_selected_index {
            true
        } else {
            false
        }
    }

    fn render_lvs_list(&mut self, frame: &mut Frame, area: Rect) {
        // sort lv data,
        self.lv_items
            .as_deref_mut()
            .unwrap()
            .sort_by_key(|item| (item.vg_name).clone());

        let v = self.lv_items.as_ref().unwrap().clone();

        let mut item_to_rndr: usize = self.lvs_items_rndr_start;
        let mut i = 0;

        while item_to_rndr < v.len() {
            let lv_data = v.get(item_to_rndr).unwrap();
            // Get relevant vg.
            let mut vg: Option<LvmVgData> = None;
            for vg_data in self.vg_items.as_ref().unwrap().iter() {
                if vg_data.name.eq(&lv_data.vg_name) {
                    vg = Some(vg_data.clone());
                }
            }

            let mut percent: f64 = 0.0;
            if vg.is_some() {
                percent = (lv_data.size as f64) / (vg.as_ref().unwrap().size as f64) * 100.0;
            }
            let percent = percent as u16;

            let border_style = match self.lv_index_is_sel(item_to_rndr) {
                true => Style::new().fg(self.colors.block_border),
                false => Style::new().fg(self.colors.header_bg),
            };

            let y = area.y + i;
            let lv_area = Rect::new(area.x, y, area.width - 3, 3);
            let bar = Gauge::default()
                .block(
                    Block::default()
                        .border_style(border_style)
                        .title(lv_data.lv_name.clone())
                        .title_style(Style::new().fg(self.colors.block_border))
                        .border_type(BorderType::Rounded)
                        .borders(Borders::ALL),
                )
                .gauge_style(
                    Style::new()
                        .fg(self.colors.selected_row_style_fg)
                        .on_black()
                        .italic(),
                )
                .percent(percent)
                .label(format!("{}% of vg:{}", percent, lv_data.vg_name));
            frame.render_widget(bar, lv_area);

            i += ITEM_HEIGHT;
            item_to_rndr += 1;
            if y > area.height {
                break;
            }
        }
        self.lvs_item_rndr_end = item_to_rndr;
    }
}
