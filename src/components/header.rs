use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Tabs},
    Frame,
};

use crate::{
    config::ConnectorApiVersion,
    types::{
        info::InfoSheet,
        nav::{Menu, Workspace},
    },
};

use self::{help::InfoComponent, msg::HeaderMsg};

use super::{
    Action, Component, ComponentEvent, ComponentMsg, ComponentReturn, Notification,
    StatelessComponent,
};

pub mod help;
pub mod msg;

pub struct HeaderComponent {
    menu: Menu,
    /// API version of the selected connector, deciding which entries are shown.
    version: Option<ConnectorApiVersion>,
    /// Last selected entry of each workspace (indexed by `Workspace as usize`), restored
    /// when switching back to it.
    last: [Menu; 2],
    info: InfoComponent,
    sheet: InfoSheet,
}

impl Default for HeaderComponent {
    fn default() -> Self {
        Self::with_sheet(InfoSheet::default())
    }
}

impl HeaderComponent {
    pub fn with_sheet(sheet: InfoSheet) -> HeaderComponent {
        HeaderComponent {
            menu: Menu::default(),
            version: None,
            last: [
                Menu::home(Workspace::Operations),
                Menu::home(Workspace::Admin),
            ],
            info: InfoComponent::default(),
            sheet,
        }
    }

    pub fn set_selected_menu(&mut self, menu: impl Into<Menu>) {
        self.menu = menu.into();
        self.last[self.workspace() as usize] = self.menu.clone();
    }

    /// The connector API version deciding which menu entries are shown and cycled.
    pub fn set_version(&mut self, version: Option<ConnectorApiVersion>) {
        self.version = version;
    }

    pub fn workspace(&self) -> Workspace {
        self.menu.workspace()
    }

    /// The entries of the active workspace, as shown in the tab bar.
    fn visible_menus(&self) -> Vec<Menu> {
        Menu::available_for(self.version, self.workspace())
    }

    fn selected_position(&self) -> Option<usize> {
        self.visible_menus().iter().position(|m| m == &self.menu)
    }

    /// Move the selection by `step` entries within the visible menu list, wrapping around.
    fn step_menu(&mut self, step: isize) -> ComponentReturn<HeaderMsg> {
        let menus = self.visible_menus();
        let len = menus.len();
        if len == 0 {
            return ComponentReturn::empty();
        }
        let pos = self.selected_position().unwrap_or(0) as isize;
        let idx = (pos + step).rem_euclid(len as isize) as usize;
        self.set_selected_menu(menus[idx].clone());
        ComponentReturn::action(Action::NavTo(self.menu.clone().into()))
    }

    /// Navigate to the last entry of `target` (or of the other workspace when `None`),
    /// refusing when it is not available for the selected connector.
    fn switch_workspace(&self, target: Option<Workspace>) -> ComponentReturn<HeaderMsg> {
        let target = target.unwrap_or_else(|| self.workspace().other());
        if target == self.workspace() {
            return ComponentReturn::empty();
        }
        if !target.is_available_for(self.version) {
            return ComponentReturn::action(Action::Notification(Notification::error(format!(
                "{} workspace requires API version v5 (connector is {})",
                target.name(),
                self.version.map(|v| v.as_str()).unwrap_or("n/a")
            ))));
        }
        ComponentReturn::action(Action::NavTo(self.last[target as usize].clone().into()))
    }

    pub fn update_sheet(&mut self, sheet: InfoSheet) {
        self.sheet = sheet;
    }

    pub fn selected_menu(&self) -> &Menu {
        &self.menu
    }
}

#[async_trait::async_trait]
impl Component for HeaderComponent {
    type Msg = HeaderMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .split(rect);

        let workspace = self.workspace();
        let highlight = match workspace {
            Workspace::Operations => Color::Yellow,
            Workspace::Admin => Color::LightMagenta,
        };
        let menus = self.visible_menus();
        let tabs = Tabs::new(menus.iter().map(Menu::name))
            .block(Block::bordered().title(format!("Menu [{}]", workspace.name())))
            .style(Style::default().white())
            .highlight_style(Style::default().fg(highlight))
            .select(self.selected_position())
            .divider("|")
            .padding(" ", " ");

        f.render_widget(tabs, layout[0]);
        self.info.view(&self.sheet, f, layout[1]);
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            HeaderMsg::NextTab => Ok(self.step_menu(1)),
            HeaderMsg::PrevTab => Ok(self.step_menu(-1)),
            HeaderMsg::SwitchWorkspace(target) => Ok(self.switch_workspace(target)),
        }
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        if let ComponentEvent::Event(Event::Key(key)) = evt {
            return Ok(self.handle_key(key));
        }
        Ok(vec![])
    }
}

