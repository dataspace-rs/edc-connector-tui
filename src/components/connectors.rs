use std::path::PathBuf;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Rect,
    widgets::{Paragraph, Row},
    Frame,
};

use crate::{
    config::{Config, ConnectorApiVersion, ConnectorConfig},
    secrets,
    types::{connector::Connector, info::InfoSheet, nav::Nav},
    widgets::{form::Form, popup},
};

use self::{
    form::{ConnectorForm, ConnectorFormMsg, ConnectorFormOutput},
    msg::ConnectorsMsg,
};

use super::{
    table::{msg::TableMsg, TableEntry, UiTable},
    Action, Component, ComponentEvent, ComponentMsg, ComponentReturn, Notification,
};

pub mod form;
pub mod msg;

pub type ConnectorsTable = UiTable<ConnectorEntry, Box<ConnectorsMsg>>;

#[derive(Debug, Default)]
pub struct ConnectorsComponent {
    table: ConnectorsTable,
    selected: Option<Connector>,
    config_path: Option<PathBuf>,
    mode: Mode,
}

#[derive(Default)]
enum Mode {
    #[default]
    List,
    Form(Box<ConnectorForm>),
    ConfirmDelete {
        index: usize,
        name: String,
    },
}

impl std::fmt::Debug for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::List => write!(f, "List"),
            Mode::Form(_) => write!(f, "Form"),
            Mode::ConfirmDelete { index, name } => f
                .debug_struct("ConfirmDelete")
                .field("index", index)
                .field("name", name)
                .finish(),
        }
    }
}

#[derive(Debug)]
pub struct ConnectorEntry(Connector);

impl TableEntry for ConnectorEntry {
    fn row(&self) -> Row<'_> {
        Row::new(vec![
            self.0.config().name(),
            self.0.config().address(),
            self.0.config().version().as_str(),
            self.0.config().auth().kind(),
            self.0.status().as_str(),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec!["NAME", "ADDRESS", "API VERSION", "AUTH", "STATUS"])
    }
}

