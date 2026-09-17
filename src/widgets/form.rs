use std::collections::HashMap;

use button::{ButtonComponent, ButtonMsg};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use msg::{FieldMsg, FormLocalMsg, FormMsg};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};
use row::RowField;
use select::SelectField;
use text::TextField;

use crate::{
    components::{Component, ComponentEvent, ComponentMsg, ComponentReturn},
    types::info::InfoSheet,
};
pub mod button;
pub mod msg;
pub mod row;
pub mod select;
pub mod text;
pub mod values;

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
    /// The key bindings of every form, for the header help sheet.
    pub fn key_bindings() -> InfoSheet {
        InfoSheet::default()
            .key_binding("<esc>", "Cancel")
            .key_binding("<tab>", "Next field")
            .key_binding("<shift+tab>", "Prev field")
            .key_binding("<up/down>", "Prev/Next row")
            .key_binding("<left/right>", "Move in row")
            .key_binding("<space>", "Cycle value")
            .key_binding("<enter>", "Next/Confirm")
    }

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

    pub fn push_field(&mut self, field: impl Into<FieldComponent>) {
        self.fields.push(field.into());
    }

    /// Drops every field after the first `len`, keeping the focus valid.
    pub fn truncate_fields(&mut self, len: usize) {
        self.fields.truncate(len);
        if self.selected >= self.fields.len() {
            self.selected = self.fields.len().saturating_sub(1);
            self.confirm_focus = false;
            self.confirm.set_selected(false);
            if let Some(field) = self.fields.get_mut(self.selected) {
                field.set_selected(true);
            }
        }
    }

    #[cfg(test)]
    pub fn fields_len(&self) -> usize {
        self.fields.len()
    }

    /// Current value of the top-level or row-nested field called `name`.
    pub fn field_value(&self, name: &str) -> Option<String> {
        self.fields.iter().find_map(|f| match f {
            FieldComponent::Row(row) => row.as_map().remove(name).and_then(|f| f.try_into().ok()),
            other if other.name() == name => other.clone().try_into().ok(),
            _ => None,
        })
    }

    /// Tab: next field within the current row, else the next row (entered at its first field).
    fn next(&mut self) {
        if let Some(FieldComponent::Row(row)) = self.current_field() {
            if !row.is_last() {
                row.move_right();
                return;
            }
        }
        self.move_down();
        if let Some(FieldComponent::Row(row)) = self.current_field() {
            row.select_first();
        }
    }

    /// Shift+Tab: previous field within the current row, else the previous row (entered at its
    /// last field).
    fn prev(&mut self) {
        if let Some(FieldComponent::Row(row)) = self.current_field() {
            if !row.is_first() {
                row.move_left();
                return;
            }
        }
        let was_confirm = self.confirm_focus;
        let was_first = self.selected == 0;
        self.move_up();
        if was_confirm || !was_first {
            if let Some(FieldComponent::Row(row)) = self.current_field() {
                row.select_last();
            }
        }
    }

    /// The focused field, unless the confirm button has the focus.
    fn current_field(&mut self) -> Option<&mut FieldComponent> {
        if self.confirm_focus {
            None
        } else {
            self.fields.get_mut(self.selected)
        }
    }

    fn move_up(&mut self) {
        if self.fields.is_empty() {
            return;
        }
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
        if self.fields.is_empty() {
            return;
        }
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
            Ok(vec![FormMsg::Local(FormLocalMsg::Next).into()])
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
            FormMsg::Local(FormLocalMsg::Next) => self.next(),
            FormMsg::Local(FormLocalMsg::Prev) => self.prev(),
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
            (KeyCode::Char('j'), KeyModifiers::CONTROL, _) | (KeyCode::Down, _, _) => {
                Ok(vec![FormMsg::Local(FormLocalMsg::MoveDown).into()])
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL, _) | (KeyCode::Up, _, _) => {
                Ok(vec![FormMsg::Local(FormLocalMsg::MoveUp).into()])
            }
            (KeyCode::Tab, _, _) => Ok(vec![FormMsg::Local(FormLocalMsg::Next).into()]),
            (KeyCode::BackTab, _, _) => Ok(vec![FormMsg::Local(FormLocalMsg::Prev).into()]),
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
    Select(SelectField),
}

impl TryInto<String> for FieldComponent {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            FieldComponent::Text(text_field) => Ok(text_field.value()),
            FieldComponent::Select(select) => Ok(select.value()),
            FieldComponent::Row(_) => anyhow::bail!("Cannot extract a string from row"),
        }
    }
}

