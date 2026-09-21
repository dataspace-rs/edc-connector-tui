use std::{fmt::Debug, sync::Arc};

use self::{
    msg::{ActionInput, ResourcesMsg},
    resource::ResourceComponent,
};
use super::{
    table::{msg::TableMsg, TableEntry, UiTable},
    Action, Component, ComponentEvent, ComponentMsg, ComponentReturn, Notification,
};
use crate::{
    types::{connector::Connector, info::InfoSheet},
    widgets::{
        form::{msg::FormMsg, Form},
        popup,
    },
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use edc_connector_client::types::query::Query;
use filter::{Filter, FilterMsg};
use futures::future::BoxFuture;
use futures::FutureExt;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use serde::Serialize;
use std::future::Future;
pub mod filter;
pub mod msg;
pub mod resource;

pub type ResourceTable<T, R> = UiTable<T, Box<ResourcesMsg<T, R>>>;

pub type OnFetch<T> =
    Arc<dyn Fn(&Connector, Query) -> BoxFuture<'static, anyhow::Result<Vec<T>>> + Send + Sync>;

pub type OnSingleFetch<T, R> =
    Arc<dyn Fn(&Connector, T) -> BoxFuture<'static, anyhow::Result<R>> + Send + Sync>;

/// A create/update/delete/action call; `Ok` carries the text of the notification to show.
pub type OnMutate<T> =
    Arc<dyn Fn(&Connector, T) -> BoxFuture<'static, anyhow::Result<String>> + Send + Sync>;

pub type OnAction<T> = Arc<
    dyn Fn(&Connector, T, ActionInput) -> BoxFuture<'static, anyhow::Result<String>> + Send + Sync,
>;

/// Builds the add (`None`) or edit (`Some`) form of a resource.
pub type FormFactory<T> = Arc<dyn Fn(Option<&T>) -> Form<T> + Send + Sync>;

/// Builds the form of an action on `T`; the loaded detail view is passed when there is one.
pub type ActionFormFactory<T, R> = Arc<dyn Fn(&T, Option<&R>) -> Form<ActionInput> + Send + Sync>;

pub type Label<T> = Arc<dyn Fn(&T) -> String + Send + Sync>;

/// How the list is fetched, deciding which paging/filter controls make sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListMode {
    /// `POST .../request` with a full query: paging, sorting and filters.
    #[default]
    Query,
    /// `GET` with `offset`/`limit` only.
    Paged,
    /// Plain `GET` returning everything.
    Plain,
}

/// A resource specific operation bound to a key, e.g. testing a CEL expression.
pub struct ResourceAction<T, R> {
    key: char,
    label: &'static str,
    kind: ActionKind<T, R>,
}

pub enum ActionKind<T, R> {
    Immediate(OnMutate<T>),
    WithForm {
        form: ActionFormFactory<T, R>,
        run: OnAction<T>,
    },
}

