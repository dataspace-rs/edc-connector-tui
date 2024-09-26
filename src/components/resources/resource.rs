use std::fmt::Debug;

use arboard::Clipboard;
use crossterm::event::{Event, KeyCode, KeyEvent};
use msg::ResourceMsg;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect, Size},
    style::{Color, Style},
    text::Span,
    widgets::{block::Title, Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
    Frame,
};
use tui_scrollview::{ScrollView, ScrollViewState};

pub mod msg;
use super::{Component, DrawableResource, Field, FieldValue};
use crate::{
    components::{Action, ComponentEvent, ComponentMsg, ComponentReturn, Notification},
    types::info::InfoSheet,
};

pub struct ResourceComponent<T> {
    resource: Option<T>,
    name: String,
    selected_field: usize,
    clip: Clipboard,
    scroll_view_state: ScrollViewState,
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
            scroll_view_state: ScrollViewState::default(),
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
            scroll_view_state: ScrollViewState::default(),
        }
    }

    pub fn update_resource(&mut self, resource: Option<T>) {
        self.resource = resource;
    }
    fn fields_height(&self) -> u16 {
        if let Some(res) = self.resource.as_ref() {
            res.fields()
                .into_iter()
                .map(|f| match f.value {
                    FieldValue::Str(_) => 3,
                    FieldValue::Json(_) => 10,
                })
                .sum()
        } else {
            0
        }
    }
    fn fields_height_at(&self, idx: usize) -> u16 {
        if let Some(res) = self.resource.as_ref() {
            res.fields()
                .into_iter()
                .take(idx)
                .map(|f| match f.value {
                    FieldValue::Str(_) => 3,
                    FieldValue::Json(_) => 10,
                })
                .sum()
        } else {
            0
        }
    }
    pub fn field_constraints(&self) -> Vec<Constraint> {
        if let Some(res) = self.resource.as_ref() {
            res.fields()
                .into_iter()
                .map(|f| match f.value {
                    FieldValue::Str(_) => Constraint::Length(3),
                    FieldValue::Json(_) => Constraint::Length(10),
                })
                .collect()
        } else {
            vec![]
        }
    }

    pub fn render_fields(&self, buffer: &mut Buffer) {
        let constraints = self.field_constraints();

        let areas = Layout::vertical(constraints).split(buffer.area);

        if let Some(res) = self.resource.as_ref() {
            for (idx, elem) in res.fields().into_iter().enumerate() {
                let field = FieldWidget::new(&elem, idx == self.selected_field);
                field.render(areas[idx], buffer);
            }
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
        if self.resource.is_some() && self.selected_field != 0 {
            self.selected_field -= 1;
            self.scroll_to_field(self.selected_field);
        }
    }

    fn move_down(&mut self) {
        if let Some(res) = self.resource.as_ref() {
            if self.selected_field != res.fields().len() - 1 {
                self.selected_field += 1;
                self.scroll_to_field(self.selected_field);
            };
        }
    }

    fn scroll_to_field(&mut self, idx: usize) {
        let ref_field = if idx == 0 { 0 } else { idx - 1 };
        let mut offset = self.scroll_view_state.offset();
        offset.y = self.fields_height_at(ref_field);
        self.scroll_view_state.set_offset(offset);
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

        let area = block.inner(rect);

        let mut scroll_view = ScrollView::new(Size::new(area.width - 1, self.fields_height()));

        self.render_fields(scroll_view.buf_mut());
        scroll_view.render(area, f.buffer_mut(), &mut self.scroll_view_state);
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

pub struct FieldWidget<'a> {
    field: &'a Field,
    selected: bool,
}

impl<'a> FieldWidget<'a> {
    pub fn new(f: &'a Field, selected: bool) -> Self {
        Self { field: f, selected }
    }
}

impl<'a> Widget for FieldWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let style = if self.selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let styled_text = Span::styled(format!(" {} ", self.field.name), style);

        let value = Paragraph::new(self.field.value.as_ref()).block(
            Block::bordered()
                .title(styled_text)
                .border_style(style)
                .border_type(BorderType::Double),
        );

        value.render(area, buf);
    }
}
