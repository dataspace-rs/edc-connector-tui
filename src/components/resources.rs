use std::{fmt::Debug, sync::Arc};

use self::{msg::ResourcesMsg, resource::ResourceComponent};
use super::{
    table::{msg::TableMsg, TableEntry, UiTable},
    Action, Component, ComponentEvent, ComponentMsg, ComponentReturn, Notification,
};
use crate::types::{connector::Connector, info::InfoSheet};
use crossterm::event::{Event, KeyCode};
use edc_connector_client::types::query::Query;
use futures::future::BoxFuture;
use futures::FutureExt;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{block::Title, Block, Borders, Paragraph},
    Frame,
};
use serde::Serialize;
use std::future::Future;
pub mod msg;
pub mod resource;

pub type ResourceTable<T, R> = UiTable<T, Box<ResourcesMsg<T, R>>>;

pub type OnFetch<T> =
    Arc<dyn Fn(&Connector, Query) -> BoxFuture<'static, anyhow::Result<Vec<T>>> + Send + Sync>;

pub type OnSingleFetch<T, R> =
    Arc<dyn Fn(&Connector, T) -> BoxFuture<'static, anyhow::Result<R>> + Send + Sync>;

#[derive(Debug)]
pub enum Focus {
    ResourceList,
    Resource,
}

pub struct ResourcesComponent<T: TableEntry, R: DrawableResource> {
    table: ResourceTable<T, R>,
    resource: ResourceComponent<R>,
    focus: Focus,
    connector: Option<Connector>,
    on_fetch: Option<OnFetch<T>>,
    on_single_fetch: Option<OnSingleFetch<T, R>>,
    query: Query,
    page_size: u32,
}

