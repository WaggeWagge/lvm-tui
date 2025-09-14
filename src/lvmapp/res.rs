use ratatui::style::{self, Color};
use style::palette::tailwind;

pub const PALETTES: [tailwind::Palette; 4] = [
    tailwind::CYAN,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::SLATE,
];

pub const TITLE: &str = " LVM-TUI ";
pub const ITEM_HEIGHT: usize = 1;

pub struct Colors {
    pub block_border: Color,
    pub buffer_bg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub row_fg: Color,
    pub selected_row_style_fg: Color,
    pub selected_column_style_fg: Color,
    pub selected_cell_style_fg: Color,
    pub normal_row_color: Color,
    pub alt_row_color: Color,
    pub footer_border_color: Color,
    pub infotxt_fg: Color,
    pub infotxt_bg: Color,
}

impl Colors {
    pub const fn new(color: &tailwind::Palette) -> Self {
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
            infotxt_fg: color.c400,
            infotxt_bg: color.c900,
        }
    }
}
