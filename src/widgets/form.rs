use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use msg::{FieldMsg, FormLocalMsg, FormMsg};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};
use text::TextField;

use crate::components::{Component, ComponentEvent, ComponentMsg, ComponentReturn};
pub mod msg;
pub mod text;

pub struct Form {
    fields: Vec<FieldComponent>,
    selected: usize,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            fields: vec![],
            selected: 0,
        }
    }
}

impl Form {
    pub fn field(mut self, field: impl Into<FieldComponent>) -> Self {
        self.fields.push(field.into());
        self
    }

    fn move_up(&mut self) {
        if self.selected != 0 {
            self.fields[self.selected].set_selected(false);
            self.selected -= 1;
            self.fields[self.selected].set_selected(true);
        } else {
            self.fields[0].set_selected(false);
        }
    }

    fn move_down(&mut self) {
        let len = self.fields.len() - 1;
        if self.selected != len {
            self.fields[self.selected].set_selected(false);
            self.selected += 1;
            self.fields[self.selected].set_selected(true);
        } else {
            self.fields[len].set_selected(false);
        }
    }
}

#[async_trait::async_trait]
impl Component for Form {
    type Msg = FormMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, area: Rect) {
        let constraints = (0..=self.fields.len())
            .map(|_| Constraint::Length(3))
            .collect::<Vec<_>>();

        let layouts = Layout::vertical(constraints).split(area);

        for (idx, field) in self.fields.iter_mut().enumerate() {
            field.view(f, layouts[idx]);
        }
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            FormMsg::Local(FormLocalMsg::MoveUp) => dbg!(self.move_up()),
            FormMsg::Local(FormLocalMsg::MoveDown) => self.move_down(),
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

impl Form {
    fn handle_key(&self, key: KeyEvent) -> Vec<ComponentMsg<FormMsg>> {

        match (key.code, key.modifiers, true) {
            (KeyCode::Char('j'), KeyModifiers::CONTROL, _)
            | (KeyCode::Down, _, _)
            | (KeyCode::Tab, _, _)
            | (KeyCode::Enter, _, false) => {
                vec![FormMsg::Local(FormLocalMsg::MoveDown).into()]
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL, _) | (KeyCode::Up, _, _) => {
                vec![FormMsg::Local(FormLocalMsg::MoveUp).into()]
            }
            (KeyCode::Enter, _, true) => {
                vec![]
            }
            _ => vec![],
        }
    }
}

pub enum FieldComponent {
    Text(TextField),
}

impl FieldComponent {
    pub fn set_selected(&mut self, selected: bool) {
        match self {
            FieldComponent::Text(txt) => txt.set_selected(selected),
        }
    }
}

impl Component for FieldComponent {
    type Msg = FieldMsg;

    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        match self {
            FieldComponent::Text(txt) => txt.view(f, rect),
        }
    }
}

impl From<TextField> for FieldComponent {
    fn from(value: TextField) -> Self {
        FieldComponent::Text(value)
    }
}