impl FieldComponent {
    pub fn set_selected(&mut self, selected: bool) {
        match self {
            FieldComponent::Text(txt) => txt.set_selected(selected),
            FieldComponent::Row(row) => row.set_selected(selected),
            FieldComponent::Select(select) => select.set_selected(selected),
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
            (FieldComponent::Select(select), ChangeSet::Single(value)) => select.set_value(&value),

            _ => anyhow::bail!("Not supported"),
        }
    }

    pub fn name(&self) -> String {
        match self {
            FieldComponent::Text(text_field) => text_field.name().to_string(),
            FieldComponent::Row(row_field) => row_field.get_name(),
            FieldComponent::Select(select) => select.name().to_string(),
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
            FieldComponent::Select(select) => select.view(f, rect),
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
            (FieldComponent::Select(select), FieldMsg::Select(msg)) => {
                Self::forward_update(select, msg.into(), FieldMsg::Select).await
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
            FieldComponent::Select(select) => Self::forward_event(select, evt, FieldMsg::Select),
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
impl From<SelectField> for FieldComponent {
    fn from(value: SelectField) -> Self {
        FieldComponent::Select(value)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn text(name: &str) -> TextField {
        TextField::builder()
            .name(name.to_string())
            .label(name.to_string())
            .build()
            .unwrap()
    }

    /// `[a | b]`, `c`, `[d | e]`, Confirm
    fn form() -> Form<()> {
        let mut first = RowField::default()
            .name("r1")
            .field(text("a"))
            .field(text("b"));
        first.set_selected(true);
        Form::default().field(first).field(text("c")).field(
            RowField::default()
                .name("r2")
                .field(text("d"))
                .field(text("e")),
        )
    }

    async fn press(form: &mut Form<()>, key: KeyEvent) {
        for m in form.handle_event(key.into()).unwrap() {
            form.update(m).await.unwrap();
        }
    }

    /// Name of the focused (inner) field, or "confirm".
    fn focused(form: &mut Form<()>) -> String {
        if form.confirm_focus {
            return "confirm".to_string();
        }
        match &form.fields[form.selected] {
            FieldComponent::Row(row) => row
                .as_map()
                .into_iter()
                .find(|(_, f)| match f {
                    FieldComponent::Text(t) => t.is_selected(),
                    _ => false,
                })
                .map(|(name, _)| name)
                .unwrap_or_default(),
            other => other.name(),
        }
    }

    #[tokio::test]
    async fn tab_walks_every_field_and_shift_tab_walks_back() {
        let mut f = form();
        let mut seen = vec![focused(&mut f)];
        for _ in 0..6 {
            press(&mut f, KeyEvent::from(KeyCode::Tab)).await;
            seen.push(focused(&mut f));
        }
        assert_eq!(seen, ["a", "b", "c", "d", "e", "confirm", "confirm"]);

        let mut back = vec![];
        for _ in 0..6 {
            press(&mut f, KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)).await;
            back.push(focused(&mut f));
        }
        assert_eq!(back, ["e", "d", "c", "b", "a", "a"]);
    }

    #[tokio::test]
    async fn enter_moves_to_the_next_field_and_submits_on_confirm() {
        let mut f = form().on_confirm(|_| Ok(()));
        press(&mut f, KeyEvent::from(KeyCode::Enter)).await;
        assert_eq!(focused(&mut f), "b");
        for _ in 0..4 {
            press(&mut f, KeyEvent::from(KeyCode::Enter)).await;
        }
        assert_eq!(focused(&mut f), "confirm");
        let msgs = f
            .handle_event(KeyEvent::from(KeyCode::Enter).into())
            .unwrap();
        let submitted = f.update(msgs.into_iter().next().unwrap()).await.unwrap();
        let first = submitted.msgs.into_iter().next().map(|m| m.take());
        assert!(matches!(first, Some(FormMsg::Outer(()))));
    }

    #[tokio::test]
    async fn arrows_move_between_rows_keeping_the_row_position() {
        let mut f = form();
        press(&mut f, KeyEvent::from(KeyCode::Tab)).await; // b
        press(&mut f, KeyEvent::from(KeyCode::Down)).await; // c
        press(&mut f, KeyEvent::from(KeyCode::Up)).await; // back to row 1, still b
        assert_eq!(focused(&mut f), "b");
    }
}
