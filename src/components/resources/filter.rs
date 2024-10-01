use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use edc_connector_client::types::query::Query;
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{block::Title, Block, Borders, Clear, Widget},
    Frame,
};
use tui_textarea::{Input, TextArea};

use crate::components::{Component, ComponentEvent, ComponentMsg, ComponentReturn};

pub type OnConfirm<M> = Box<dyn Fn(Query) -> M + Send + Sync>;

pub struct Filter<M> {
    query: Query,
    fields: Vec<FormField>,
    on_confirm: Option<OnConfirm<M>>,
    selected_field: usize,
}

impl<M> Filter<M> {
    pub fn new(query: Query) -> Self {
        let mut limit = TextArea::default();
        limit.insert_str(format!("{}", query.limit()));
        Self {
            query,
            fields: vec![
                FormField::new("Limit", true),
                FormField::new("Sort Field", false),
                FormField::new("Sort Order", false),
            ],
            on_confirm: None,
            selected_field: 0,
        }
    }

    pub fn query(&self) -> &Query {
        &self.query
    }

    pub fn next_page(&mut self) {
        self.query = self
            .query
            .to_builder()
            .offset(self.query.offset() + self.query.limit())
            .build();
    }

    pub fn prev_page(&mut self) {
        self.query = self
            .query
            .to_builder()
            .offset(self.query.offset() - self.query.limit())
            .build();
    }

    fn popup_area(&self, area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }

    fn append_input(&mut self, input: Input) {
        self.fields[self.selected_field].text.input(input);
    }

    fn move_up(&mut self) {
        if self.selected_field != 0 {
            self.fields[self.selected_field].set_selected(false);
            self.selected_field -= 1;
            self.fields[self.selected_field].set_selected(true);
        }
    }

    fn move_down(&mut self) {
        if self.selected_field != self.fields.len() - 1 {
            self.fields[self.selected_field].set_selected(false);
            self.selected_field += 1;
            self.fields[self.selected_field].set_selected(true);
        };
    }

    fn handle_key(&self, key: KeyEvent) -> Vec<ComponentMsg<FilterMsg<M>>> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('j'), KeyModifiers::CONTROL) | (KeyCode::Down, _) => {
                vec![(ComponentMsg(FilterMsg::Local(FilterLocalMsg::MoveDown)))]
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) | (KeyCode::Up, _) => {
                vec![(ComponentMsg(FilterMsg::Local(FilterLocalMsg::MoveUp)))]
            }
            _ => vec![(ComponentMsg(FilterMsg::Local(FilterLocalMsg::AppendInput(key.into()))))],
        }
    }
}

#[derive(Debug)]
pub enum FilterMsg<M> {
    Local(FilterLocalMsg),
    Outer(M),
}

#[derive(Debug)]
pub enum FilterLocalMsg {
    MoveUp,
    MoveDown,
    AppendInput(Input),
}

#[async_trait::async_trait]
impl<M: Send + Sync + 'static> Component for Filter<M> {
    type Msg = FilterMsg<M>;
    type Props = Query;

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        match evt {
            ComponentEvent::Event(Event::Key(key)) => Ok(self.handle_key(key)),
            _ => Ok(vec![]),
        }
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            FilterMsg::Local(FilterLocalMsg::MoveDown) => self.move_down(),
            FilterMsg::Local(FilterLocalMsg::MoveUp) => self.move_up(),
            FilterMsg::Local(FilterLocalMsg::AppendInput(input)) => self.append_input(input),
            FilterMsg::Outer(_) => todo!(),
        };

        Ok(ComponentReturn::empty())
    }
    fn view(&mut self, f: &mut Frame, _rect: Rect) {
        let area = f.area();

        let styled_text = Span::styled(" Filters ", Style::default().fg(Color::Red));
        let block = Block::default()
            .title(Title::from(styled_text).alignment(Alignment::Center))
            .borders(Borders::ALL);
        let area = self.popup_area(area, 30, 50);

        let content = block.inner(area);
        f.render_widget(Clear, area); //this clears out the background
        f.render_widget(block, area);

        let constraints = (0..self.fields.len())
            .map(|_| Constraint::Length(3))
            .collect::<Vec<_>>();

        let layouts = Layout::vertical(constraints).split(content);

        for (idx, field) in self.fields.iter().enumerate() {
            f.render_widget(field, layouts[idx]);
        }
    }
}

pub struct FormField {
    name: String,
    text: TextArea<'static>,
    selected: bool,
}

impl FormField {
    pub fn new(name: &str, selected: bool) -> Self {
        let mut field = Self {
            name: name.to_string(),
            text: TextArea::default(),
            selected,
        };

        field.configure_text_area();

        field
    }

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

impl Widget for &FormField {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        self.text.render(area, buf)
    }
}
