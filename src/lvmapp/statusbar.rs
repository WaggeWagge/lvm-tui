use derive_setters::Setters;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span, Text, ToSpan},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::lvmapp::res::Colors;

#[derive(Debug, Default, Setters)]
pub struct StatusBar<'a> {
    #[setters(into)]
    content: Text<'a>,
    status_style: Style,
    colors: Colors,
}

impl<'a> StatusBar<'a> {
    pub fn new(colors: Colors) -> Self {
        Self {
            colors: colors.clone(),
            status_style: Style {
                bg: Some(colors.infotxt_bg),
                fg: Some(colors.infotxt_fg),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let line = Line::from(vec![Span::from("Status: "), self.content.to_span()])
            .style(self.status_style);

        let para = Paragraph::new(line).wrap(Wrap { trim: true }).centered();

        para.render(area, buf);
    }
}
