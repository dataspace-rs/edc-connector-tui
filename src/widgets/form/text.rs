use crossterm::event::{Event, KeyCode};
use derive_builder::Builder;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};
use tui_input::{backend::crossterm::to_input_request, Input, InputRequest};

use crate::{
    components::{Component, ComponentEvent, ComponentMsg, ComponentReturn},
    widgets::text_input::TextInput,
};

#[derive(Builder, Clone)]
pub struct TextField {
    label: String,
    name: String,
    #[builder(default)]
    #[allow(dead_code)]
    initial_value: String,
    #[builder(default = "Input::from(self.initial_value.clone().unwrap_or_default())")]
    input: Input,
    #[builder(default)]
    selected: bool,
    /// Render `*` instead of the typed characters (for secrets).
    #[builder(default)]
    masked: bool,
}

impl TextField {
    pub fn builder() -> TextFieldBuilder {
        TextFieldBuilder::default()
    }

    /// An unmasked field called `name`, titled `label` and pre-filled with `value`.
    pub fn plain(name: &str, label: &str, value: &str) -> TextField {
        Self::builder()
            .name(name.to_string())
            .label(label.to_string())
            .initial_value(value.to_string())
            .build()
            .expect("text field")
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> String {
        self.input.value().to_string()
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    #[cfg(test)]
    pub fn is_selected(&self) -> bool {
        self.selected
    }

    pub fn set_value(&mut self, input: &str) -> anyhow::Result<()> {
        self.input = Input::from(input);
        Ok(())
    }
}

#[derive(Debug)]
pub enum TextFieldMsg {
    AppendInput(InputRequest),
}

#[async_trait::async_trait]
impl Component for TextField {
    type Msg = TextFieldMsg;
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
        let masked_input;
        let input = if self.masked {
            masked_input = Input::from("*".repeat(self.input.value().chars().count()))
                .with_cursor(self.input.cursor());
            &masked_input
        } else {
            &self.input
        };
        f.render_widget(
            TextInput::new(input)
                .block(block)
                .show_cursor(self.selected),
            rect,
        );
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            TextFieldMsg::AppendInput(request) => {
                self.input.handle(request);
            }
        };
        Ok(ComponentReturn::empty())
    }

    fn handle_event(
        &mut self,
        evt: crate::components::ComponentEvent,
    ) -> anyhow::Result<Vec<crate::components::ComponentMsg<Self::Msg>>> {
        match evt {
            ComponentEvent::Event(Event::Key(key)) if key.code != KeyCode::Enter => {
                Ok(to_input_request(&Event::Key(key))
                    .map(|request| TextFieldMsg::AppendInput(request).into())
                    .into_iter()
                    .collect())
            }
            _ => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn render(field: &mut TextField) -> String {
        let mut terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
        terminal.draw(|f| field.view(f, f.area())).unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..buffer.area.width)
            .map(|x| buffer[(x, 1)].symbol().to_string())
            .collect()
    }

    #[test]
    fn masked_field_hides_its_value() {
        let mut field = TextField::builder()
            .name("secret".to_string())
            .label("Secret".to_string())
            .initial_value("abc".to_string())
            .masked(true)
            .build()
            .unwrap();
        let line = render(&mut field);
        assert!(line.contains("***"), "{line:?}");
        assert!(!line.contains("abc"), "{line:?}");
        assert_eq!(field.value(), "abc");
    }

    #[test]
    fn plain_field_shows_its_value() {
        let mut field = TextField::builder()
            .name("name".to_string())
            .label("Name".to_string())
            .initial_value("abc".to_string())
            .build()
            .unwrap();
        assert!(render(&mut field).contains("abc"));
    }
}
