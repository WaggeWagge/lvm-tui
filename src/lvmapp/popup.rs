use derive_setters::Setters;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use Constraint::{Max, Min};

use crate::lvmapp::res::Colors;

#[derive(Debug, Default, Setters)]
pub struct ConfPopup<'a> {
    #[setters(into)]
    title: Line<'a>,
    content: Text<'a>,
    border_style: Style,
    title_style: Style,
    style: Style,
    colors: Colors,
}

impl<'a> ConfPopup<'a> {
    pub fn new(colors: Colors) -> Self {
        Self {
            colors: colors.clone(),
            border_style: Style {
                fg: Some(colors.block_border),
                ..Default::default()
            },
            style: Style {
                bg: Some(colors.buffer_bg),
                ..Default::default()
            },
            title_style: Style {
                fg: Some(colors.block_border),
                bg: Some(colors.header_bg),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl Widget for ConfPopup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        // add a wrapper to get some spacing.
        let outer_layout = &Layout::vertical([Min(1), Max(2)])
            .horizontal_margin(2)
            .vertical_margin(2)
            .spacing(1);

        let block = Block::new()
            .title(self.title)
            .title_style(self.title_style)
            .borders(Borders::ALL)
            .border_style(self.border_style)
            .style(Style::new().bg(self.colors.buffer_bg));

        block.render(area, buf);

        let [content_area, act_area] = outer_layout.areas(area);

        let s1 = Style::new().white().bold();
        let s2 = Style::new()
            .bg(self.colors.infotxt_bg)
            .fg(self.colors.infotxt_fg);
        let enter = Span::from("ENTER").style(s1);
        let ok = Span::from("Ok!").style(s2);
        let esc = Span::from(" ESC").style(s1);
        let calcel = Span::from("Cancel!").style(s2);

        let line = Line::from(vec![enter, ok, esc, calcel]);

        let para = Paragraph::new(self.content)
            .wrap(Wrap { trim: true })
            .centered()
            .style(self.style);
        para.render(content_area, buf);

        let para_act = Paragraph::new(line).centered().style(self.style);
        para_act.render(act_area, buf);
    }
}