impl<T: TableEntry + Send + Sync + 'static, R: DrawableResource + Send + Sync + 'static>
    ResourcesComponent<T, R>
{
    pub fn on_fetch<F, Fut>(mut self, on_fetch: F) -> Self
    where
        F: Fn(Connector, Query) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<Vec<T>>> + Send,
    {
        let handler = Arc::new(on_fetch);
        self.on_fetch = Some(Arc::new(move |conn, query| {
            let c = conn.clone();
            let inner_handler = handler.clone();
            async move { inner_handler(c, query).await }.boxed()
        }));

        self
    }

    pub fn on_single_fetch<F, Fut>(mut self, on_single_fetch: F) -> Self
    where
        F: Fn(Connector, T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<R>> + Send,
    {
        let handler = Arc::new(on_single_fetch);
        self.on_single_fetch = Some(Arc::new(move |conn, entity| {
            let c = conn.clone();
            let inner_handler = handler.clone();
            async move { inner_handler(c, entity).await }.boxed()
        }));

        self
    }

    pub fn info_sheet(&self) -> InfoSheet {
        match self.focus {
            Focus::ResourceList => self.table.info_sheet().merge(self.pagination_sheet()),
            Focus::Resource => self.resource.info_sheet(),
        }
    }

    fn pagination_sheet(&self) -> InfoSheet {
        InfoSheet::default()
            .key_binding("<n>", "Next Page")
            .key_binding("<p>", "Prev page")
            .key_binding("<r>", "Refresh page")
    }

    fn fetch(&self) -> anyhow::Result<ComponentReturn<ResourcesMsg<T, R>>> {
        if let (Some(connector), Some(on_fetch)) = (self.connector.as_ref(), self.on_fetch.as_ref())
        {
            let query = self.query.clone();

            let connector = connector.clone();
            let on_fetch = on_fetch.clone();
            Ok(ComponentReturn::cmd(
                async move {
                    match on_fetch(&connector, query).await {
                        Ok(elements) => Ok(vec![ResourcesMsg::ResourcesFetched(elements).into()]),
                        Err(err) => Ok(vec![
                            ResourcesMsg::ResourcesFetchFailed(err.to_string()).into()
                        ]),
                    }
                }
                .boxed(),
            ))
        } else {
            Ok(ComponentReturn::empty())
        }
    }

    fn single_fetch(&self, selected: T) -> anyhow::Result<ComponentReturn<ResourcesMsg<T, R>>> {
        if let (Some(connector), Some(on_single_fetch)) =
            (self.connector.as_ref(), self.on_single_fetch.as_ref())
        {
            let connector = connector.clone();
            let on_single_fetch = on_single_fetch.clone();
            Ok(ComponentReturn::cmd(
                async move {
                    match on_single_fetch(&connector, selected).await {
                        Ok(element) => Ok(vec![ResourcesMsg::ResourceFetched(element).into()]),
                        Err(err) => Ok(vec![
                            ResourcesMsg::ResourcesFetchFailed(err.to_string()).into()
                        ]),
                    }
                }
                .boxed(),
            ))
        } else {
            Ok(ComponentReturn::empty())
        }
    }

    fn view_table(&mut self, f: &mut Frame, area: Rect) {
        let styled_text =
            Span::styled(format!(" {} ", R::title()), Style::default().fg(Color::Red));
        let block = Block::default()
            .title(Title::from(styled_text).alignment(Alignment::Center))
            .borders(Borders::ALL);

        let new_area = block.inner(area);
        let constraints = vec![Constraint::Min(1), Constraint::Length(2)];
        let layout = Layout::vertical(constraints).split(new_area);
        self.table.view(f, layout[0]);
        self.render_footer(f, layout[1]);

        f.render_widget(block, area)
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let sort = self
            .query
            .sort()
            .map(|s| format!("{}[{:?}]", s.field(), s.order()))
            .unwrap_or_else(|| String::from("None"));
        let filter = self
            .query
            .filter_expression()
            .iter()
            .map(|criterion| {
                format!(
                    "{} {} {:?}",
                    criterion.operand_left(),
                    criterion.operator(),
                    criterion.operand_right()
                )
            })
            .collect::<Vec<_>>();

        let text = format!(
            "Offset: {} | Limit: {} | Sort: {} | Filter: [{}]",
            self.query.offset(),
            self.query.limit(),
            sort,
            filter.join(" , ")
        );
        let info_footer = Paragraph::new(Line::from(text))
            .centered()
            .block(Block::default().borders(Borders::TOP));

        frame.render_widget(info_footer, area);
    }
}

impl<T: TableEntry + Clone, R: DrawableResource> Default for ResourcesComponent<T, R> {
    fn default() -> Self {
        Self {
            table: ResourceTable::new(R::title().to_string())
                .on_select(|res: &T| Box::new(ResourcesMsg::ResourceSelected(res.clone()))),
            resource: ResourceComponent::new(R::title().to_string()),
            focus: Focus::ResourceList,
            connector: None,
            on_fetch: None,
            on_single_fetch: None,
            query: Query::default(),
            page_size: 50,
        }
    }
}

#[async_trait::async_trait]
impl<T: TableEntry + Send + Sync + 'static, R: DrawableResource + Send + Sync + 'static> Component
    for ResourcesComponent<T, R>
{
    type Msg = ResourcesMsg<T, R>;
    type Props = Connector;

    async fn init(&mut self, props: Self::Props) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        self.connector = Some(props.clone());
        self.fetch()
    }

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        match self.focus {
            Focus::ResourceList => self.view_table(f, rect),
            Focus::Resource => self.resource.view(f, rect),
        }
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            ResourcesMsg::ResourceFetched(resource) => {
                self.resource.update_resource(Some(resource));
                self.focus = Focus::Resource;
                Ok(ComponentReturn::action(Action::ChangeSheet))
            }
            ResourcesMsg::ResourceSelected(selected) => self.single_fetch(selected),
            ResourcesMsg::TableEvent(table) => {
                Self::forward_update(&mut self.table, table.into(), ResourcesMsg::TableEvent).await
            }
            ResourcesMsg::ResourcesFetched(resources) => {
                self.table.update_elements(resources);
                Ok(ComponentReturn::empty())
            }
            ResourcesMsg::Back => {
                self.focus = Focus::ResourceList;
                Ok(ComponentReturn::action(Action::ChangeSheet))
            }
            ResourcesMsg::NextPage => {
                if self.table.elements().len() as u32 == self.page_size {
                    self.query = self
                        .query
                        .to_builder()
                        .offset(self.query.offset() + self.page_size)
                        .build();

                    self.fetch()
                } else {
                    Ok(ComponentReturn::empty())
                }
            }
            ResourcesMsg::PrevPage => {
                if self.query.offset() > 0 {
                    self.query = self
                        .query
                        .to_builder()
                        .offset(self.query.offset() - self.page_size)
                        .build();

                    self.fetch()
                } else {
                    Ok(ComponentReturn::empty())
                }
            }
            ResourcesMsg::RefreshPage => self.fetch(),
            ResourcesMsg::ResourceMsg(msg) => {
                Self::forward_update(&mut self.resource, msg.into(), ResourcesMsg::ResourceMsg)
                    .await
            }
            ResourcesMsg::ResourcesFetchFailed(error) => Ok(ComponentReturn::action(
                Action::Notification(Notification::error(error)),
            )),
        }
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        match self.focus {
            Focus::ResourceList => match evt {
                ComponentEvent::Event(Event::Key(key)) if key.code == KeyCode::Char('n') => {
                    Ok(vec![ResourcesMsg::NextPage.into()])
                }
                ComponentEvent::Event(Event::Key(key)) if key.code == KeyCode::Char('p') => {
                    Ok(vec![ResourcesMsg::PrevPage.into()])
                }
                ComponentEvent::Event(Event::Key(key)) if key.code == KeyCode::Char('r') => {
                    Ok(vec![ResourcesMsg::RefreshPage.into()])
                }
                _ => Self::forward_event(&mut self.table, evt, |msg| match msg {
                    TableMsg::Local(table) => ResourcesMsg::TableEvent(TableMsg::Local(table)),
                    TableMsg::Outer(outer) => *outer,
                }),
            },
            Focus::Resource => match evt {
                ComponentEvent::Event(Event::Key(k)) if k.code == KeyCode::Esc => {
                    Ok(vec![ResourcesMsg::Back.into()])
                }
                _ => Self::forward_event(&mut self.resource, evt, ResourcesMsg::ResourceMsg),
            },
        }
    }
}

pub trait DrawableResource {
    fn id(&self) -> &str;

    fn title() -> &'static str;

    fn fields(&self) -> Vec<Field>;
}

pub struct Field {
    name: String,
    value: FieldValue,
}

impl Field {
    pub fn new(name: String, value: FieldValue) -> Self {
        Self { name, value }
    }

    pub fn string(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: FieldValue::Str(value.into()),
        }
    }

    pub fn json<T: Serialize + ?Sized>(name: impl Into<String>, value: &T) -> Self {
        Self {
            name: name.into(),
            value: FieldValue::Json(serde_json::to_string_pretty(value).unwrap()),
        }
    }
}

pub enum FieldValue {
    Str(String),
    Json(String),
}

impl AsRef<str> for FieldValue {
    fn as_ref(&self) -> &str {
        match self {
            FieldValue::Str(s) => s,
            FieldValue::Json(s) => s,
        }
    }
}
