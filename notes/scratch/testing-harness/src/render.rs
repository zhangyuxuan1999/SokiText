use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use crate::Core;

pub fn draw(f: &mut Frame, core: &Core) {
    let area = f.area();
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
    let text: String = core.text.to_string();
    f.render_widget(Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("jio")), chunks[0]);
    let status = format!(" {} | cur {} ", if core.dirty { "[+]" } else { "   " }, core.cursor);
    f.render_widget(Paragraph::new(status).style(Style::new().bg(Color::Blue).fg(Color::White)), chunks[1]);
}
