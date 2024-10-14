use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::components::{Component, ComponentEvent};

pub type OnClick<M> = Box<dyn Fn() -> M + Send + Sync>;

pub struct ButtonComponent<M> {
    selected: bool,
    label: String,
    on_click: Option<OnClick<M>>,
}

impl<M> Default for ButtonComponent<M> {
    fn default() -> Self {
        Self {
            selected: Default::default(),
            label: Default::default(),
            on_click: Default::default(),
        }
    }
}

impl<M: Send + Sync + 'static> Component for ButtonComponent<M> {
    type Msg = ButtonMsg<M>;

    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        let style = if self.selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let styled_text = Span::styled(self.label.clone(), style);

        let confirm = Paragraph::new(Line::from(styled_text))
            .centered()
            .block(Block::default().borders(Borders::TOP));

        f.render_widget(confirm, rect);
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<crate::components::ComponentMsg<Self::Msg>>> {
        match evt {
            ComponentEvent::Event(Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            })) => Ok(self
                .on_click
                .as_ref()
                .map(|cb| cb())
                .map(ButtonMsg::Outer)
                .map(|msg| vec![msg.into()])
                .unwrap_or_default()),
            _ => Ok(vec![]),
        }
    }
}

#[derive(Debug)]
pub enum ButtonMsg<M> {
    Outer(M),
}

impl<M> ButtonComponent<M> {
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    pub fn on_click(mut self, cb: impl Fn() -> M + Send + Sync + 'static) -> Self {
        self.on_click = Some(Box::new(cb));
        self
    }

    pub fn label(mut self, label: &str) -> ButtonComponent<M> {
        self.label = label.to_string();
        self
    }
}
