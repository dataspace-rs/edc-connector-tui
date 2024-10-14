use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::components::{Component, ComponentEvent, ComponentMsg, ComponentReturn};

use super::{msg::FieldMsg, ChangeSet, FieldComponent};

#[derive(Default, Clone)]
pub struct RowField {
    name: String,
    fields: Vec<FieldComponent>,
    selected: bool,
    selected_idx: usize,
}

impl RowField {
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
        self.fields[self.selected_idx].set_selected(selected);
    }

    pub fn as_map(&self) -> HashMap<String, FieldComponent> {
        self.fields
            .clone()
            .into_iter()
            .map(|f| (f.name().clone(), f))
            .collect()
    }

    pub fn set_values(&mut self, values: Vec<(String, ChangeSet)>) -> anyhow::Result<()> {
        for val in values {
            let f = self
                .fields
                .iter_mut()
                .find(|f| f.name() == val.0)
                .ok_or_else(|| anyhow::anyhow!("Field {} not found", val.0))?;

            f.change(val.1)?;
        }
        Ok(())
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn field(mut self, field: impl Into<FieldComponent>) -> Self {
        self.fields.push(field.into());
        self
    }
}

#[async_trait::async_trait]
impl Component for RowField {
    type Msg = RowMsg;

    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        let percentage = if !self.fields.is_empty() {
            100 / self.fields.len() as u16
        } else {
            100
        };

        let constraints = (0..self.fields.len())
            .map(|_| Constraint::Percentage(percentage))
            .collect::<Vec<_>>();

        let layouts = Layout::horizontal(constraints).split(rect);

        for (idx, field) in self.fields.iter_mut().enumerate() {
            field.view(f, layouts[idx]);
        }
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            RowMsg::MoveLeft => self.move_left(),
            RowMsg::MoveRight => self.move_right(),
            RowMsg::FieldMsg(msg) => {
                Self::forward_update(&mut self.fields[self.selected_idx], (*msg).into(), |msg| {
                    RowMsg::FieldMsg(Box::new(msg))
                })
                .await?;
            }
        };
        Ok(ComponentReturn::empty())
    }

    fn handle_event(
        &mut self,
        evt: crate::components::ComponentEvent,
    ) -> anyhow::Result<Vec<crate::components::ComponentMsg<Self::Msg>>> {
        match evt {
            ComponentEvent::Event(Event::Key(key)) => self.handle_key(key),
            _ => Ok(vec![]),
        }
    }
}

impl RowField {
    fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<Vec<ComponentMsg<RowMsg>>> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('h'), KeyModifiers::CONTROL) | (KeyCode::Left, _) => {
                Ok(vec![RowMsg::MoveLeft.into()])
            }
            (KeyCode::Char('l'), KeyModifiers::CONTROL) | (KeyCode::Right, _) => {
                Ok(vec![RowMsg::MoveRight.into()])
            }
            _ => Self::forward_event(&mut self.fields[self.selected_idx], key.into(), |msg| {
                RowMsg::FieldMsg(Box::new(msg))
            }),
        }
    }

    fn move_left(&mut self) {
        if self.selected_idx != 0 {
            self.fields[self.selected_idx].set_selected(false);
            self.selected_idx -= 1;
            self.fields[self.selected_idx].set_selected(true);
        } else {
            self.fields[self.selected_idx].set_selected(true);
        }
    }

    fn move_right(&mut self) {
        let len = self.fields.len() - 1;
        if self.selected_idx != len {
            self.fields[self.selected_idx].set_selected(false);
            self.selected_idx += 1;
            self.fields[self.selected_idx].set_selected(true);
        }
    }
}

#[derive(Debug)]
pub enum RowMsg {
    MoveRight,
    MoveLeft,
    FieldMsg(Box<FieldMsg>),
}