#[async_trait::async_trait]
impl Component for ConnectorsComponent {
    type Msg = ConnectorsMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        self.table.view(f, rect);
        match &mut self.mode {
            Mode::List => {}
            Mode::Form(form) => form.view(f, rect),
            Mode::ConfirmDelete { name, .. } => {
                let area = popup::centered_fixed(f.area(), 60, 5);
                let content = popup::framed(f, area, " Delete connector ");
                f.render_widget(
                    Paragraph::new(format!("Delete connector '{}'? (y/n)", name)).centered(),
                    content,
                );
            }
        }
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            ConnectorsMsg::ConnectorSelected(connector) => {
                self.selected = Some(*connector);
                Ok(ComponentReturn::action(Action::NavTo(Nav::AssetsList)))
            }
            ConnectorsMsg::TableEvent(table) => {
                Self::forward_update::<_, ConnectorsTable>(
                    &mut self.table,
                    table.into(),
                    ConnectorsMsg::TableEvent,
                )
                .await
            }
            ConnectorsMsg::ShowAdd => {
                self.mode = Mode::Form(Box::new(ConnectorForm::new(self.names(), None)));
                Ok(ComponentReturn::action(Action::ChangeSheet))
            }
            ConnectorsMsg::ShowEdit => match self.table.selected_element() {
                Some(entry) => {
                    let cfg = entry.0.config().clone();
                    let others = self
                        .names()
                        .into_iter()
                        .filter(|n| n != cfg.name())
                        .collect();
                    self.mode = Mode::Form(Box::new(ConnectorForm::new(others, Some(&cfg))));
                    Ok(ComponentReturn::action(Action::ChangeSheet))
                }
                None => Ok(Self::error("No connector selected")),
            },
            ConnectorsMsg::ShowDelete => match self.table.selected_index() {
                Some(index) => {
                    let name = self.table.elements()[index].0.config().name().to_string();
                    self.mode = Mode::ConfirmDelete { index, name };
                    Ok(ComponentReturn::action(Action::ChangeSheet))
                }
                None => Ok(Self::error("No connector selected")),
            },
            ConnectorsMsg::ConfirmDelete => match std::mem::take(&mut self.mode) {
                Mode::ConfirmDelete { index, name } => self.delete(index, &name),
                other => {
                    self.mode = other;
                    Ok(ComponentReturn::empty())
                }
            },
            ConnectorsMsg::Cancel => {
                self.mode = Mode::List;
                Ok(ComponentReturn::action(Action::ChangeSheet))
            }
            ConnectorsMsg::FormMsg(msg) => match &mut self.mode {
                Mode::Form(form) => {
                    Self::forward_update(form.as_mut(), msg.into(), |msg| match msg {
                        ConnectorFormMsg::Submitted(out) => ConnectorsMsg::Submitted(out),
                        other => ConnectorsMsg::FormMsg(other),
                    })
                    .await
                }
                _ => Ok(ComponentReturn::empty()),
            },
            ConnectorsMsg::Submitted(out) => self.save(*out),
            ConnectorsMsg::Noop => Ok(ComponentReturn::empty()),
        }
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        let key = match &evt {
            ComponentEvent::Event(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                Some(*key)
            }
            _ => None,
        };
        let plain = key.filter(|k| k.modifiers.is_empty()).map(|k| k.code);
        let key = key.map(|k| k.code);
        match (&mut self.mode, key) {
            (Mode::List, _) if plain == Some(KeyCode::Char('a')) => {
                Ok(vec![ConnectorsMsg::ShowAdd.into()])
            }
            (Mode::List, _) if plain == Some(KeyCode::Char('e')) => {
                Ok(vec![ConnectorsMsg::ShowEdit.into()])
            }
            (Mode::List, _) if plain == Some(KeyCode::Char('d')) => {
                Ok(vec![ConnectorsMsg::ShowDelete.into()])
            }
            (Mode::List, _) => Self::forward_event(&mut self.table, evt, |msg| match msg {
                TableMsg::Local(table) => ConnectorsMsg::TableEvent(TableMsg::Local(table)),
                TableMsg::Outer(outer) => *outer,
            }),
            (Mode::Form(_), Some(KeyCode::Esc)) => Ok(vec![ConnectorsMsg::Cancel.into()]),
            (Mode::Form(form), _) => {
                let msgs = Self::forward_event(form.as_mut(), evt, ConnectorsMsg::FormMsg)?;
                if msgs.is_empty() && key.is_some() {
                    Ok(vec![ConnectorsMsg::Noop.into()])
                } else {
                    Ok(msgs)
                }
            }
            (Mode::ConfirmDelete { .. }, Some(KeyCode::Char('y') | KeyCode::Enter)) => {
                Ok(vec![ConnectorsMsg::ConfirmDelete.into()])
            }
            (Mode::ConfirmDelete { .. }, Some(KeyCode::Char('n') | KeyCode::Esc)) => {
                Ok(vec![ConnectorsMsg::Cancel.into()])
            }
            (Mode::ConfirmDelete { .. }, Some(_)) => Ok(vec![ConnectorsMsg::Noop.into()]),
            (Mode::ConfirmDelete { .. }, None) => Ok(vec![]),
        }
    }
}

impl ConnectorsComponent {
    pub fn new(connectors: Vec<Connector>, config_path: Option<PathBuf>) -> Self {
        let selected = connectors.first().cloned();
        Self {
            table: ConnectorsTable::with_elements(
                "Connectors".to_string(),
                connectors.into_iter().map(ConnectorEntry).collect(),
                true,
            )
            .on_select(|connector| {
                Box::new(ConnectorsMsg::ConnectorSelected(Box::new(
                    connector.0.clone(),
                )))
            }),
            selected,
            config_path,
            mode: Mode::List,
        }
    }

    pub fn selected(&self) -> Option<&Connector> {
        self.selected.as_ref()
    }

    pub fn selected_version(&self) -> Option<ConnectorApiVersion> {
        self.selected.as_ref().map(|c| *c.config().version())
    }

