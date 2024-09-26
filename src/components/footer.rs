use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::{Component, Notification, NotificationKind};

pub mod msg;

#[derive(Default)]
pub struct Footer {
    noty: Option<Notification>,
    noty_counter: u32,
}

#[async_trait::async_trait]
impl Component for Footer {
    type Msg = ();
    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        let block = Block::default().borders(Borders::all());

        let content = self.noty.as_ref().map(Notification::msg).unwrap_or("");

        let style = self
            .noty
            .as_ref()
            .map(Footer::map_color)
            .unwrap_or_default();

        let text = Text::from(Span::styled(content, style));
        let p = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(p, rect)
    }
}

impl Footer {
    pub fn show_notification(&mut self, noty: Notification) {
        self.noty = Some(noty);
        self.noty_counter += 1;
    }
    pub fn clear_notification(&mut self) {
        self.noty_counter -= 1;

        if self.noty_counter == 0 {
            self.noty = None;
        }
    }

    fn map_color(noty: &Notification) -> Style {
        match noty.kind() {
            NotificationKind::Error => Style::default().fg(Color::Red),
            NotificationKind::Info => Style::default().fg(Color::Cyan),
        }
    }
}
