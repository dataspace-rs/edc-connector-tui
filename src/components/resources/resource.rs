use std::fmt::Debug;

use arboard::Clipboard;
use crossterm::event::{Event, KeyCode, KeyEvent};
use msg::ResourceMsg;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{block::Title, Block, BorderType, Borders, Paragraph},
    Frame,
};

pub mod msg;
use super::{Component, DrawableResource, Field, FieldValue};
use crate::{
    components::{
        Action, ComponentEvent, ComponentMsg, ComponentReturn, Notification, StatelessComponent,
    },
    types::info::InfoSheet,
};

pub struct ResourceComponent<T> {
    resource: Option<T>,
    name: String,
    selected_field: usize,
    clip: Clipboard,
}

impl<T> Debug for ResourceComponent<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceComponent")
            .field("name", &self.name)
            .field("selected_field", &self.selected_field)
            .finish()
    }
}

impl<T> Default for ResourceComponent<T> {
    fn default() -> Self {
        Self {
            resource: Default::default(),
            name: String::default(),
            selected_field: 0,
            clip: Clipboard::new().unwrap(),
        }
    }
}

impl<T> ResourceComponent<T> {
    pub fn info_sheet(&self) -> InfoSheet {
        InfoSheet::default()
            .key_binding("<j/down>", "Down")
            .key_binding("<k/down>", "Up")
            .key_binding("<y>", "Copy value")
    }
}

impl<T: DrawableResource> ResourceComponent<T> {
    pub fn new(name: String) -> Self {
        Self {
            name,
            resource: None,
            selected_field: 0,
            clip: Clipboard::new().unwrap(),
        }
    }

    pub fn update_resource(&mut self, resource: Option<T>) {
        self.resource = resource;
    }

    pub fn field_constraints(&self) -> Vec<Constraint> {
        if let Some(res) = self.resource.as_ref() {
            res.fields()
                .into_iter()
                .map(|f| match f.value {
                    FieldValue::Str(_) => Constraint::Length(3),
                    FieldValue::Json(_) => Constraint::Min(5),
                })
                .collect()
        } else {
            vec![]
        }
    }

    fn handle_key(&self, key: KeyEvent) -> Vec<ComponentMsg<ResourceMsg>> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => vec![(ComponentMsg(ResourceMsg::MoveDown))],
            KeyCode::Char('k') | KeyCode::Up => vec![(ComponentMsg(ResourceMsg::MoveUp))],
            KeyCode::Char('y') => vec![(ComponentMsg(ResourceMsg::Yank))],
            _ => vec![],
        }
    }

    fn yank(&mut self) -> anyhow::Result<ComponentReturn<ResourceMsg>> {
        if let Some(res) = self.resource.as_ref() {
            if let Some(field) = res.fields().get(self.selected_field) {
                self.clip
                    .set_text(field.value.as_ref().to_string())
                    .unwrap();

                let notification =
                    Notification::info(format!("Value of '{}' field copied!", field.name));
                return Ok(ComponentReturn::action(Action::Notification(notification)));
            }
        }
        Ok(ComponentReturn::empty())
    }

    fn move_up(&mut self) {
        if let Some(res) = self.resource.as_ref() {
            let pos = if self.selected_field == 0 {
                res.fields().len() - 1
            } else {
                self.selected_field - 1
            };
            self.selected_field = pos;
        }
    }

    fn move_down(&mut self) {
        if let Some(res) = self.resource.as_ref() {
            let pos = if self.selected_field == res.fields().len() - 1 {
                0
            } else {
                self.selected_field + 1
            };
            self.selected_field = pos;
        }
    }
}

#[async_trait::async_trait]
impl<T: DrawableResource + Send> Component for ResourceComponent<T> {
    type Msg = ResourceMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        let styled_text = Span::styled(
            format!(
                " {}({}) ",
                self.name,
                self.resource.as_ref().map(|a| a.id()).unwrap_or("N/A")
            ),
            Style::default().fg(Color::Red),
        );
        let block = Block::default()
            .title(Title::from(styled_text).alignment(Alignment::Center))
            .borders(Borders::ALL);

        let constraints = self.field_constraints();

        if !constraints.is_empty() {
            let area = block.inner(rect);
            let layout = Layout::vertical(constraints).split(area);
            if let Some(res) = self.resource.as_ref() {
                for (idx, elem) in res.fields().into_iter().enumerate() {
                    let mut field = FieldComponent;
                    field.view(&(elem, idx == self.selected_field), f, layout[idx]);
                }
            }
        }

        f.render_widget(block, rect);
    }

    async fn update(
        &mut self,
        message: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match message.take() {
            ResourceMsg::MoveUp => self.move_up(),
            ResourceMsg::MoveDown => self.move_down(),
            ResourceMsg::Yank => return self.yank(),
        };

        Ok(ComponentReturn::empty())
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        match evt {
            ComponentEvent::Event(Event::Key(key)) => Ok(self.handle_key(key)),
            _ => Ok(vec![]),
        }
    }
}

pub struct FieldComponent;

impl StatelessComponent for FieldComponent {
    type Props = (Field, bool);

    fn view(&mut self, (field, selected): &Self::Props, f: &mut Frame, rect: Rect) {
        let style = if *selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let styled_text = Span::styled(format!(" {} ", field.name), style);

        let value = Paragraph::new(field.value.as_ref()).block(
            Block::bordered()
                .title(styled_text)
                .border_style(style)
                .border_type(BorderType::Double),
        );

        f.render_widget(value, rect)
    }
}
