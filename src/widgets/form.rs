use std::collections::HashMap;

use button::{ButtonComponent, ButtonMsg};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use msg::{FieldMsg, FormLocalMsg, FormMsg};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};
use row::RowField;
use text::TextField;

use crate::components::{Component, ComponentEvent, ComponentMsg, ComponentReturn};
pub mod button;
pub mod msg;
pub mod row;
pub mod text;

pub type OnConfirm<M> =
    Box<dyn Fn(HashMap<String, FieldComponent>) -> anyhow::Result<M> + Send + Sync>;

pub struct Form<M> {
    fields: Vec<FieldComponent>,
    selected: usize,
    confirm: ButtonComponent<FormLocalMsg>,
    on_confirm: Option<OnConfirm<M>>,
    confirm_focus: bool,
}

impl<M> Default for Form<M> {
    fn default() -> Self {
        Self {
            fields: vec![],
            selected: 0,
            confirm_focus: false,
            confirm: ButtonComponent::default()
                .label("Confirm")
                .on_click(|| FormLocalMsg::Submit),

            on_confirm: None,
        }
    }
}

impl<M> Form<M> {
    pub fn on_confirm(
        mut self,
        cb: impl Fn(HashMap<String, FieldComponent>) -> anyhow::Result<M> + Send + Sync + 'static,
    ) -> Self {
        self.on_confirm = Some(Box::new(cb));
        self
    }
    pub fn field(mut self, field: impl Into<FieldComponent>) -> Self {
        self.fields.push(field.into());
        self
    }

    fn move_up(&mut self) {
        if self.selected != 0 && !self.confirm_focus {
            self.fields[self.selected].set_selected(false);
            self.selected -= 1;
            self.fields[self.selected].set_selected(true);
        } else {
            self.confirm_focus = false;
            self.fields[self.selected].set_selected(true);
            self.confirm.set_selected(false);
        }
    }

    fn move_down(&mut self) {
        let len = self.fields.len() - 1;
        if self.selected != len && !self.confirm_focus {
            self.fields[self.selected].set_selected(false);
            self.selected += 1;
            self.fields[self.selected].set_selected(true);
        } else {
            self.confirm_focus = true;
            self.fields[self.selected].set_selected(false);
            self.confirm.set_selected(true);
        }
    }

    pub fn change_field(&mut self, name: &str, change: impl Into<ChangeSet>) -> anyhow::Result<()> {
        let field = self
            .fields
            .iter_mut()
            .find(|f| f.name() == name)
            .ok_or_else(|| anyhow::anyhow!("Field {} not found", name))?;

        field.change(change.into())?;

        Ok(())
    }
}

impl<M: Send + Sync + 'static> Form<M> {
    fn handle_enter(&mut self, key: KeyEvent) -> anyhow::Result<Vec<ComponentMsg<FormMsg<M>>>> {
        let msg = Self::forward_event(&mut self.fields[self.selected], key.into(), |msg| {
            FormMsg::Local(FormLocalMsg::FieldMsg(msg))
        })?;
        if msg.is_empty() {
            Ok(vec![FormMsg::Local(FormLocalMsg::MoveDown).into()])
        } else {
            Ok(msg)
        }
    }
}

#[async_trait::async_trait]
impl<M: Send + Sync + 'static> Component for Form<M> {
    type Msg = FormMsg<M>;
    type Props = ();

    fn view(&mut self, f: &mut Frame, area: Rect) {
        let constraints = (0..=self.fields.len())
            .map(|_| Constraint::Length(3))
            .collect::<Vec<_>>();

        let layouts = Layout::vertical(constraints).split(area);

        for (idx, field) in self.fields.iter_mut().enumerate() {
            field.view(f, layouts[idx]);
        }

        self.confirm.view(f, layouts[self.fields.len()]);
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            FormMsg::Local(FormLocalMsg::MoveUp) => self.move_up(),
            FormMsg::Local(FormLocalMsg::MoveDown) => self.move_down(),
            FormMsg::Local(FormLocalMsg::FieldMsg(msg)) => {
                return Self::forward_update(&mut self.fields[self.selected], msg.into(), |msg| {
                    FormMsg::Local(FormLocalMsg::FieldMsg(msg))
                })
                .await
            }
            FormMsg::Local(FormLocalMsg::Submit) => {
                if let Some(cb) = self.on_confirm.as_ref() {
                    let fields = self
                        .fields
                        .clone()
                        .into_iter()
                        .map(|f| (f.name(), f))
                        .collect();
                    let msg = cb(fields)?;
                    return Ok(ComponentReturn::msg(FormMsg::Outer(msg).into()));
                }
            }
            FormMsg::Outer(_) => unimplemented!(),
        };
        Ok(ComponentReturn::empty())
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        match evt {
            ComponentEvent::Event(Event::Key(key)) => self.handle_key(key),
            _ => Ok(vec![]),
        }
    }
}