    pub fn info_sheet(&self) -> InfoSheet {
        if let Some(c) = self.selected.as_ref() {
            InfoSheet::default()
                .info("Connector Name", c.config().name())
                .info("Connector Address", c.config().address())
        } else {
            InfoSheet::default()
                .info("Connector Name", "n/a")
                .info("Connector Address", "n/a")
        }
    }

    /// Key bindings for the current mode, shown in the header when the connectors menu is active.
    pub fn key_bindings(&self) -> InfoSheet {
        match self.mode {
            Mode::List => InfoSheet::default()
                .key_binding("<enter>", "Select connector")
                .key_binding("<a>", "Add connector")
                .key_binding("<e>", "Edit connector")
                .key_binding("<d>", "Delete connector"),
            Mode::Form(_) => Form::<ConnectorFormOutput>::key_bindings(),
            Mode::ConfirmDelete { .. } => InfoSheet::default()
                .key_binding("<y>", "Confirm delete")
                .key_binding("<n/esc>", "Cancel"),
        }
    }

    fn connectors(&self) -> Vec<Connector> {
        self.table.elements().iter().map(|e| e.0.clone()).collect()
    }

    fn configs(&self) -> Vec<ConnectorConfig> {
        self.table
            .elements()
            .iter()
            .map(|e| e.0.config().clone())
            .collect()
    }

    fn names(&self) -> Vec<String> {
        self.table
            .elements()
            .iter()
            .map(|e| e.0.config().name().to_string())
            .collect()
    }

    fn selected_name(&self) -> Option<&str> {
        self.selected.as_ref().map(|c| c.config().name())
    }

    fn set_connectors(&mut self, connectors: Vec<Connector>) {
        self.table
            .update_elements(connectors.into_iter().map(ConnectorEntry).collect());
    }

    fn error(msg: &str) -> ComponentReturn<ConnectorsMsg> {
        ComponentReturn::action(Action::Notification(Notification::error(msg.to_string())))
    }

    fn delete(
        &mut self,
        index: usize,
        name: &str,
    ) -> anyhow::Result<ComponentReturn<ConnectorsMsg>> {
        let mut connectors = self.connectors();
        if index < connectors.len() {
            connectors.remove(index);
        }
        if self.selected_name() == Some(name) {
            self.selected = connectors.first().cloned();
        }
        self.set_connectors(connectors);
        self.persist(format!("Connector '{}' deleted", name))
    }

    fn save(&mut self, out: ConnectorFormOutput) -> anyhow::Result<ComponentReturn<ConnectorsMsg>> {
        for (alias, secret) in &out.secrets {
            secrets::store(alias, secret).map_err(|e| {
                anyhow::anyhow!("Failed to store secret for alias {}: {}", alias, e)
            })?;
        }
        let connector = Connector::from_config(out.config)?;
        let name = connector.config().name().to_string();

        let mut connectors = self.connectors();
        let index = match out.target.as_deref() {
            Some(target) => match connectors.iter().position(|c| c.config().name() == target) {
                Some(pos) => {
                    connectors[pos] = connector.clone();
                    if self.selected_name() == Some(target) {
                        self.selected = Some(connector.clone());
                    }
                    pos
                }
                None => {
                    connectors.push(connector.clone());
                    connectors.len() - 1
                }
            },
            None => {
                connectors.push(connector.clone());
                connectors.len() - 1
            }
        };
        if self.selected.is_none() {
            self.selected = Some(connector);
        }
        self.set_connectors(connectors);
        self.table.select(Some(index));
        self.mode = Mode::List;
        self.persist(format!("Connector '{}' saved", name))
    }