impl<T: Clone + Send + Sync + 'static, R> ResourceAction<T, R> {
    /// An action that runs on the selected resource right away.
    pub fn immediate<F, Fut>(key: char, label: &'static str, run: F) -> Self
    where
        F: Fn(Connector, T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<String>> + Send,
    {
        Self {
            key,
            label,
            kind: ActionKind::Immediate(mutate(run)),
        }
    }

    /// An action that first asks for input through `form`.
    pub fn with_form<FF, F, Fut>(key: char, label: &'static str, form: FF, run: F) -> Self
    where
        FF: Fn(&T, Option<&R>) -> Form<ActionInput> + Send + Sync + 'static,
        F: Fn(Connector, T, ActionInput) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<String>> + Send,
    {
        let handler = Arc::new(run);
        Self {
            key,
            label,
            kind: ActionKind::WithForm {
                form: Arc::new(form),
                run: Arc::new(move |conn, entity, input| {
                    let c = conn.clone();
                    let inner_handler = handler.clone();
                    async move { inner_handler(c, entity, input).await }.boxed()
                }),
            },
        }
    }
}

fn mutate<T, F, Fut>(handler: F) -> OnMutate<T>
where
    T: Send + 'static,
    F: Fn(Connector, T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<String>> + Send,
{
    let handler = Arc::new(handler);
    Arc::new(move |conn, entity| {
        let c = conn.clone();
        let inner_handler = handler.clone();
        async move { inner_handler(c, entity).await }.boxed()
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Focus {
    ResourceList,
    Resource,
}

/// The popup open on top of the list/detail view.
#[derive(Default)]
enum Mode<T> {
    #[default]
    List,
    Form {
        form: Box<Form<T>>,
        title: String,
        editing: bool,
    },
    ConfirmDelete {
        target: T,
        label: String,
    },
    ActionForm {
        form: Box<Form<ActionInput>>,
        title: String,
        action: usize,
        target: T,
    },
}

struct LoadingState {
    request_id: u64,
    message: String,
    spinner_frame: usize,
}

impl LoadingState {
    const FRAMES: [&'static str; 4] = ["|", "/", "-", "\\"];

    fn spinner(&self) -> &'static str {
        Self::FRAMES[self.spinner_frame % Self::FRAMES.len()]
    }

    fn advance(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % Self::FRAMES.len();
    }
}

impl<T> Debug for Mode<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::List => write!(f, "List"),
            Mode::Form { title, editing, .. } => f
                .debug_struct("Form")
                .field("title", title)
                .field("editing", editing)
                .finish(),
            Mode::ConfirmDelete { label, .. } => f
                .debug_struct("ConfirmDelete")
                .field("label", label)
                .finish(),
            Mode::ActionForm { title, action, .. } => f
                .debug_struct("ActionForm")
                .field("title", title)
                .field("action", action)
                .finish(),
        }
    }
}

pub struct ResourcesComponent<T: TableEntry, R: DrawableResource> {
    table: ResourceTable<T, R>,
    resource: ResourceComponent<R>,
    filter: Filter<Box<ResourcesMsg<T, R>>>,
    query: Query,
    focus: Focus,
    show_filters: bool,
    connector: Option<Connector>,
    on_fetch: Option<OnFetch<T>>,
    on_single_fetch: Option<OnSingleFetch<T, R>>,
    loading: Option<LoadingState>,
    next_request_id: u64,
    /// The entry whose detail view is (being) shown.
    selected: Option<T>,
    mode: Mode<T>,
    list_mode: ListMode,
    form_factory: Option<FormFactory<T>>,
    on_create: Option<OnMutate<T>>,
    on_update: Option<OnMutate<T>>,
    on_delete: Option<(Label<T>, OnMutate<T>)>,
    actions: Vec<ResourceAction<T, R>>,
}

impl<
        T: TableEntry + Clone + Send + Sync + 'static,
        R: DrawableResource + Send + Sync + 'static,
    > ResourcesComponent<T, R>
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

    pub fn list_mode(mut self, mode: ListMode) -> Self {
        self.list_mode = mode;
        self
    }

    /// Enables `a`/`e` with the given add/edit form.
    pub fn editable(
        mut self,
        form: impl Fn(Option<&T>) -> Form<T> + Send + Sync + 'static,
    ) -> Self {
        self.form_factory = Some(Arc::new(form));
        self
    }

    pub fn on_create<F, Fut>(mut self, on_create: F) -> Self
    where
        F: Fn(Connector, T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<String>> + Send,
    {
        self.on_create = Some(mutate(on_create));
        self
    }

    pub fn on_update<F, Fut>(mut self, on_update: F) -> Self
    where
        F: Fn(Connector, T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<String>> + Send,
    {
        self.on_update = Some(mutate(on_update));
        self
    }

    /// Enables `d`; `label` names the resource in the confirmation popup.
    pub fn on_delete<L, F, Fut>(mut self, label: L, on_delete: F) -> Self
    where
        L: Fn(&T) -> String + Send + Sync + 'static,
        F: Fn(Connector, T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<String>> + Send,
    {
        self.on_delete = Some((Arc::new(label), mutate(on_delete)));
        self
    }

    pub fn action(mut self, action: ResourceAction<T, R>) -> Self {
        self.actions.push(action);
        self
    }

    pub fn info_sheet(&self) -> InfoSheet {
        match (&self.mode, &self.focus) {
            (Mode::Form { .. }, _) => Form::<T>::key_bindings(),
            (Mode::ActionForm { .. }, _) => Form::<T>::key_bindings(),
            (Mode::ConfirmDelete { .. }, _) => InfoSheet::default()
                .key_binding("<y>", "Confirm delete")
                .key_binding("<n/esc>", "Cancel"),
            (Mode::List, Focus::ResourceList) => self
                .table
                .info_sheet()
                .merge(self.pagination_sheet())
                .merge(self.crud_sheet()),
            (Mode::List, Focus::Resource) => self.resource.info_sheet().merge(self.crud_sheet()),
        }
    }

    fn pagination_sheet(&self) -> InfoSheet {
        let sheet = InfoSheet::default().key_binding("<r>", "Refresh page");
        let sheet = match self.list_mode {
            ListMode::Query | ListMode::Paged => sheet
                .key_binding("<n>", "Next Page")
                .key_binding("<p>", "Prev page"),
            ListMode::Plain => sheet,
        };
        match self.list_mode {
            ListMode::Query => sheet.key_binding("<f>", "Filters"),
            _ => sheet,
        }
    }

    fn crud_sheet(&self) -> InfoSheet {
        let mut sheet = InfoSheet::default();
        if self.form_factory.is_some() {
            sheet = sheet.key_binding("<a>", "Add").key_binding("<e>", "Edit");
        }
        if self.on_delete.is_some() {
            sheet = sheet.key_binding("<d>", "Delete");
        }
        for action in &self.actions {
            sheet = sheet.key_binding(format!("<{}>", action.key), action.label);
        }
        sheet
    }

    fn begin_loading(&mut self, message: String) -> u64 {
        self.next_request_id = self.next_request_id.wrapping_add(1);
        self.loading = Some(LoadingState {
            request_id: self.next_request_id,
            message,
            spinner_frame: 0,
        });
        self.next_request_id
    }

    fn finish_loading(&mut self, request_id: u64) -> bool {
        if self
            .loading
            .as_ref()
            .is_some_and(|loading| loading.request_id == request_id)
        {
            self.loading = None;
            true
        } else {
            false
        }
    }

    fn fetch(&mut self) -> anyhow::Result<ComponentReturn<ResourcesMsg<T, R>>> {
        let Some(connector) = self.connector.clone() else {
            return Ok(ComponentReturn::empty());
        };
        let Some(on_fetch) = self.on_fetch.clone() else {
            return Ok(ComponentReturn::empty());
        };
        let query = self.query.clone();
        let request_id = self.begin_loading(format!("Loading {}...", R::title()));

        Ok(ComponentReturn::cmd(
            async move {
                match on_fetch(&connector, query).await {
                    Ok(resources) => Ok(vec![ResourcesMsg::ResourcesFetched {
                        request_id,
                        resources,
                    }
                    .into()]),
                    Err(err) => Ok(vec![ResourcesMsg::ResourcesFetchFailed {
                        request_id,
                        error: err.to_string(),
                    }
                    .into()]),
                }
            }
            .boxed(),
        ))
    }

    fn single_fetch(&mut self, selected: T) -> anyhow::Result<ComponentReturn<ResourcesMsg<T, R>>> {
        let Some(connector) = self.connector.clone() else {
            return Ok(ComponentReturn::empty());
        };
        let Some(on_single_fetch) = self.on_single_fetch.clone() else {
            return Ok(ComponentReturn::empty());
        };
        let request_id = self.begin_loading(format!("Loading {}...", R::title()));
        Ok(ComponentReturn::cmd(
            async move {
                match on_single_fetch(&connector, selected).await {
                    Ok(resource) => Ok(vec![ResourcesMsg::ResourceFetched {
                        request_id,
                        resource,
                    }
                    .into()]),
                    Err(err) => Ok(vec![ResourcesMsg::ResourcesFetchFailed {
                        request_id,
                        error: err.to_string(),
                    }
                    .into()]),
                }
            }
            .boxed(),
        ))
    }

    /// Runs `hook` on `target` in the background and reports the outcome.
    fn mutate(
        &self,
        hook: OnMutate<T>,
        target: T,
        reselect: Option<T>,
    ) -> ComponentReturn<ResourcesMsg<T, R>> {
        let Some(connector) = self.connector.clone() else {
            return Self::error(format!("No connector selected for {}", R::title()));
        };
        ComponentReturn::cmd(
            async move {
                Ok(vec![match hook(&connector, target).await {
                    Ok(msg) => ResourcesMsg::Saved { msg, reselect }.into(),
                    Err(err) => ResourcesMsg::OperationFailed(err.to_string()).into(),
                }])
            }
            .boxed(),
        )
    }

    fn page(&mut self, offset: u32) {
        self.query = self.query.to_builder().offset(offset).build();
    }

    /// The entry the CRUD keys act on: the highlighted row, or the one shown in the detail view.
    fn target(&self) -> Option<T> {
        match self.focus {
            Focus::ResourceList => self.table.selected_element().cloned(),
            Focus::Resource => self.selected.clone(),
        }
    }

    fn error(msg: String) -> ComponentReturn<ResourcesMsg<T, R>> {
        ComponentReturn::action(Action::Notification(Notification::error(msg)))
    }

    fn no_target() -> ComponentReturn<ResourcesMsg<T, R>> {
        Self::error(format!("No {} selected", R::title()))
    }

    fn with_sheet(
        mut ret: ComponentReturn<ResourcesMsg<T, R>>,
    ) -> ComponentReturn<ResourcesMsg<T, R>> {
        ret.actions.push(Action::ChangeSheet);
        ret
    }

    fn show_form(&mut self, edit: Option<T>) -> ComponentReturn<ResourcesMsg<T, R>> {
        let Some(factory) = self.form_factory.as_ref() else {
            return ComponentReturn::empty();
        };
        let title = match &edit {
            Some(_) => format!(" Edit {} ", R::title()),
            None => format!(" Add {} ", R::title()),
        };
        self.mode = Mode::Form {
            form: Box::new(factory(edit.as_ref())),
            title,
            editing: edit.is_some(),
        };
        ComponentReturn::action(Action::ChangeSheet)
    }

    fn submitted(&mut self, entry: T) -> ComponentReturn<ResourcesMsg<T, R>> {
        let editing = matches!(self.mode, Mode::Form { editing: true, .. });
        self.mode = Mode::List;
        let hook = if editing {
            self.on_update.clone()
        } else {
            self.on_create.clone()
        };
        let ret = match hook {
            Some(hook) => self.mutate(hook, entry.clone(), editing.then_some(entry)),
            None => ComponentReturn::empty(),
        };
        Self::with_sheet(ret)
    }

    fn show_action(&mut self, key: char) -> ComponentReturn<ResourcesMsg<T, R>> {
        let Some(idx) = self.actions.iter().position(|a| a.key == key) else {
            return ComponentReturn::empty();
        };
        let Some(target) = self.target() else {
            return Self::no_target();
        };
        let action = &self.actions[idx];
        match &action.kind {
            ActionKind::Immediate(run) => self.mutate(run.clone(), target.clone(), Some(target)),
            ActionKind::WithForm { form, .. } => {
                let form = form(&target, self.resource.resource());
                self.mode = Mode::ActionForm {
                    form: Box::new(form),
                    title: format!(" {} ", action.label),
                    action: idx,
                    target,
                };
                ComponentReturn::action(Action::ChangeSheet)
            }
        }
    }

    fn run_action(
        &mut self,
        idx: usize,
        target: T,
        input: ActionInput,
    ) -> ComponentReturn<ResourcesMsg<T, R>> {
        self.mode = Mode::List;
        let ret = match self.actions.get(idx).map(|a| &a.kind) {
            Some(ActionKind::WithForm { run, .. }) => {
                let run = run.clone();
                let connector = self.connector.clone();
                match connector {
                    Some(connector) => {
                        let reselect = target.clone();
                        ComponentReturn::cmd(
                            async move {
                                Ok(vec![match run(&connector, target, input).await {
                                    Ok(msg) => ResourcesMsg::Saved {
                                        msg,
                                        reselect: Some(reselect),
                                    }
                                    .into(),
                                    Err(err) => {
                                        ResourcesMsg::OperationFailed(err.to_string()).into()
                                    }
                                }])
                            }
                            .boxed(),
                        )
                    }
                    None => Self::error(format!("No connector selected for {}", R::title())),
                }
            }
            _ => ComponentReturn::empty(),
        };
        Self::with_sheet(ret)
    }

    fn saved(&mut self, msg: String, reselect: Option<T>) -> ComponentReturn<ResourcesMsg<T, R>> {
        let mut msgs = vec![ResourcesMsg::RefreshPage.into()];
        if self.focus == Focus::Resource {
            match reselect {
                Some(entry) => msgs.push(ResourcesMsg::ResourceSelected(entry).into()),
                None => self.focus = Focus::ResourceList,
            }
        }
        ComponentReturn {
            msgs,
            cmds: vec![],
            actions: vec![
                Action::Notification(Notification::info(msg)),
                Action::ChangeSheet,
            ],
        }
    }

    fn view_table(&mut self, f: &mut Frame, area: Rect) {
        let styled_text = Span::styled(
            format!(" {} ", R::title()),
            Style::default().fg(Color::Cyan),
        );
        let block = Block::default()
            .title_top(Line::from(styled_text).centered())
            .borders(Borders::ALL);

        let new_area = block.inner(area);
        let footer = if self.list_mode == ListMode::Plain {
            0
        } else {
            2
        };
        let constraints = vec![Constraint::Min(1), Constraint::Length(footer)];
        let layout = Layout::vertical(constraints).split(new_area);
        self.table.view(f, layout[0]);
        if footer > 0 {
            self.render_footer(f, layout[1]);
        }

        f.render_widget(block, area)
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let text = match self.list_mode {
            ListMode::Query => {
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
                format!(
                    "Offset: {} | Limit: {} | Sort: {} | Filter: [{}]",
                    self.query.offset(),
                    self.query.limit(),
                    sort,
                    filter.join(" , ")
                )
            }
            _ => format!(
                "Offset: {} | Limit: {}",
                self.query.offset(),
                self.query.limit()
            ),
        };
        let info_footer = Paragraph::new(Line::from(text))
            .centered()
            .block(Block::default().borders(Borders::TOP));

        frame.render_widget(info_footer, area);
    }

    fn view_popup(&mut self, f: &mut Frame) {
        match &mut self.mode {
            Mode::List => {}
            Mode::Form { form, title, .. } => {
                let inner = popup::framed(f, popup::centered(f.area(), 80, 80), title);
                form.view(f, inner);
            }
            Mode::ActionForm { form, title, .. } => {
                let inner = popup::framed(f, popup::centered(f.area(), 80, 80), title);
                form.view(f, inner);
            }
            Mode::ConfirmDelete { label, .. } => {
                let area = popup::centered_fixed(f.area(), 60, 5);
                let content = popup::framed(f, area, &format!(" Delete {} ", R::title()));
                f.render_widget(
                    Paragraph::new(format!("Delete '{}'? (y/n)", label)).centered(),
                    content,
                );
            }
        }
    }

    fn view_loading(&self, f: &mut Frame, area: Rect) {
        let Some(loading) = self.loading.as_ref() else {
            return;
        };
        let popup_area = popup::centered_fixed(area, 32, 5);
        let block = Block::default()
            .title_top(Line::from(" Loading ").centered())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let content = block.inner(popup_area);
        f.render_widget(Clear, popup_area);
        f.render_widget(block, popup_area);
        f.render_widget(
            Paragraph::new(format!("{} {}", loading.spinner(), loading.message)).centered(),
            content,
        );
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
            show_filters: false,
            on_fetch: None,
            query: Query::default(),
            on_single_fetch: None,
            loading: None,
            next_request_id: 0,
            filter: Filter::new(Query::default())
                .on_confirm(|query| Box::new(ResourcesMsg::ChangeQuery(query))),
            selected: None,
            mode: Mode::List,
            list_mode: ListMode::default(),
            form_factory: None,
            on_create: None,
            on_update: None,
            on_delete: None,
            actions: vec![],
        }
    }
}

#[async_trait::async_trait]
impl<
        T: TableEntry + Clone + Debug + Send + Sync + 'static,
        R: DrawableResource + Send + Sync + 'static,
    > Component for ResourcesComponent<T, R>
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
        };

        if self.show_filters {
            self.filter.view(f, rect);
        }
        self.view_popup(f);
        self.view_loading(f, rect);
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            ResourcesMsg::ResourceFetched {
                request_id,
                resource,
            } if self.finish_loading(request_id) => {
                self.resource.update_resource(Some(resource));
                self.focus = Focus::Resource;
                Ok(ComponentReturn::action(Action::ChangeSheet))
            }
            ResourcesMsg::ResourceSelected(selected) => {
                self.selected = Some(selected.clone());
                self.single_fetch(selected)
            }
            ResourcesMsg::TableMsg(table) => {
                Self::forward_update(&mut self.table, table.into(), ResourcesMsg::TableMsg).await
            }
            ResourcesMsg::FilterMsg(filter) => {
                Self::forward_update(&mut self.filter, filter.into(), |msg| match msg {
                    FilterMsg::Local(filter) => ResourcesMsg::FilterMsg(FilterMsg::Local(filter)),
                    FilterMsg::Outer(outer) => *outer,
                })
                .await
            }
            ResourcesMsg::ResourcesFetched {
                request_id,
                resources,
            } if self.finish_loading(request_id) => {
                self.table.update_elements(resources);
                Ok(ComponentReturn::empty())
            }
            ResourcesMsg::ShowFilters => {
                self.show_filters = true;
                self.filter.set_query(self.query.clone())?;
                Ok(ComponentReturn::empty())
            }
            ResourcesMsg::HideFilters => {
                self.show_filters = false;
                Ok(ComponentReturn::empty())
            }
            ResourcesMsg::Back => {
                self.focus = Focus::ResourceList;
                Ok(ComponentReturn::action(Action::ChangeSheet))
            }
            ResourcesMsg::NextPage => {
                if self.table.elements().len() as u32 == self.query.limit() {
                    self.page(self.query.offset() + self.query.limit());
                    self.fetch()
                } else {
                    Ok(ComponentReturn::empty())
                }
            }
            ResourcesMsg::PrevPage => {
                if self.query.offset() > 0 {
                    self.page(self.query.offset().saturating_sub(self.query.limit()));
                    self.fetch()
                } else {
                    Ok(ComponentReturn::empty())
                }
            }
            ResourcesMsg::RefreshPage => self.fetch(),
            ResourcesMsg::ChangeQuery(query) => {
                self.show_filters = false;
                self.query = query;
                self.fetch()
            }
            ResourcesMsg::ResourceMsg(msg) => {
                Self::forward_update(&mut self.resource, msg.into(), ResourcesMsg::ResourceMsg)
                    .await
            }
            ResourcesMsg::ResourcesFetchFailed { request_id, error }
                if self.finish_loading(request_id) =>
            {
                Ok(ComponentReturn::action(Action::Notification(
                    Notification::error(error),
                )))
            }
            ResourcesMsg::ResourceFetched { .. }
            | ResourcesMsg::ResourcesFetched { .. }
            | ResourcesMsg::ResourcesFetchFailed { .. } => Ok(ComponentReturn::empty()),
            ResourcesMsg::ShowAdd => Ok(self.show_form(None)),
            ResourcesMsg::ShowEdit => Ok(match self.target() {
                Some(target) => self.show_form(Some(target)),
                None => Self::no_target(),
            }),
            ResourcesMsg::ShowDelete => Ok(match (self.target(), self.on_delete.as_ref()) {
                (Some(target), Some((label, _))) => {
                    let label = label(&target);
                    self.mode = Mode::ConfirmDelete { target, label };
                    ComponentReturn::action(Action::ChangeSheet)
                }
                (None, Some(_)) => Self::no_target(),
                (_, None) => ComponentReturn::empty(),
            }),
            ResourcesMsg::ConfirmDelete => match std::mem::take(&mut self.mode) {
                Mode::ConfirmDelete { target, .. } => Ok(match self.on_delete.as_ref() {
                    Some((_, hook)) => Self::with_sheet(self.mutate(hook.clone(), target, None)),
                    None => ComponentReturn::empty(),
                }),
                other => {
                    self.mode = other;
                    Ok(ComponentReturn::empty())
                }
            },
            ResourcesMsg::Cancel => {
                self.mode = Mode::List;
                Ok(ComponentReturn::action(Action::ChangeSheet))
            }
            ResourcesMsg::FormMsg(msg) => match &mut self.mode {
                Mode::Form { form, .. } => {
                    Self::forward_update(form.as_mut(), msg.into(), |msg| match msg {
                        FormMsg::Outer(entry) => ResourcesMsg::Submitted(entry),
                        other => ResourcesMsg::FormMsg(other),
                    })
                    .await
                }
                _ => Ok(ComponentReturn::empty()),
            },
            ResourcesMsg::Submitted(entry) => Ok(self.submitted(entry)),
            ResourcesMsg::ShowAction(key) => Ok(self.show_action(key)),
            ResourcesMsg::ActionFormMsg(msg) => match &mut self.mode {
                Mode::ActionForm {
                    form,
                    action,
                    target,
                    ..
                } => {
                    let (action, target) = (*action, target.clone());
                    Self::forward_update(form.as_mut(), msg.into(), move |msg| match msg {
                        FormMsg::Outer(input) => ResourcesMsg::RunAction {
                            action,
                            target: target.clone(),
                            input,
                        },
                        other => ResourcesMsg::ActionFormMsg(other),
                    })
                    .await
                }
                _ => Ok(ComponentReturn::empty()),
            },
            ResourcesMsg::RunAction {
                action,
                target,
                input,
            } => Ok(self.run_action(action, target, input)),
            ResourcesMsg::Saved { msg, reselect } => Ok(self.saved(msg, reselect)),
            ResourcesMsg::OperationFailed(error) => Ok(Self::with_sheet(Self::error(error))),
            ResourcesMsg::Noop => Ok(ComponentReturn::empty()),
        }
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        if matches!(evt, ComponentEvent::Tick) {
            if let Some(loading) = self.loading.as_mut() {
                loading.advance();
            }
            return Ok(vec![]);
        }
        if self.loading.is_some() {
            return Ok(vec![]);
        }
        let key: Option<KeyEvent> = match &evt {
            ComponentEvent::Event(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                Some(*key)
            }
            _ => None,
        };
        let code = key.map(|k| k.code);
        let plain = key.filter(|k| k.modifiers.is_empty()).map(|k| k.code);

        match &mut self.mode {
            Mode::Form { form, .. } => {
                if code == Some(KeyCode::Esc) {
                    return Ok(vec![ResourcesMsg::Cancel.into()]);
                }
                let msgs = Self::forward_event(form.as_mut(), evt, ResourcesMsg::FormMsg)?;
                return Ok(if msgs.is_empty() && key.is_some() {
                    vec![ResourcesMsg::Noop.into()]
                } else {
                    msgs
                });
            }
            Mode::ActionForm { form, .. } => {
                if code == Some(KeyCode::Esc) {
                    return Ok(vec![ResourcesMsg::Cancel.into()]);
                }
                let msgs = Self::forward_event(form.as_mut(), evt, ResourcesMsg::ActionFormMsg)?;
                return Ok(if msgs.is_empty() && key.is_some() {
                    vec![ResourcesMsg::Noop.into()]
                } else {
                    msgs
                });
            }
            Mode::ConfirmDelete { .. } => {
                return Ok(match code {
                    Some(KeyCode::Char('y') | KeyCode::Enter) => {
                        vec![ResourcesMsg::ConfirmDelete.into()]
                    }
                    Some(KeyCode::Char('n') | KeyCode::Esc) => vec![ResourcesMsg::Cancel.into()],
                    Some(_) => vec![ResourcesMsg::Noop.into()],
                    None => vec![],
                });
            }
            Mode::List => {}
        }

        if !self.show_filters {
            match plain {
                Some(KeyCode::Char('a')) if self.form_factory.is_some() => {
                    return Ok(vec![ResourcesMsg::ShowAdd.into()]);
                }
                Some(KeyCode::Char('e')) if self.form_factory.is_some() => {
                    return Ok(vec![ResourcesMsg::ShowEdit.into()]);
                }
                Some(KeyCode::Char('d')) if self.on_delete.is_some() => {
                    return Ok(vec![ResourcesMsg::ShowDelete.into()]);
                }
                Some(KeyCode::Char(c)) if self.actions.iter().any(|a| a.key == c) => {
                    return Ok(vec![ResourcesMsg::ShowAction(c).into()]);
                }
                _ => {}
            }
        }

        let paged = matches!(self.list_mode, ListMode::Query | ListMode::Paged);
        let filterable = self.list_mode == ListMode::Query;
        match self.focus {
            Focus::ResourceList => match (plain, self.show_filters) {
                (Some(KeyCode::Char('n')), false) if paged => {
                    Ok(vec![ResourcesMsg::NextPage.into()])
                }
                (Some(KeyCode::Char('p')), false) if paged => {
                    Ok(vec![ResourcesMsg::PrevPage.into()])
                }
                (Some(KeyCode::Char('r')), false) => Ok(vec![ResourcesMsg::RefreshPage.into()]),
                (Some(KeyCode::Char('f')), false) if filterable => {
                    Ok(vec![ResourcesMsg::ShowFilters.into()])
                }
                (Some(KeyCode::Esc), true) => Ok(vec![ResourcesMsg::HideFilters.into()]),
                (_, true) => Self::forward_event(&mut self.filter, evt, |msg| match msg {
                    FilterMsg::Local(filter) => ResourcesMsg::FilterMsg(FilterMsg::Local(filter)),
                    FilterMsg::Outer(outer) => *outer,
                }),
                (_, false) => Self::forward_event(&mut self.table, evt, |msg| match msg {
                    TableMsg::Local(table) => ResourcesMsg::TableMsg(TableMsg::Local(table)),
                    TableMsg::Outer(outer) => *outer,
                }),
            },
            Focus::Resource => match code {
                Some(KeyCode::Esc) => Ok(vec![ResourcesMsg::Back.into()]),
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

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use crossterm::event::{KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, widgets::Row, Terminal};

    use super::*;
    use crate::{
        config::ConnectorConfig,
        widgets::form::{text::TextField, values::flatten},
    };

    #[derive(Debug, Clone)]
    struct Entry(String);

    impl TableEntry for Entry {
        fn row(&self) -> Row<'_> {
            Row::new(vec![self.0.clone()])
        }
        fn headers() -> Row<'static> {
            Row::new(vec!["ID"])
        }
    }

    impl DrawableResource for Entry {
        fn id(&self) -> &str {
            &self.0
        }
        fn title() -> &'static str {
            "Things"
        }
        fn fields(&self) -> Vec<Field> {
            vec![Field::string("id", &self.0)]
        }
    }

    type Log = Arc<Mutex<Vec<String>>>;
    type Comp = ResourcesComponent<Entry, Entry>;

    fn connector() -> Connector {
        Connector::from_config(
            ConnectorConfig::builder()
                .name("c")
                .address("http://localhost:1/management")
                .build()
                .unwrap(),
        )
        .unwrap()
    }

    fn entry_form(edit: Option<&Entry>) -> Form<Entry> {
        let mut field = TextField::plain("id", "Id", edit.map(|e| e.0.as_str()).unwrap_or(""));
        field.set_selected(true);
        Form::default()
            .field(field)
            .on_confirm(|fields| Ok(Entry(flatten(fields)["id"].clone())))
    }

    fn logging(
        log: &Log,
        op: &'static str,
    ) -> impl Fn(Connector, Entry) -> futures::future::Ready<anyhow::Result<String>> {
        let log = log.clone();
        move |_, e| {
            log.lock().unwrap().push(format!("{} {}", op, e.0));
            futures::future::ready(Ok(format!("{} {}", op, e.0)))
        }
    }

    fn component(log: &Log, elements: Vec<&str>) -> Comp {
        let elements: Vec<Entry> = elements.into_iter().map(|e| Entry(e.to_string())).collect();
        let action_log = log.clone();
        Comp::default()
            .on_fetch(move |_, _| {
                let elements = elements.clone();
                async move { Ok(elements) }
            })
            .on_single_fetch(|_, e| async move { Ok(e) })
            .editable(entry_form)
            .on_create(logging(log, "create"))
            .on_update(logging(log, "update"))
            .on_delete(|e| e.0.clone(), logging(log, "delete"))
            .action(ResourceAction::immediate('x', "Run x", logging(log, "x")))
            .action(ResourceAction::with_form(
                't',
                "Run t",
                |_, _| {
                    let mut field = TextField::plain("arg", "Arg", "");
                    field.set_selected(true);
                    Form::default()
                        .field(field)
                        .on_confirm(|fields| Ok(flatten(fields)))
                },
                move |_, e: Entry, input: ActionInput| {
                    action_log
                        .lock()
                        .unwrap()
                        .push(format!("t {} arg={}", e.0, input["arg"]));
                    futures::future::ready(Ok("t done".to_string()))
                },
            ))
    }

    /// Applies every message and command of `ret`, returning the collected actions.
    async fn drain(c: &mut Comp, ret: ComponentReturn<ResourcesMsg<Entry, Entry>>) -> Vec<Action> {
        let mut actions = ret.actions;
        let mut queue: VecDeque<_> = ret.msgs.into();
        let mut cmds = ret.cmds;
        loop {
            if let Some(m) = queue.pop_front() {
                let r = c.update(m).await.unwrap();
                actions.extend(r.actions);
                queue.extend(r.msgs);
                cmds.extend(r.cmds);
            } else if let Some(cmd) = cmds.pop() {
                queue.extend(cmd.await.unwrap());
            } else {
                return actions;
            }
        }
    }

    async fn start(log: &Log, elements: Vec<&str>) -> Comp {
        let mut c = component(log, elements);
        let ret = c.init(connector()).await.unwrap();
        drain(&mut c, ret).await;
        c
    }

    fn render(c: &mut Comp) -> String {
        let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
        terminal.draw(|f| c.view(f, f.area())).unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..buffer.area.height)
            .flat_map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    async fn press(c: &mut Comp, key: KeyEvent) -> Vec<Action> {
        let mut actions = vec![];
        for m in c.handle_event(key.into()).unwrap() {
            let ret = c.update(m).await.unwrap();
            actions.extend(drain(c, ret).await);
        }
        actions
    }

    async fn type_and_confirm(c: &mut Comp, text: &str) -> Vec<Action> {
        for ch in text.chars() {
            press(c, key(KeyCode::Char(ch))).await;
        }
        press(c, key(KeyCode::Enter)).await; // next -> confirm button
        press(c, key(KeyCode::Enter)).await // submit
    }

    fn notifications(actions: &[Action]) -> Vec<String> {
        actions
            .iter()
            .filter_map(|a| match a {
                Action::Notification(n) => Some(n.msg().to_string()),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn add_opens_a_form_and_calls_on_create() {
        let log = Log::default();
        let mut c = start(&log, vec!["a", "b"]).await;
        assert_eq!(c.table.elements().len(), 2);

        let actions = press(&mut c, key(KeyCode::Char('a'))).await;
        assert!(matches!(c.mode, Mode::Form { editing: false, .. }));
        assert!(matches!(actions.as_slice(), [Action::ChangeSheet]));

        let actions = type_and_confirm(&mut c, "n").await;
        assert!(matches!(c.mode, Mode::List));
        assert_eq!(*log.lock().unwrap(), vec!["create n"]);
        assert_eq!(notifications(&actions), vec!["create n"]);
        assert!(actions.iter().any(|a| matches!(a, Action::ChangeSheet)));
    }

    #[tokio::test]
    async fn edit_prefills_the_selected_entry_and_calls_on_update() {
        let log = Log::default();
        let mut c = start(&log, vec!["a", "b"]).await;
        press(&mut c, key(KeyCode::Char('e'))).await;
        match &c.mode {
            Mode::Form { form, editing, .. } => {
                assert!(editing);
                assert_eq!(form.field_value("id").as_deref(), Some("a"));
            }
            other => panic!("unexpected {other:?}"),
        }
        type_and_confirm(&mut c, "").await;
        assert_eq!(*log.lock().unwrap(), vec!["update a"]);
    }

    #[tokio::test]
    async fn delete_needs_confirmation() {
        let log = Log::default();
        let mut c = start(&log, vec!["a"]).await;
        press(&mut c, key(KeyCode::Char('d'))).await;
        assert!(matches!(&c.mode, Mode::ConfirmDelete { label, .. } if label == "a"));

        let msgs = c.handle_event(key(KeyCode::Char('z')).into()).unwrap();
        assert!(matches!(
            msgs.as_slice(),
            [ComponentMsg(ResourcesMsg::Noop)]
        ));
        press(&mut c, key(KeyCode::Char('n'))).await;
        assert!(matches!(c.mode, Mode::List));
        assert!(log.lock().unwrap().is_empty());

        press(&mut c, key(KeyCode::Char('d'))).await;
        let actions = press(&mut c, key(KeyCode::Char('y'))).await;
        assert!(matches!(c.mode, Mode::List));
        assert_eq!(*log.lock().unwrap(), vec!["delete a"]);
        assert_eq!(notifications(&actions), vec!["delete a"]);
    }

    #[tokio::test]
    async fn edit_and_delete_without_a_selection_notify() {
        let log = Log::default();
        let mut c = start(&log, vec![]).await;
        for k in ['e', 'd', 'x'] {
            let actions = press(&mut c, key(KeyCode::Char(k))).await;
            assert_eq!(notifications(&actions), vec!["No Things selected"], "{k}");
            assert!(matches!(c.mode, Mode::List));
        }
    }

    #[tokio::test]
    async fn loading_overlay_animates_and_ignores_resource_input() {
        let log = Log::default();
        let mut c = component(&log, vec!["a"]);
        let _request = c.init(connector()).await.unwrap();

        assert!(render(&mut c).contains("Loading Things..."));
        assert_eq!(c.loading.as_ref().unwrap().spinner(), "|");
        c.handle_event(ComponentEvent::Tick).unwrap();
        assert_eq!(c.loading.as_ref().unwrap().spinner(), "/");
        assert!(c
            .handle_event(key(KeyCode::Char('r')).into())
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn stale_fetch_results_do_not_replace_active_request() {
        let log = Log::default();
        let mut c = component(&log, vec![]);
        let _first = c.init(connector()).await.unwrap();
        let first_request_id = c.loading.as_ref().unwrap().request_id;
        let _second = c.fetch().unwrap();
        let second_request_id = c.loading.as_ref().unwrap().request_id;

        c.update(
            ResourcesMsg::ResourcesFetched {
                request_id: first_request_id,
                resources: vec![Entry("stale".to_string())],
            }
            .into(),
        )
        .await
        .unwrap();

        assert_eq!(c.loading.as_ref().unwrap().request_id, second_request_id);
        assert!(c.table.elements().is_empty());

        c.update(
            ResourcesMsg::ResourcesFetched {
                request_id: second_request_id,
                resources: vec![Entry("current".to_string())],
            }
            .into(),
        )
        .await
        .unwrap();

        assert!(c.loading.is_none());
        assert_eq!(c.table.elements()[0].0, "current");
    }

    #[tokio::test]
    async fn crud_keys_are_inert_without_hooks_and_with_modifiers() {
        let mut c = Comp::default();
        for k in ['a', 'e', 'd', 'x', 't'] {
            assert!(c
                .handle_event(key(KeyCode::Char(k)).into())
                .unwrap()
                .is_empty());
        }
        let log = Log::default();
        let mut c = start(&log, vec!["a"]).await;
        let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert!(c.handle_event(ctrl_a.into()).unwrap().is_empty());
        assert!(matches!(c.mode, Mode::List));
    }

    #[tokio::test]
    async fn open_popups_swallow_global_keys_and_esc_cancels() {
        let log = Log::default();
        let mut c = start(&log, vec!["a"]).await;
        press(&mut c, key(KeyCode::Char('a'))).await;
        for k in [
            key(KeyCode::Tab),
            key(KeyCode::Char(':')),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
        ] {
            assert!(!c.handle_event(k.into()).unwrap().is_empty(), "{k:?}");
        }
        let actions = press(&mut c, key(KeyCode::Esc)).await;
        assert!(matches!(c.mode, Mode::List));
        assert!(matches!(actions.as_slice(), [Action::ChangeSheet]));
        assert!(log.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn plain_lists_ignore_paging_and_filters() {
        let log = Log::default();
        let mut c = component(&log, vec!["a"]).list_mode(ListMode::Plain);
        for k in ['n', 'p', 'f'] {
            assert!(c
                .handle_event(key(KeyCode::Char(k)).into())
                .unwrap()
                .is_empty());
        }
        assert!(!c
            .handle_event(key(KeyCode::Char('r')).into())
            .unwrap()
            .is_empty());

        let mut paged = component(&log, vec!["a"]).list_mode(ListMode::Paged);
        assert!(!paged
            .handle_event(key(KeyCode::Char('n')).into())
            .unwrap()
            .is_empty());
        assert!(paged
            .handle_event(key(KeyCode::Char('f')).into())
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn actions_work_from_the_detail_view_and_reload_it() {
        let log = Log::default();
        let mut c = start(&log, vec!["a", "b"]).await;
        press(&mut c, key(KeyCode::Down)).await;
        press(&mut c, key(KeyCode::Enter)).await;
        assert_eq!(c.focus, Focus::Resource);
        assert_eq!(c.selected.as_ref().map(|e| e.0.as_str()), Some("b"));

        let actions = press(&mut c, key(KeyCode::Char('x'))).await;
        assert_eq!(*log.lock().unwrap(), vec!["x b"]);
        assert_eq!(notifications(&actions), vec!["x b"]);
        assert_eq!(c.focus, Focus::Resource);

        press(&mut c, key(KeyCode::Char('t'))).await;
        assert!(matches!(c.mode, Mode::ActionForm { action: 1, .. }));
        type_and_confirm(&mut c, "v").await;
        assert_eq!(log.lock().unwrap().last().unwrap(), "t b arg=v");
        assert!(matches!(c.mode, Mode::List));

        press(&mut c, key(KeyCode::Char('d'))).await;
        press(&mut c, key(KeyCode::Char('y'))).await;
        assert_eq!(log.lock().unwrap().last().unwrap(), "delete b");
        assert_eq!(c.focus, Focus::ResourceList);
    }

    #[tokio::test]
    async fn failures_are_reported_as_error_notifications() {
        let log = Log::default();
        let mut c = component(&log, vec!["a"])
            .on_delete(|e| e.0.clone(), |_, _| async { anyhow::bail!("boom") });
        let ret = c.init(connector()).await.unwrap();
        drain(&mut c, ret).await;
        press(&mut c, key(KeyCode::Char('d'))).await;
        let actions = press(&mut c, key(KeyCode::Char('y'))).await;
        assert_eq!(notifications(&actions), vec!["boom"]);
    }

    #[test]
    fn info_sheet_lists_the_enabled_keys() {
        let log = Log::default();
        let c = component(&log, vec![]);
        let keys: Vec<String> = c
            .info_sheet()
            .iter_key_bindings()
            .map(|(k, _)| k.clone())
            .collect();
        for k in ["<a>", "<e>", "<d>", "<x>", "<t>", "<n>", "<f>"] {
            assert!(keys.contains(&k.to_string()), "{k}");
        }
        let plain = Comp::default().list_mode(ListMode::Plain);
        let keys: Vec<String> = plain
            .info_sheet()
            .iter_key_bindings()
            .map(|(k, _)| k.clone())
            .collect();
        for k in ["<a>", "<d>", "<n>", "<f>"] {
            assert!(!keys.contains(&k.to_string()), "{k}");
        }
    }
}
