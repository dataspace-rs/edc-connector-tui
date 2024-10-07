use edc_connector_client::types::query::{Query, SortOrder};
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{block::Title, Block, Borders, Clear},
    Frame,
};

use crate::{
    components::{Component, ComponentEvent, ComponentMsg, ComponentReturn},
    widgets::form::{msg::FormMsg, row::RowField, text::TextField, Form},
};

pub type OnConfirm<M> = Box<dyn Fn(Query) -> M + Send + Sync>;

pub struct Filter<M> {
    query: Query,
    on_confirm: Option<OnConfirm<M>>,
    form: Form,
}

impl<M> Filter<M> {
    pub fn new(query: Query) -> Self {
        let form = Self::form(&query);
        Self {
            query,
            on_confirm: None,
            form,
        }
    }

    fn form(query: &Query) -> Form {
        Form::default()
            .field(
                TextField::builder()
                    .label("Limit".to_string())
                    .initial_value(query.limit().to_string())
                    .selected(true)
                    .build()
                    .unwrap(),
            )
            .field(
                TextField::builder()
                    .label("Sort Field".to_string())
                    .build()
                    .unwrap(),
            )
            .field(
                TextField::builder()
                    .label("Sort Order".to_string())
                    .initial_value("ASC".to_string())
                    .build()
                    .unwrap(),
            )
            .field(
                RowField::default()
                    .field(
                        TextField::builder()
                            .label("LeftOperand".to_string())
                            .build()
                            .unwrap(),
                    )
                    .field(
                        TextField::builder()
                            .label("Operator".to_string())
                            .build()
                            .unwrap(),
                    )
                    .field(
                        TextField::builder()
                            .label("RightOperand".to_string())
                            .build()
                            .unwrap(),
                    ),
            )
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

    // fn parse_fields(&self) -> anyhow::Result<Query> {
    //     let mut query = self.query.to_builder();

    //     if let Some(limit) = self.fields[0].text.lines().first() {
    //         query = query.limit(limit.parse()?);
    //     }

    //     match (
    //         self.fields[1].text.lines().first(),
    //         self.fields[2].text.lines().first(),
    //     ) {
    //         (Some(sort_field), Some(sort_order)) => {
    //             if !sort_order.is_empty() && !sort_field.is_empty() {
    //                 let order = match sort_order.as_str() {
    //                     "ASC" => SortOrder::Asc,
    //                     "DESC" => SortOrder::Desc,
    //                     _ => anyhow::bail!(
    //                         "Sort order {} not supported, expected ASC or DESC",
    //                         sort_order
    //                     ),
    //                 };
    //                 query = query.sort(&sort_field, order);
    //             }
    //         }
    //         _ => {}
    //     }

    //     Ok(query.build())
    // }

    pub fn set_query(&mut self, query: Query) {
        // self.query = query;
        // self.fields = Self::fields(&self.query);
        // self.selected_field = 0;
        // self.highlight_confirm = false;
    }
}

impl<M: Send + Sync + 'static> Filter<M> {
    fn handle_confirm(&mut self) -> anyhow::Result<ComponentReturn<FilterMsg<M>>> {
        // if let Some(cb) = self.on_confirm.as_ref() {
        //     match self.parse_fields() {
        //         Ok(query) => {
        //             self.query = query;
        //             return Ok(ComponentReturn::msg(ComponentMsg(FilterMsg::Outer(cb(
        //                 self.query.clone(),
        //             )))));
        //         }
        //         Err(err) => {
        //             return Ok(ComponentReturn::action(Action::Notification(
        //                 Notification::error(err.to_string()),
        //             )))
        //         }
        //     }
        // }
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
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            FilterMsg::Local(FilterLocalMsg::Form(form)) => {
                Self::forward_update(&mut self.form, form.into(), |msg| match msg {
                    FormMsg::Local(local) => {
                        FilterMsg::Local(FilterLocalMsg::Form(FormMsg::Local(local)))
                    }
                })
                .await?;
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
    }
}
