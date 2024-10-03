use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use edc_connector_client::types::query::{Query, SortOrder};
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Clear, Paragraph, Widget},
    Frame,
};
use tui_textarea::{Input, TextArea};

use crate::{
    components::{Action, Component, ComponentEvent, ComponentMsg, ComponentReturn, Notification},
    widgets::form::{msg::FormMsg, text::TextField, FieldComponent, Form},
};

pub type OnConfirm<M> = Box<dyn Fn(Query) -> M + Send + Sync>;

pub struct Filter<M> {
    query: Query,
    fields: Vec<FormField>,
    on_confirm: Option<OnConfirm<M>>,
    selected_field: usize,
    highlight_confirm: bool,
    form: Form,
}

impl<M> Filter<M> {
    pub fn new(query: Query) -> Self {
        let form = Self::form(&query);
        let fields = Self::fields(&query);
        Self {
            query,
            fields,
            on_confirm: None,
            selected_field: 0,
            highlight_confirm: false,
            form,
        }
    }

    fn form(query: &Query) -> Form {
        Form::default()
            .field(
                TextField::builder()
                    .name("Limit".to_string())
                    .initial_value(query.limit().to_string())
                    .selected(true)
                    .build()
                    .unwrap(),
            )
            .field(
                TextField::builder()
                    .name("Sort Field".to_string())
                    .build()
                    .unwrap(),
            )
            .field(
                TextField::builder()
                    .name("Sort Order".to_string())
                    .initial_value("ASC".to_string())
                    .build()
                    .unwrap(),
            )
    }

    fn fields(query: &Query) -> Vec<FormField> {
        vec![
            FormField::with_initial("Limit", &query.limit().to_string(), true),
            FormField::new("Sort Field", false),
            FormField::with_initial("Sort Order", "ASC", false),
        ]
    }

    pub fn on_confirm(mut self, cb: impl Fn(Query) -> M + Send + Sync + 'static) -> Self {
        self.on_confirm = Some(Box::new(cb));
        self
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
        if self.highlight_confirm {
            self.selected_field = self.fields.len() - 1;
            self.fields[self.selected_field].set_selected(true);
            self.highlight_confirm = false;
        } else {
            if self.selected_field != 0 {
                self.fields[self.selected_field].set_selected(false);
                self.selected_field -= 1;
                self.fields[self.selected_field].set_selected(true);
            } else {
                self.highlight_confirm = true;
                self.fields[0].set_selected(false);
            }
        }
    }

    fn move_down(&mut self) {
        let len = self.fields.len() - 1;
        if self.highlight_confirm {
            self.selected_field = 0;
            self.fields[self.selected_field].set_selected(true);
            self.highlight_confirm = false;
        } else {
            if self.selected_field != len {
                self.fields[self.selected_field].set_selected(false);
                self.selected_field += 1;
                self.fields[self.selected_field].set_selected(true);
            } else {
                self.highlight_confirm = true;
                self.fields[len].set_selected(false);
            }
        }
    }

    fn parse_fields(&self) -> anyhow::Result<Query> {
        let mut query = self.query.to_builder();

        if let Some(limit) = self.fields[0].text.lines().first() {
            query = query.limit(limit.parse()?);
        }

        match (
            self.fields[1].text.lines().first(),
            self.fields[2].text.lines().first(),
        ) {
            (Some(sort_field), Some(sort_order)) => {
                if !sort_order.is_empty() && !sort_field.is_empty() {
                    let order = match sort_order.as_str() {
                        "ASC" => SortOrder::Asc,
                        "DESC" => SortOrder::Desc,
                        _ => anyhow::bail!(
                            "Sort order {} not supported, expected ASC or DESC",
                            sort_order
                        ),
                    };
                    query = query.sort(&sort_field, order);
                }
            }
            _ => {}
        }

        Ok(query.build())
    }

