use Constraint::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use itertools::Itertools;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{
        Block, BorderType, Borders, Gauge, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, TableState, Widget,
    },
};

use crate::{
    lvm::{self, LvmLvData, LvmPVData, LvmVgData},
    lvmapp::{
        View, ViewType,
        res::{self, Colors},
    },
};

pub struct VgPvInfo {
    colors: Colors,
    vg_list: Vec<LvmVgData>,
}

const ITEM_HEIGHT: u16 = 3;

impl VgPvInfo {
    pub fn new(vg_list: Vec<LvmVgData>) -> Self {
        Self {
            colors: Colors::new(&res::PALETTES[0]),
            vg_list: vg_list,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: &Rect) {
        let v_outer_layout = &Layout::vertical([Min(1)])
            .vertical_margin(0)
            .horizontal_margin(2);
        let [v_area0] = v_outer_layout.areas(*area);

        self.render_vgpv_list(frame, v_area0);
        //self.render_scrollbar(frame, v_area1);
    }

    fn render_vgpv_list(&mut self, frame: &mut Frame, area: Rect) {
        let v_vgs = vec![
            "vg00", "vg01", "vg02", "vg01", "vg02", "vg01", "vg02", "vg01", "vg02", "vg01", "vg02",
            "vg01", "vg02", "vg01", "vg02",
        ];
        let v_pvs = vec!["/dev/xda1", "/dev/xda3", "/dev/xda5"];

        // let v_layout = &Layout::vertical([ Length(10)]);
        // let [ca] = v_layout.areas(area);
        let mut y = area.y;

        for vg in v_vgs {
            // for the rows we have available render vg items.
            let para = Paragraph::new(format!("{}", vg)).centered().style(
                Style::new()
                    .fg(self.colors.selected_row_style_fg)
                    .on_black()
                    .italic(),
            );

            let n_area = Rect::new(area.x, y, area.width, 1);
            para.render(n_area, frame.buffer_mut());

            y += 2;
            if y >= area.height {
                break;
            }
        }
    }
}
