use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use enum_ordinalize::Ordinalize;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, Tabs},
    Frame,
};

use crate::types::{info::InfoSheet, nav::Menu};

use self::{help::InfoComponent, msg::HeaderMsg};

use super::{Component, ComponentEvent, ComponentMsg, ComponentReturn, StatelessComponent};

pub mod help;
pub mod msg;

#[derive(Default)]
pub struct HeaderComponent {
    menu: Menu,
    info: InfoComponent,
    sheet: InfoSheet,
}

impl HeaderComponent {
    pub fn with_sheet(sheet: InfoSheet) -> HeaderComponent {
        HeaderComponent {
            menu: Menu::default(),
            info: InfoComponent::default(),
            sheet,
        }
    }
    pub fn set_selected_menu(&mut self, menu: impl Into<Menu>) {
        self.menu = menu.into();
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

        let tabs = Tabs::new(Menu::names())
            .block(Block::bordered().title("Menu"))
            .style(Style::default().white())
            .highlight_style(Style::default().yellow())
            .select(self.menu.ordinal())
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
            HeaderMsg::NextTab => {
                let current = self.menu.clone();
                let idx = (self.menu.ordinal() + 1) % Menu::VALUES.len();
                self.menu = Menu::from_ordinal(idx).unwrap_or(current);
                Ok(ComponentReturn::action(super::Action::NavTo(
                    self.menu.clone().into(),
                )))
            }
            HeaderMsg::PrevTab => {
                let current = self.menu.clone();
                let initial = if self.menu.ordinal() == 0 {
                    Menu::VALUES.len()
                } else {
                    self.menu.ordinal()
                };
                let idx = (initial - 1) % Menu::VALUES.len();
                self.menu = Menu::from_ordinal(idx).unwrap_or(current);
                Ok(ComponentReturn::action(super::Action::NavTo(
                    self.menu.clone().into(),
                )))
            }
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
