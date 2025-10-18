use crate::tui::app::TuiApp;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, area: Rect, _app: &TuiApp) {
    let block = Block::default().borders(Borders::ALL).title("Requests");
    let text = Paragraph::new("Requests tab - Coming in Phase 3").block(block);
    f.render_widget(text, area);
}
