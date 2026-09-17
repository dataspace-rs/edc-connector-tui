use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear},
    Frame,
};

/// A rectangle covering `percent_x` x `percent_y` of `area`, centered.
pub fn centered(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

/// A rectangle of at most `width` x `height` cells, centered in `area`.
pub fn centered_fixed(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(width)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

/// Clears `area`, draws a bordered block with a centered red `title` and returns the inner
/// rectangle to render the popup content into.
pub fn framed(f: &mut Frame, area: Rect, title: &str) -> Rect {
    let styled_text = Span::styled(title.to_string(), Style::default().fg(Color::Red));
    let block = Block::default()
        .title_top(Line::from(styled_text).centered())
        .borders(Borders::ALL);
    let content = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    content
}
