use crossterm::event::{Event, KeyCode};
use derive_builder::Builder;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};
use tui_textarea::{Input, TextArea};

use crate::components::{Component, ComponentEvent, ComponentMsg, ComponentReturn};

#[derive(Builder, Clone)]
pub struct TextField {
    label: String,
    name: String,
    #[builder(default)]
    #[allow(dead_code)]
    initial_value: String,
    #[builder(default = "self.default_text_area()?")]
    text: TextArea<'static>,
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
        self.text.lines().join("")
    }
}

impl TextFieldBuilder {
    fn default_text_area(&self) -> Result<TextArea<'static>, String> {
        let mut text = TextArea::default();
        let selected = self.selected.unwrap_or_default();
        let border_style = if selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        text.set_block(
            Block::default()
                .borders(Borders::all())
                .border_style(border_style)
                .title(self.label.clone().unwrap_or_default()),
        );
        text.set_cursor_line_style(Style::default());
        text.insert_str(self.initial_value.as_deref().unwrap_or_default());

        if !selected {
            text.set_cursor_style(Style::default());
        }

        Ok(text)
    }
}

impl TextField {
    fn configure_text_area(&mut self) {
        let border_style = if self.selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        self.text.set_block(
            Block::default()
                .borders(Borders::all())
                .border_style(border_style)
                .title(self.label.clone()),
        );
        self.text.set_cursor_line_style(Style::default());

        if !self.selected {
            self.text.set_cursor_style(Style::default());
        } else {
            self.text
                .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED))
        }
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
        self.configure_text_area();
    }

    pub fn set_value(&mut self, input: &str) -> anyhow::Result<()> {
        self.text.move_cursor(tui_textarea::CursorMove::Head);
        self.text.delete_line_by_end();
        self.text.insert_str(input);

        Ok(())
    }

    fn append_input(&mut self, input: Input) {
        self.text.input(input);
    }
}

#[derive(Debug)]
pub enum TextFieldMsg {
    AppendInput(Input),
}

#[async_trait::async_trait]
impl Component for TextField {
    type Msg = TextFieldMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        f.render_widget(&self.text, rect);
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            TextFieldMsg::AppendInput(input) => self.append_input(input),
        };
        Ok(ComponentReturn::empty())
    }

    fn handle_event(
        &mut self,
        evt: crate::components::ComponentEvent,
    ) -> anyhow::Result<Vec<crate::components::ComponentMsg<Self::Msg>>> {
        match evt {
            ComponentEvent::Event(Event::Key(key)) if key.code != KeyCode::Enter => {
                Ok(vec![TextFieldMsg::AppendInput(key.into()).into()])
            }
            _ => Ok(vec![]),
        }
    }
}
