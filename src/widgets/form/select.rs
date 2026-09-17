use crossterm::event::{Event, KeyCode};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::components::{Component, ComponentEvent, ComponentMsg, ComponentReturn};

/// A form field cycling through a fixed list of values.
///
/// `Space`/`l` move to the next value and `Backspace`/`h` to the previous one. `Left`/`Right`
/// are not used because a [`super::row::RowField`] consumes them to move between its fields.
#[derive(Clone, Debug)]
pub struct SelectField {
    name: String,
    label: String,
    options: Vec<String>,
    idx: usize,
    selected: bool,
}

#[derive(Debug)]
pub enum SelectFieldMsg {
    Next,
    Prev,
}

impl SelectField {
    pub fn new(
        name: &str,
        label: &str,
        options: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.to_string(),
            label: label.to_string(),
            options: options.into_iter().map(Into::into).collect(),
            idx: 0,
            selected: false,
        }
    }

    /// Pre-selects `value`; unknown values are ignored.
    pub fn with_value(mut self, value: &str) -> Self {
        let _ = self.set_value(value);
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> String {
        self.options.get(self.idx).cloned().unwrap_or_default()
    }

    pub fn set_value(&mut self, value: &str) -> anyhow::Result<()> {
        match self.options.iter().position(|o| o == value) {
            Some(idx) => {
                self.idx = idx;
                Ok(())
            }
            None => anyhow::bail!(
                "Value '{}' is not one of {}",
                value,
                self.options.join(", ")
            ),
        }
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    fn next(&mut self) {
        if !self.options.is_empty() {
            self.idx = (self.idx + 1) % self.options.len();
        }
    }

    fn prev(&mut self) {
        if !self.options.is_empty() {
            self.idx = (self.idx + self.options.len() - 1) % self.options.len();
        }
    }
}

#[async_trait::async_trait]
impl Component for SelectField {
    type Msg = SelectFieldMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        let border_style = if self.selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let block = Block::default()
            .borders(Borders::all())
            .border_style(border_style)
            .title(self.label.clone());
        f.render_widget(
            Paragraph::new(format!("< {} >", self.value())).block(block),
            rect,
        );
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            SelectFieldMsg::Next => self.next(),
            SelectFieldMsg::Prev => self.prev(),
        }
        Ok(ComponentReturn::empty())
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        match evt {
            ComponentEvent::Event(Event::Key(key)) => Ok(match key.code {
                KeyCode::Char(' ') | KeyCode::Char('l') => vec![SelectFieldMsg::Next.into()],
                KeyCode::Backspace | KeyCode::Char('h') => vec![SelectFieldMsg::Prev.into()],
                _ => vec![],
            }),
            _ => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn field() -> SelectField {
        SelectField::new("v", "Version", ["v3", "v4", "v5"])
    }

    async fn press(field: &mut SelectField, code: KeyCode) -> usize {
        let msgs = field.handle_event(KeyEvent::from(code).into()).unwrap();
        let count = msgs.len();
        for m in msgs {
            field.update(m).await.unwrap();
        }
        count
    }

    #[tokio::test]
    async fn cycles_and_wraps_in_both_directions() {
        let mut f = field();
        assert_eq!(f.value(), "v3");
        press(&mut f, KeyCode::Char(' ')).await;
        assert_eq!(f.value(), "v4");
        press(&mut f, KeyCode::Char('l')).await;
        press(&mut f, KeyCode::Char(' ')).await;
        assert_eq!(f.value(), "v3");
        press(&mut f, KeyCode::Backspace).await;
        assert_eq!(f.value(), "v5");
        press(&mut f, KeyCode::Char('h')).await;
        assert_eq!(f.value(), "v4");
    }

    #[tokio::test]
    async fn enter_and_text_keys_produce_nothing() {
        let mut f = field();
        assert_eq!(press(&mut f, KeyCode::Enter).await, 0);
        assert_eq!(press(&mut f, KeyCode::Char('x')).await, 0);
        assert_eq!(press(&mut f, KeyCode::Left).await, 0);
        assert_eq!(f.value(), "v3");
    }

    #[test]
    fn set_value_rejects_unknown_values() {
        let mut f = field();
        assert!(f.set_value("v9").is_err());
        assert!(f.set_value("v5").is_ok());
        assert_eq!(f.value(), "v5");
        assert_eq!(field().with_value("nope").value(), "v3");
    }
}