impl<M: Send + Sync + 'static> Form<M> {
    fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<Vec<ComponentMsg<FormMsg<M>>>> {
        match (key.code, key.modifiers, self.confirm_focus) {
            (KeyCode::Char('j'), KeyModifiers::CONTROL, _)
            | (KeyCode::Down, _, _)
            | (KeyCode::Tab, _, _) => Ok(vec![FormMsg::Local(FormLocalMsg::MoveDown).into()]),
            (KeyCode::Char('k'), KeyModifiers::CONTROL, _) | (KeyCode::Up, _, _) => {
                Ok(vec![FormMsg::Local(FormLocalMsg::MoveUp).into()])
            }
            (KeyCode::Enter, _, false) => self.handle_enter(key),
            (KeyCode::Enter, _, true) => {
                Self::forward_event(&mut self.confirm, key.into(), |msg| match msg {
                    ButtonMsg::Outer(msg) => FormMsg::Local(msg),
                })
            }
            _ => Self::forward_event(&mut self.fields[self.selected], key.into(), |msg| {
                FormMsg::Local(FormLocalMsg::FieldMsg(msg))
            }),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum FieldComponent {
    Text(TextField),
    Row(RowField),
}

impl TryInto<String> for FieldComponent {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            FieldComponent::Text(text_field) => Ok(text_field.value()),
            FieldComponent::Row(_) => anyhow::bail!("Cannot extract a string from row"),
        }
    }
}

impl FieldComponent {
    pub fn set_selected(&mut self, selected: bool) {
        match self {
            FieldComponent::Text(txt) => txt.set_selected(selected),
            FieldComponent::Row(row) => row.set_selected(selected),
        }
    }

    pub fn change(&mut self, change: ChangeSet) -> anyhow::Result<()> {
        match (self, change) {
            (FieldComponent::Text(text_field), ChangeSet::Single(txt)) => {
                text_field.set_value(&txt)
            }
            (FieldComponent::Row(row_field), ChangeSet::Row(vec)) => {
                row_field.set_values(vec.into_iter().map(|(f, v)| (f, *v)).collect())
            }

            _ => anyhow::bail!("Not supported"),
        }
    }

    pub fn name(&self) -> String {
        match self {
            FieldComponent::Text(text_field) => text_field.name().to_string(),
            FieldComponent::Row(row_field) => row_field.get_name(),
        }
    }
}

#[async_trait::async_trait]
impl Component for FieldComponent {
    type Msg = FieldMsg;

    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        match self {
            FieldComponent::Text(txt) => txt.view(f, rect),
            FieldComponent::Row(row_field) => row_field.view(f, rect),
        }
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match (self, msg.take()) {
            (FieldComponent::Text(text), FieldMsg::Text(msg)) => {
                Self::forward_update(text, msg.into(), FieldMsg::Text).await
            }
            (FieldComponent::Row(row), FieldMsg::Row(msg)) => {
                Self::forward_update(row, msg.into(), FieldMsg::Row).await
            }
            _ => unreachable!(),
        }
    }

    fn handle_event(
        &mut self,
        evt: crate::components::ComponentEvent,
    ) -> anyhow::Result<Vec<crate::components::ComponentMsg<Self::Msg>>> {
        match self {
            FieldComponent::Text(text) => Self::forward_event(text, evt, FieldMsg::Text),
            FieldComponent::Row(row) => Self::forward_event(row, evt, FieldMsg::Row),
        }
    }
}

impl From<TextField> for FieldComponent {
    fn from(value: TextField) -> Self {
        FieldComponent::Text(value)
    }
}
impl From<RowField> for FieldComponent {
    fn from(value: RowField) -> Self {
        FieldComponent::Row(value)
    }
}

pub enum ChangeSet {
    Single(String),
    Row(Vec<(String, Box<ChangeSet>)>),
}

impl ChangeSet {
    pub fn single(value: &str) -> ChangeSet {
        ChangeSet::Single(value.to_string())
    }

    pub fn row(values: Vec<(&str, ChangeSet)>) -> ChangeSet {
        ChangeSet::Row(
            values
                .into_iter()
                .map(|s| (s.0.to_string(), Box::new(s.1)))
                .collect(),
        )
    }
}

impl From<&str> for ChangeSet {
    fn from(value: &str) -> Self {
        ChangeSet::single(value)
    }
}
impl From<String> for ChangeSet {
    fn from(value: String) -> Self {
        ChangeSet::Single(value)
    }
}
impl From<u32> for ChangeSet {
    fn from(value: u32) -> Self {
        ChangeSet::Single(value.to_string())
    }
}
