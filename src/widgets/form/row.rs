use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::components::Component;

use super::FieldComponent;

#[derive(Default)]
pub struct RowField {
    fields: Vec<FieldComponent>,
    selected: bool,
}

impl RowField {
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
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
        let percentage = if self.fields.len() > 0 {
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
}

#[derive(Debug)]
pub enum RowMsg {}
