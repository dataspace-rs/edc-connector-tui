use derive_builder::Builder;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};
use tui_textarea::TextArea;

use crate::components::Component;

use super::msg::FieldMsg;

#[derive(Builder)]
pub struct TextField {
    name: String,
    #[builder(default)]
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
                .title(self.name.clone().unwrap_or_default()),
        );
        text.set_cursor_line_style(Style::default());

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
                .title(self.name.clone()),
        );
        self.text.set_cursor_line_style(Style::default());

        if !self.selected {
            self.text.set_cursor_style(Style::default());
        }
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
        self.configure_text_area();
    }
}

pub enum TextFieldMsg {}

impl Component for TextField {
    type Msg = FieldMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        f.render_widget(&self.text, rect);
    }
}
