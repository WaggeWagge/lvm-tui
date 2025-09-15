use derive_setters::Setters;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::lvmapp::res::{self, Colors};

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

//.style(Style::new().bg(self.colors.buffer_bg)) 
                //.title_style(Style::new().bold().fg(self.colors.header_fg))
                //.border_style(Style::new().fg(self.colors.block_border)) 
impl <'a> ConfPopup<'a>  {
   pub fn new(colors: Colors) -> Self {
        Self { colors: colors,            
        ..Default::default() 
        }
   }
}

impl Widget for ConfPopup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {        
        Clear.render(area, buf);        
      
        let s1 = Style::new().white().bold();
        let s2 = Style::new()
                .bg(self.colors.infotxt_bg)
                .fg(self.colors.infotxt_fg);
            let enter = Span::from("ENTER").style(s1);
            let ok = Span::from("Ok!").style(s2);
            let esc = Span::from(" ESC").style(s1);
            let calcel = Span::from("Cancel!").style(s2);        
  
            let line = Line::from(vec![enter, ok, esc, calcel]);

            let mut text = Text::from(self.content);
            
            text.push_line(Line::from(""));
            text.push_line(line);
            
        let block = Block::new()
            .title(self.title)
            .title_style(self.title_style)
            .borders(Borders::ALL)
            .border_style(self.border_style);
        let para = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .centered()
            .style(self.style)
            .block(block);
        para.render(area, buf);

    }
}