    /// Writes the current connectors to the config file (when there is one) and reports it.
    fn persist(&self, msg: String) -> anyhow::Result<ComponentReturn<ConnectorsMsg>> {
        let notification = match &self.config_path {
            Some(path) => {
                Config {
                    connectors: self.configs(),
                }
                .save(path)
                .map_err(|e| anyhow::anyhow!("Failed to save {}: {}", path.display(), e))?;
                Notification::info(msg)
            }
            None => Notification::info(format!("{} (not persisted: no config file)", msg)),
        };
        Ok(ComponentReturn {
            msgs: vec![],
            cmds: vec![],
            actions: vec![Action::Notification(notification), Action::ChangeSheet],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::form::msg::{FormLocalMsg, FormMsg};
    use crossterm::event::KeyEvent;

    fn connector(name: &str) -> Connector {
        Connector::from_config(config(name)).unwrap()
    }

    fn config(name: &str) -> ConnectorConfig {
        ConnectorConfig::builder()
            .name(name)
            .address("http://localhost:29193/management")
            .build()
            .unwrap()
    }

    fn key(code: KeyCode) -> ComponentEvent {
        ComponentEvent::Event(Event::Key(KeyEvent::from(code)))
    }

    fn msgs(c: &mut ConnectorsComponent, code: KeyCode) -> Vec<ConnectorsMsg> {
        c.handle_event(key(code))
            .unwrap()
            .into_iter()
            .map(|m| m.take())
            .collect()
    }

    async fn press(c: &mut ConnectorsComponent, code: KeyCode) -> Vec<Action> {
        let mut actions = vec![];
        for m in c.handle_event(key(code)).unwrap() {
            actions.extend(c.update(m).await.unwrap().actions);
        }
        actions
    }

    fn submitted(name: &str, target: Option<&str>) -> ComponentMsg<ConnectorsMsg> {
        ConnectorsMsg::Submitted(Box::new(ConnectorFormOutput {
            config: config(name),
            secrets: vec![],
            target: target.map(String::from),
        }))
        .into()
    }

    fn names(c: &ConnectorsComponent) -> Vec<String> {
        c.names()
    }

    #[test]
    fn ctrl_a_does_not_open_the_add_form() {
        let mut c = ConnectorsComponent::new(vec![connector("a")], None);
        let ctrl_a = ComponentEvent::Event(Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            crossterm::event::KeyModifiers::CONTROL,
        )));
        assert!(c.handle_event(ctrl_a).unwrap().is_empty());
        assert!(matches!(c.mode, Mode::List));
        assert!(matches!(
            msgs(&mut c, KeyCode::Char('a')).as_slice(),
            [ConnectorsMsg::ShowAdd]
        ));
    }

    fn notification_text(actions: &[Action]) -> String {
        actions
            .iter()
            .filter_map(|a| match a {
                Action::Notification(n) => Some(n.msg().to_string()),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn add_flow_without_config_file() {
        let mut c = ConnectorsComponent::new(vec![], None);
        assert!(matches!(
            msgs(&mut c, KeyCode::Char('a')).as_slice(),
            [ConnectorsMsg::ShowAdd]
        ));
        press(&mut c, KeyCode::Char('a')).await;
        assert!(matches!(c.mode, Mode::Form(_)));

        let ret = c.update(submitted("new", None)).await.unwrap();
        assert!(matches!(c.mode, Mode::List));
        assert_eq!(names(&c), vec!["new".to_string()]);
        assert_eq!(c.selected().unwrap().config().name(), "new");
        assert_eq!(c.table.selected_index(), Some(0));
        let text = notification_text(&ret.actions);
        assert!(
            text.contains("saved") && text.contains("not persisted"),
            "{text}"
        );
        assert!(ret.actions.iter().any(|a| matches!(a, Action::ChangeSheet)));
    }

    #[tokio::test]
    async fn add_flow_persists_to_config_file() {
        let path = std::env::temp_dir()
            .join(format!("edc-tui-connectors-{}", std::process::id()))
            .join("config.toml");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        let mut c = ConnectorsComponent::new(vec![connector("a")], Some(path.clone()));
        press(&mut c, KeyCode::Char('a')).await;
        c.update(submitted("b", None)).await.unwrap();

        let saved = Config::load(&path).unwrap();
        let saved: Vec<_> = saved.connectors.iter().map(|c| c.name()).collect();
        assert_eq!(saved, vec!["a", "b"]);
        assert_eq!(c.table.selected_index(), Some(1));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[tokio::test]
    async fn edit_renames_the_selected_connector() {
        let mut c = ConnectorsComponent::new(vec![connector("a"), connector("b")], None);
        press(&mut c, KeyCode::Char('e')).await;
        assert!(matches!(c.mode, Mode::Form(_)));
        c.update(submitted("a2", Some("a"))).await.unwrap();
        assert_eq!(names(&c), vec!["a2".to_string(), "b".to_string()]);
        assert_eq!(c.selected().unwrap().config().name(), "a2");
        assert_eq!(c.table.selected_index(), Some(0));
    }

    #[tokio::test]
    async fn delete_flow_empties_the_list_and_navigation_survives() {
        let mut c = ConnectorsComponent::new(vec![connector("a")], None);
        assert!(matches!(
            msgs(&mut c, KeyCode::Char('d')).as_slice(),
            [ConnectorsMsg::ShowDelete]
        ));
        press(&mut c, KeyCode::Char('d')).await;
        assert!(matches!(c.mode, Mode::ConfirmDelete { index: 0, .. }));
        assert!(matches!(
            msgs(&mut c, KeyCode::Char('x')).as_slice(),
            [ConnectorsMsg::Noop]
        ));

        let actions = press(&mut c, KeyCode::Char('y')).await;
        assert!(notification_text(&actions).contains("deleted"));
        assert!(matches!(c.mode, Mode::List));
        assert!(names(&c).is_empty());
        assert!(c.selected().is_none());
        press(&mut c, KeyCode::Down).await;
        press(&mut c, KeyCode::Up).await;
        assert!(matches!(
            msgs(&mut c, KeyCode::Char('e')).as_slice(),
            [ConnectorsMsg::ShowEdit]
        ));
        let actions = press(&mut c, KeyCode::Char('e')).await;
        assert!(notification_text(&actions).contains("No connector selected"));
    }

    #[tokio::test]
    async fn deleting_the_selected_connector_selects_the_first_remaining() {
        let mut c = ConnectorsComponent::new(vec![connector("a"), connector("b")], None);
        press(&mut c, KeyCode::Char('d')).await;
        press(&mut c, KeyCode::Enter).await;
        assert_eq!(names(&c), vec!["b".to_string()]);
        assert_eq!(c.selected().unwrap().config().name(), "b");
    }

    #[tokio::test]
    async fn esc_and_n_cancel_popups() {
        let mut c = ConnectorsComponent::new(vec![connector("a")], None);
        press(&mut c, KeyCode::Char('a')).await;
        assert!(matches!(
            msgs(&mut c, KeyCode::Esc).as_slice(),
            [ConnectorsMsg::Cancel]
        ));
        press(&mut c, KeyCode::Esc).await;
        assert!(matches!(c.mode, Mode::List));

        press(&mut c, KeyCode::Char('d')).await;
        press(&mut c, KeyCode::Char('n')).await;
        assert!(matches!(c.mode, Mode::List));
        assert_eq!(names(&c), vec!["a".to_string()]);
    }

    #[tokio::test]
    async fn invalid_form_keeps_the_form_open() {
        let mut c = ConnectorsComponent::new(vec![], None);
        press(&mut c, KeyCode::Char('a')).await;
        let submit = ConnectorsMsg::FormMsg(ConnectorFormMsg::Local(FormMsg::Local(
            FormLocalMsg::Submit,
        )));
        let err = c.update(submit.into()).await.unwrap_err().to_string();
        assert!(err.contains("Name is required"), "{err}");
        assert!(matches!(c.mode, Mode::Form(_)));
        assert!(names(&c).is_empty());
    }

    #[tokio::test]
    async fn keys_in_form_mode_are_consumed() {
        let mut c = ConnectorsComponent::new(vec![], None);
        press(&mut c, KeyCode::Char('a')).await;
        assert!(matches!(
            msgs(&mut c, KeyCode::Char(':')).as_slice(),
            [ConnectorsMsg::FormMsg(_)]
        ));
        assert!(matches!(
            msgs(&mut c, KeyCode::Tab).as_slice(),
            [ConnectorsMsg::FormMsg(_)]
        ));
        assert!(matches!(
            msgs(&mut c, KeyCode::F(5)).as_slice(),
            [ConnectorsMsg::Noop]
        ));
    }
}