    fn handle_key(&self, key: KeyEvent) -> Vec<ComponentMsg<FilterMsg<M>>> {
        match (key.code, key.modifiers, self.highlight_confirm) {
            (KeyCode::Char('j'), KeyModifiers::CONTROL, _)
            | (KeyCode::Down, _, _)
            | (KeyCode::Tab, _, _)
            | (KeyCode::Enter, _, false) => {
                vec![(ComponentMsg(FilterMsg::Local(FilterLocalMsg::MoveDown)))]
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL, _) | (KeyCode::Up, _, _) => {
                vec![(ComponentMsg(FilterMsg::Local(FilterLocalMsg::MoveUp)))]
            }
            (KeyCode::Enter, _, true) => {
                vec![ComponentMsg(FilterMsg::Local(FilterLocalMsg::Confirm))]
            }
            _ => vec![(ComponentMsg(FilterMsg::Local(FilterLocalMsg::AppendInput(key.into()))))],
        }
    }

    pub fn set_query(&mut self, query: Query) {
        self.query = query;
        self.fields = Self::fields(&self.query);
        self.selected_field = 0;
        self.highlight_confirm = false;
    }
}

impl<M: Send + Sync + 'static> Filter<M> {
    fn handle_confirm(&mut self) -> anyhow::Result<ComponentReturn<FilterMsg<M>>> {
        if let Some(cb) = self.on_confirm.as_ref() {
            match self.parse_fields() {
                Ok(query) => {
                    self.query = query;
                    return Ok(ComponentReturn::msg(ComponentMsg(FilterMsg::Outer(cb(
                        self.query.clone(),
                    )))));
                }
                Err(err) => {
                    return Ok(ComponentReturn::action(Action::Notification(
                        Notification::error(err.to_string()),
                    )))
                }
            }
        }
        Ok(ComponentReturn::empty())
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
    Confirm,
    Form(FormMsg),
}

#[async_trait::async_trait]
impl<M: Send + Sync + 'static> Component for Filter<M> {
    type Msg = FilterMsg<M>;
    type Props = Query;

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        Self::forward_event(&mut self.form, evt, |msg| match msg {
            FormMsg::Local(local) => FilterMsg::Local(FilterLocalMsg::Form(FormMsg::Local(local))),
        })
        // match evt {
        //     ComponentEvent::Event(Event::Key(key)) => Ok(self.handle_key(key)),
        //     _ => Ok(vec![]),
        // }
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            FilterMsg::Local(FilterLocalMsg::MoveDown) => self.move_down(),
            FilterMsg::Local(FilterLocalMsg::MoveUp) => self.move_up(),
            FilterMsg::Local(FilterLocalMsg::AppendInput(input)) => self.append_input(input),
            FilterMsg::Local(FilterLocalMsg::Confirm) => return self.handle_confirm(),
            FilterMsg::Local(FilterLocalMsg::Form(form)) => {
                Self::forward_update(&mut self.form, form.into(), |msg| match msg {
                    FormMsg::Local(local) => {
                        FilterMsg::Local(FilterLocalMsg::Form(FormMsg::Local(local)))
                    }
                }).await?;
            }
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

        self.form.view(f, content);

        // let constraints = (0..=self.fields.len())
        //     .map(|_| Constraint::Length(3))
        //     .collect::<Vec<_>>();

        // let layouts = Layout::vertical(constraints).split(content);

        // for (idx, field) in self.fields.iter().enumerate() {
        //     f.render_widget(field, layouts[idx]);
        // }

        // let style = if self.highlight_confirm {
        //     Style::default().fg(Color::Yellow)
        // } else {
        //     Style::default()
        // };

        // let styled_text = Span::styled("Confirm", style);

        // let confirm = Paragraph::new(Line::from(styled_text))
        //     .centered()
        //     .block(Block::default().borders(Borders::TOP));

        // f.render_widget(confirm, layouts[self.fields.len()]);
    }
}

pub struct FormField {
    name: String,
    text: TextArea<'static>,
    selected: bool,
}

impl FormField {
    pub fn new(name: &str, selected: bool) -> Self {
        Self::with_initial(name, "", selected)
    }
    pub fn with_initial(name: &str, value: &str, selected: bool) -> Self {
        let mut field = Self {
            name: name.to_string(),
            text: TextArea::default(),
            selected,
        };

        field.text.insert_str(value);
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
