use std::collections::HashMap;

use edc_connector_client::types::query::{Query, SortOrder};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear},
    Frame,
};

use crate::{
    components::{Component, ComponentEvent, ComponentMsg, ComponentReturn},
    widgets::form::{
        msg::FormMsg, row::RowField, text::TextField, ChangeSet, FieldComponent, Form,
    },
};

pub type OnConfirm<M> = Box<dyn Fn(Query) -> M + Send + Sync>;

pub struct Filter<M> {
    query: Query,
    on_confirm: Option<OnConfirm<M>>,
    form: Form<FilterFormMsg>,
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

    fn form(query: &Query) -> Form<FilterFormMsg> {
        Form::default()
            .field(
                TextField::builder()
                    .name("limit".to_string())
                    .label("Limit".to_string())
                    .initial_value(query.limit().to_string())
                    .selected(true)
                    .build()
                    .unwrap(),
            )
            .field(
                TextField::builder()
                    .name("sort_field".to_string())
                    .label("Sort Field".to_string())
                    .build()
                    .unwrap(),
            )
            .field(
                TextField::builder()
                    .name("sort_order".to_string())
                    .label("Sort Order".to_string())
                    .initial_value("ASC".to_string())
                    .build()
                    .unwrap(),
            )
            .field(
                RowField::default()
                    .name("filter_0")
                    .field(
                        TextField::builder()
                            .name("left_operand".to_string())
                            .label("LeftOperand".to_string())
                            .build()
                            .unwrap(),
                    )
                    .field(
                        TextField::builder()
                            .name("operator".to_string())
                            .label("Operator".to_string())
                            .initial_value("=".to_string())
                            .build()
                            .unwrap(),
                    )
                    .field(
                        TextField::builder()
                            .name("right_operand".to_string())
                            .label("RightOperand".to_string())
                            .build()
                            .unwrap(),
                    ),
            )
            .field(
                RowField::default()
                    .name("filter_1")
                    .field(
                        TextField::builder()
                            .name("left_operand".to_string())
                            .label("LeftOperand".to_string())
                            .build()
                            .unwrap(),
                    )
                    .field(
                        TextField::builder()
                            .name("operator".to_string())
                            .label("Operator".to_string())
                            .initial_value("=".to_string())
                            .build()
                            .unwrap(),
                    )
                    .field(
                        TextField::builder()
                            .name("right_operand".to_string())
                            .label("RightOperand".to_string())
                            .build()
                            .unwrap(),
                    ),
            )
            .on_confirm(|fields| Self::parse_fields(fields).map(FilterFormMsg::Changed))
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

    fn parse_fields(fields: HashMap<String, FieldComponent>) -> anyhow::Result<Query> {
        let limit: String = fields["limit"].clone().try_into()?;
        let sort_field: String = fields["sort_field"].clone().try_into()?;
        let sort_order = fields["sort_order"]
            .clone()
            .try_into()
            .and_then(|order: String| match order.as_str() {
                "ASC" => Ok(SortOrder::Asc),
                "DESC" => Ok(SortOrder::Desc),
                wrong => anyhow::bail!("Sort order {} not supported, expected ASC or DESC", wrong),
            })?;

        let mut query = Query::builder().limit(limit.parse()?);

        if !sort_field.is_empty() {
            query = query.sort(&sort_field, sort_order);
        }

        let filter_1: Criteria = fields["filter_0"].clone().try_into()?;
        let filter_2: Criteria = fields["filter_1"].clone().try_into()?;

        if filter_1.is_valid() {
            query = query.filter(&filter_1.field, &filter_1.operator, filter_1.value);
        }

        if filter_2.is_valid() {
            query = query.filter(&filter_2.field, &filter_2.operator, filter_2.value);
        }

        Ok(query.build())
    }

    pub fn set_query(&mut self, query: Query) -> anyhow::Result<()> {
        self.form.change_field("limit", query.limit())?;

        if let Some(s) = query.sort() {
            self.form.change_field("sort_field", s.field())?;

            let order = match s.order() {
                SortOrder::Asc => "ASC",
                SortOrder::Desc => "DESC",
            };
            self.form.change_field("sort_order", order)?;
        } else {
            self.form.change_field("sort_order", "ASC")?;
        }

        for (idx, f) in query.filter_expression().iter().enumerate() {
            let value: String = f.operand_right().try_from()?;

            self.form.change_field(
                &format!("filter_{}", idx),
                ChangeSet::row(vec![
                    ("left_operand", f.operand_left().into()),
                    ("operator", f.operator().into()),
                    ("right_operand", value.into()),
                ]),
            )?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum FilterMsg<M> {
    Local(FilterLocalMsg),
    Outer(M),
}

#[derive(Debug)]
pub enum FilterLocalMsg {
    Form(FormMsg<FilterFormMsg>),
    FilterForm(FilterFormMsg),
}

#[derive(Debug)]
pub enum FilterFormMsg {
    Changed(Query),
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
            FormMsg::Outer(_) => todo!(),
        })
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            FilterMsg::Local(FilterLocalMsg::Form(form)) => {
                return Self::forward_update(&mut self.form, form.into(), |msg| match msg {
                    FormMsg::Local(local) => {
                        FilterMsg::Local(FilterLocalMsg::Form(FormMsg::Local(local)))
                    }
                    FormMsg::Outer(msg) => FilterMsg::Local(FilterLocalMsg::FilterForm(msg)),
                })
                .await;
            }
            FilterMsg::Local(FilterLocalMsg::FilterForm(FilterFormMsg::Changed(query))) => {
                self.query = query.clone();
                if let Some(cb) = self.on_confirm.as_ref() {
                    return Ok(ComponentReturn::msg(ComponentMsg(FilterMsg::Outer(cb(
                        query.clone(),
                    )))));
                }
            }
            FilterMsg::Outer(_) => todo!(),
        };

        Ok(ComponentReturn::empty())
    }
    fn view(&mut self, f: &mut Frame, _rect: Rect) {
        let area = f.area();

        let styled_text = Span::styled(" Filters ", Style::default().fg(Color::Red));
        let block = Block::default()
            .title_top(Line::from(styled_text).centered())
            .borders(Borders::ALL);
        let area = self.popup_area(area, 30, 50);

        let content = block.inner(area);
        f.render_widget(Clear, area); //this clears out the background
        f.render_widget(block, area);

        self.form.view(f, content);
    }
}

struct Criteria {
    field: String,
    operator: String,
    value: String,
}

impl Criteria {
    fn is_valid(&self) -> bool {
        !self.field.is_empty() && !self.operator.is_empty() && !self.value.is_empty()
    }
}

impl TryInto<Criteria> for FieldComponent {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Criteria, Self::Error> {
        match self {
            FieldComponent::Text(_) => {
                anyhow::bail!("Cannot extract a filter from text field")
            }
            FieldComponent::Row(row_field) => {
                let fields = row_field.as_map();
                Ok(Criteria {
                    field: fields["left_operand"].clone().try_into()?,
                    operator: fields["operator"].clone().try_into()?,
                    value: fields["right_operand"].clone().try_into()?,
                })
            }
        }
    }
}
