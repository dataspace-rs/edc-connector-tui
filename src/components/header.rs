use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Tabs},
    Frame,
};

use crate::types::{info::InfoSheet, nav::Menu};

use self::{help::InfoComponent, msg::HeaderMsg};

use super::{Component, ComponentEvent, ComponentMsg, ComponentReturn, StatelessComponent};

pub mod help;
pub mod msg;

pub struct HeaderComponent {
    menu: Menu,
    menus: Vec<Menu>,
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
            menus: Menu::available_for(None),
            info: InfoComponent::default(),
            sheet,
        }
    }

    pub fn set_selected_menu(&mut self, menu: impl Into<Menu>) {
        self.menu = menu.into();
    }

    /// Replace the list of menu entries shown in the tab bar and cycled with Tab/Shift+Tab.
    pub fn set_menus(&mut self, menus: Vec<Menu>) {
        self.menus = menus;
    }

    fn selected_position(&self) -> Option<usize> {
        self.menus.iter().position(|m| m == &self.menu)
    }

    /// Move the selection by `step` entries within the visible menu list, wrapping around.
    fn step_menu(&mut self, step: isize) -> ComponentReturn<HeaderMsg> {
        let len = self.menus.len();
        if len == 0 {
            return ComponentReturn::empty();
        }
        let pos = self.selected_position().unwrap_or(0) as isize;
        let idx = (pos + step).rem_euclid(len as isize) as usize;
        self.menu = self.menus[idx].clone();
        ComponentReturn::action(super::Action::NavTo(self.menu.clone().into()))
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

        let tabs = Tabs::new(self.menus.iter().map(Menu::name))
            .block(Block::bordered().title("Menu"))
            .style(Style::default().white())
            .highlight_style(Style::default().yellow())
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
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{components::Action, config::ConnectorApiVersion, types::nav::Nav};

    fn header(menus: Vec<Menu>, selected: Menu) -> HeaderComponent {
        let mut header = HeaderComponent::default();
        header.set_menus(menus);
        header.set_selected_menu(selected);
        header
    }

    async fn step(header: &mut HeaderComponent, msg: HeaderMsg) -> Vec<Action> {
        header.update(msg.into()).await.unwrap().actions
    }

    #[tokio::test]
    async fn next_tab_reaches_edrs_when_available() {
        let mut header = header(Menu::available_for(None), Menu::TransferProcesses);
        let actions = step(&mut header, HeaderMsg::NextTab).await;
        assert_eq!(header.selected_menu(), &Menu::Edrs);
        assert!(matches!(actions.as_slice(), [Action::NavTo(Nav::Edrs)]));
    }

    #[tokio::test]
    async fn next_tab_skips_edrs_for_v4() {
        let mut header = header(
            Menu::available_for(Some(ConnectorApiVersion::V4)),
            Menu::TransferProcesses,
        );
        let actions = step(&mut header, HeaderMsg::NextTab).await;
        assert_eq!(header.selected_menu(), &Menu::DataPlanes);
        assert!(matches!(
            actions.as_slice(),
            [Action::NavTo(Nav::DataPlanes)]
        ));
    }

    #[tokio::test]
    async fn prev_tab_skips_edrs_for_v4() {
        let mut header = header(
            Menu::available_for(Some(ConnectorApiVersion::V4)),
            Menu::DataPlanes,
        );
        let actions = step(&mut header, HeaderMsg::PrevTab).await;
        assert_eq!(header.selected_menu(), &Menu::TransferProcesses);
        assert!(matches!(
            actions.as_slice(),
            [Action::NavTo(Nav::TransferProcesses)]
        ));
    }

    #[tokio::test]
    async fn tabs_wrap_around_in_both_directions() {
        let mut header = header(Menu::available_for(None), Menu::DataPlanes);
        step(&mut header, HeaderMsg::NextTab).await;
        assert_eq!(header.selected_menu(), &Menu::Connectors);
        step(&mut header, HeaderMsg::PrevTab).await;
        assert_eq!(header.selected_menu(), &Menu::DataPlanes);
    }

    #[tokio::test]
    async fn empty_menu_list_is_a_noop() {
        let mut header = header(vec![], Menu::Assets);
        let actions = step(&mut header, HeaderMsg::NextTab).await;
        assert!(actions.is_empty());
        assert_eq!(header.selected_menu(), &Menu::Assets);
    }
}
