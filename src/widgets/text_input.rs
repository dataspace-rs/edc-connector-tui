use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};
use tui_input::Input;

/// Single-line text input rendered from a [`tui_input::Input`].
///
/// The cursor is drawn as a reversed cell inside the buffer rather than with
/// the terminal cursor, so several inputs can be on screen at once and only
/// the focused one shows a cursor.
pub struct TextInput<'a> {
    input: &'a Input,
    prefix: Option<&'a str>,
    block: Option<Block<'a>>,
    show_cursor: bool,
}

impl<'a> TextInput<'a> {
    pub fn new(input: &'a Input) -> Self {
        Self {
            input,
            prefix: None,
            block: None,
            show_cursor: true,
        }
    }

    /// Fixed text drawn before the value that never scrolls and cannot be edited.
    pub fn prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn show_cursor(mut self, show_cursor: bool) -> Self {
        self.show_cursor = show_cursor;
        self
    }
}

impl Widget for TextInput<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = match &self.block {
            Some(block) => block.inner(area),
            None => area,
        };
        if let Some(block) = self.block {
            block.render(area, buf);
        }
        if inner.is_empty() {
            return;
        }

        let prefix = Span::raw(self.prefix.unwrap_or_default());
        let prefix_width = (prefix.width() as u16).min(inner.width);
        let prefix_area = Rect {
            width: prefix_width,
            ..inner
        };
        let text_area = Rect {
            x: inner.x + prefix_width,
            width: inner.width - prefix_width,
            ..inner
        };
        Paragraph::new(prefix).render(prefix_area, buf);
        if text_area.is_empty() {
            return;
        }

        let chars: Vec<char> = self.input.value().chars().collect();
        let cursor = self.input.cursor().min(chars.len());
        let before: String = chars[..cursor].iter().collect();
        let (at, after): (String, String) = match chars.get(cursor) {
            Some(c) => (c.to_string(), chars[cursor + 1..].iter().collect()),
            None => (" ".to_string(), String::new()),
        };
        let cursor_style = if self.show_cursor {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let line = Line::from(vec![
            Span::raw(before),
            Span::styled(at, cursor_style),
            Span::raw(after),
        ]);

        // Keep one column free so the cursor cell is visible at the end of the value.
        let visible_width = text_area.width.saturating_sub(1).max(1) as usize;
        let scroll = self.input.visual_scroll(visible_width);
        Paragraph::new(line)
            .scroll((0, scroll as u16))
            .render(text_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::widgets::Borders;

    fn render(widget: TextInput<'_>, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        buf
    }

    fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect()
    }

    fn reversed_columns(buf: &Buffer, y: u16) -> Vec<u16> {
        (0..buf.area.width)
            .filter(|&x| buf[(x, y)].modifier.contains(Modifier::REVERSED))
            .collect()
    }

    #[test]
    fn renders_prefix_value_and_cursor_at_end() {
        let input = Input::from("assets");
        let buf = render(TextInput::new(&input).prefix(" $> "), 20, 1);

        assert_eq!(row(&buf, 0), " $> assets          ");
        // cursor cell is the blank right after the value, drawn reversed
        assert_eq!(reversed_columns(&buf, 0), vec![10]);
    }

    #[test]
    fn cursor_in_the_middle_highlights_the_char_under_it() {
        let input = Input::from("assets").with_cursor(2);
        let buf = render(TextInput::new(&input), 10, 1);

        assert_eq!(row(&buf, 0), "assets    ");
        assert_eq!(reversed_columns(&buf, 0), vec![2]);
        assert_eq!(buf[(2, 0)].symbol(), "s");
    }

    #[test]
    fn hidden_cursor_draws_nothing_reversed() {
        let input = Input::from("assets");
        let buf = render(TextInput::new(&input).show_cursor(false), 10, 1);

        assert_eq!(row(&buf, 0), "assets    ");
        assert!(reversed_columns(&buf, 0).is_empty());
    }

    #[test]
    fn renders_inside_block() {
        let input = Input::from("hi");
        let block = Block::default().borders(Borders::ALL);
        let buf = render(TextInput::new(&input).block(block), 8, 3);

        assert_eq!(row(&buf, 1), "│hi    │");
        assert_eq!(reversed_columns(&buf, 1), vec![3]);
    }

    #[test]
    fn long_value_scrolls_to_keep_cursor_visible() {
        let input = Input::from("0123456789abcdef");
        let buf = render(TextInput::new(&input).prefix("> "), 10, 1);

        let line = row(&buf, 0);
        assert!(line.starts_with("> "), "prefix must not scroll: {line:?}");
        assert!(
            line.contains("cdef"),
            "tail of value must be visible: {line:?}"
        );
        assert!(
            !line.contains("0123"),
            "head of value must be scrolled out: {line:?}"
        );
        assert_eq!(reversed_columns(&buf, 0), vec![9]);
    }

    #[test]
    fn empty_area_does_not_panic() {
        let input = Input::from("x");
        let block = Block::default().borders(Borders::ALL);
        render(TextInput::new(&input).prefix(" $> ").block(block), 2, 2);
        render(TextInput::new(&input).prefix(" $> "), 3, 1);
    }
}
