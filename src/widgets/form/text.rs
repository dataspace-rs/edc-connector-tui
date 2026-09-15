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
}

impl TextField {
    pub fn builder() -> TextFieldBuilder {
        TextFieldBuilder::default()
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
        f.render_widget(
            TextInput::new(&self.input)
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