impl HeaderComponent {
    fn handle_key(&self, key: KeyEvent) -> Vec<ComponentMsg<HeaderMsg>> {
        match key.code {
            KeyCode::Tab if key.modifiers.is_empty() => {
                vec![HeaderMsg::NextTab.into()]
            }
            KeyCode::BackTab if key.modifiers == KeyModifiers::SHIFT => {
                vec![HeaderMsg::PrevTab.into()]
            }
            KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
                vec![HeaderMsg::SwitchWorkspace(None).into()]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{components::Action, config::ConnectorApiVersion, types::nav::Nav};

    fn header(version: Option<ConnectorApiVersion>, selected: Menu) -> HeaderComponent {
        let mut header = HeaderComponent::default();
        header.set_version(version);
        header.set_selected_menu(selected);
        header
    }

    async fn step(header: &mut HeaderComponent, msg: HeaderMsg) -> Vec<Action> {
        header.update(msg.into()).await.unwrap().actions
    }

    #[tokio::test]
    async fn next_tab_reaches_edrs_when_available() {
        let mut header = header(None, Menu::TransferProcesses);
        let actions = step(&mut header, HeaderMsg::NextTab).await;
        assert_eq!(header.selected_menu(), &Menu::Edrs);
        assert!(matches!(actions.as_slice(), [Action::NavTo(Nav::Edrs)]));
    }

    #[tokio::test]
    async fn next_tab_skips_edrs_for_v4() {
        let mut header = header(Some(ConnectorApiVersion::V4), Menu::TransferProcesses);
        let actions = step(&mut header, HeaderMsg::NextTab).await;
        assert_eq!(header.selected_menu(), &Menu::DataPlanes);
        assert!(matches!(
            actions.as_slice(),
            [Action::NavTo(Nav::DataPlanes)]
        ));
    }

    #[tokio::test]
    async fn prev_tab_skips_edrs_for_v4() {
        let mut header = header(Some(ConnectorApiVersion::V4), Menu::DataPlanes);
        let actions = step(&mut header, HeaderMsg::PrevTab).await;
        assert_eq!(header.selected_menu(), &Menu::TransferProcesses);
        assert!(matches!(
            actions.as_slice(),
            [Action::NavTo(Nav::TransferProcesses)]
        ));
    }

    #[tokio::test]
    async fn tabs_wrap_around_within_the_workspace() {
        let mut header = header(Some(ConnectorApiVersion::V5), Menu::DataPlanes);
        step(&mut header, HeaderMsg::NextTab).await;
        assert_eq!(header.selected_menu(), &Menu::Connectors);
        step(&mut header, HeaderMsg::PrevTab).await;
        assert_eq!(header.selected_menu(), &Menu::DataPlanes);

        header.set_selected_menu(Menu::SchemaValidators);
        step(&mut header, HeaderMsg::NextTab).await;
        assert_eq!(header.selected_menu(), &Menu::Participants);
        step(&mut header, HeaderMsg::PrevTab).await;
        assert_eq!(header.selected_menu(), &Menu::SchemaValidators);
    }

    #[tokio::test]
    async fn empty_menu_list_is_a_noop() {
        // Admin menus are not available for v3, so there is nothing to cycle.
        let mut header = header(Some(ConnectorApiVersion::V3), Menu::DcpScopes);
        let actions = step(&mut header, HeaderMsg::NextTab).await;
        assert!(actions.is_empty());
        assert_eq!(header.selected_menu(), &Menu::DcpScopes);
    }

    #[tokio::test]
    async fn toggle_switches_workspace_and_remembers_the_last_menu() {
        let mut header = header(Some(ConnectorApiVersion::V5), Menu::Policies);
        let actions = step(&mut header, HeaderMsg::SwitchWorkspace(None)).await;
        assert!(matches!(
            actions.as_slice(),
            [Action::NavTo(Nav::Participants)]
        ));

        // The app routes the NavTo back into the header.
        header.set_selected_menu(Menu::CelExpressions);
        assert_eq!(header.workspace(), Workspace::Admin);
        let actions = step(&mut header, HeaderMsg::SwitchWorkspace(None)).await;
        assert!(matches!(
            actions.as_slice(),
            [Action::NavTo(Nav::PoliciesList)]
        ));
        header.set_selected_menu(Menu::Policies);
        let actions = step(
            &mut header,
            HeaderMsg::SwitchWorkspace(Some(Workspace::Admin)),
        )
        .await;
        assert!(matches!(
            actions.as_slice(),
            [Action::NavTo(Nav::CelExpressions)]
        ));
    }

    #[tokio::test]
    async fn switching_to_the_current_workspace_is_a_noop() {
        let mut header = header(Some(ConnectorApiVersion::V5), Menu::Policies);
        let actions = step(
            &mut header,
            HeaderMsg::SwitchWorkspace(Some(Workspace::Operations)),
        )
        .await;
        assert!(actions.is_empty());
    }

    #[tokio::test]
    async fn admin_workspace_is_refused_below_v5() {
        for version in [ConnectorApiVersion::V3, ConnectorApiVersion::V4] {
            let mut header = header(Some(version), Menu::Assets);
            let actions = step(&mut header, HeaderMsg::SwitchWorkspace(None)).await;
            match actions.as_slice() {
                [Action::Notification(noty)] => {
                    assert!(noty.msg().contains("requires API version v5"))
                }
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn ctrl_a_toggles_and_plain_a_does_not() {
        let header = header(None, Menu::Assets);
        let msgs = header.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert!(matches!(
            msgs.as_slice(),
            [ComponentMsg(HeaderMsg::SwitchWorkspace(None))]
        ));
        assert!(header
            .handle_key(KeyEvent::from(KeyCode::Char('a')))
            .is_empty());
    }
}
