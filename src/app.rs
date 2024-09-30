use std::{rc::Rc, time::Duration};
mod action;
mod fetch;
pub mod model;
mod msg;

use crossterm::event::{self, Event, KeyCode};
use edc_connector_client::{Auth, EdcConnectorClient};
use futures::FutureExt;
use keyring::Entry;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::{
    components::{
        agreements::ContractAgreementsComponent, assets::AssetsComponent,
        connectors::ConnectorsComponent, contract_definitions::ContractDefinitionsComponent,
        contract_negotiations::ContractNegotiationsComponent, footer::Footer,
        header::HeaderComponent, launch_bar::LaunchBar, policies::PolicyDefinitionsComponent,
        transfer_processes::TransferProcessesComponent, Component, ComponentEvent, ComponentMsg,
        ComponentReturn, Notification, NotificationMsg,
    },
    config::{AuthKind, Config, ConnectorConfig},
    types::{
        connector::{Connector, ConnectorStatus},
        info::InfoSheet,
        nav::{Menu, Nav},
    },
};

use self::{model::AppFocus, msg::AppMsg};

const SERVICE: &str = "edc-connector-tui";

pub struct App {
    connectors: ConnectorsComponent,
    policies: PolicyDefinitionsComponent,
    assets: AssetsComponent,
    contract_definitions: ContractDefinitionsComponent,
    contract_negotiations: ContractNegotiationsComponent,
    contract_agreements: ContractAgreementsComponent,
    transfer_processes: TransferProcessesComponent,
    launch_bar: LaunchBar,
    launch_bar_visible: bool,
    focus: AppFocus,
    header: HeaderComponent,
    footer: Footer,
}

impl App {
    fn auth(cfg: &ConnectorConfig) -> (ConnectorStatus, Auth) {
        match cfg.auth() {
            AuthKind::NoAuth => (ConnectorStatus::Connected, Auth::NoAuth),
            AuthKind::Token { token_alias } => {
                let entry = Entry::new(SERVICE, token_alias).and_then(|entry| entry.get_password());

                match entry {
                    Ok(pwd) => (ConnectorStatus::Connected, Auth::api_token(pwd)),
                    Err(_err) => (
                        ConnectorStatus::Custom(format!(
                            "Token not found for alias {}",
                            token_alias
                        )),
                        Auth::NoAuth,
                    ),
                }
            }
        }
    }

    fn init_connector(cfg: ConnectorConfig) -> Connector {
        let (status, auth) = Self::auth(&cfg);
        let client = EdcConnectorClient::builder()
            .management_url(cfg.address())
            .with_auth(auth)
            .build()
            .unwrap();
        Connector::new(cfg, client, status)
    }

    pub fn init_with_connectors(connectors: Vec<Connector>) -> App {
        let connectors = ConnectorsComponent::new(connectors);

        let sheet = connectors.info_sheet().merge(Self::info_sheet());

        App {
            connectors,
            policies: PolicyDefinitionsComponent::default().on_fetch(Self::fetch_policies),
            assets: AssetsComponent::default().on_fetch(Self::fetch_assets),
            contract_definitions: ContractDefinitionsComponent::default()
                .on_fetch(Self::fetch_contract_definitions),
            contract_negotiations: ContractNegotiationsComponent::default()
                .on_fetch(Self::fetch_contract_negotiations),
            contract_agreements: ContractAgreementsComponent::default()
                .on_fetch(Self::fetch_contract_agreements),
            transfer_processes: TransferProcessesComponent::default()
                .on_fetch(Self::fetch_transfer_processes),
            launch_bar: LaunchBar::default(),
            launch_bar_visible: false,
            focus: AppFocus::ConnectorList,
            footer: Footer::default(),
            header: HeaderComponent::with_sheet(sheet),
        }
    }

    pub fn init(cfg: Config) -> App {
        let connectors = cfg
            .connectors
            .into_iter()
            .map(App::init_connector)
            .collect();

        Self::init_with_connectors(connectors)
    }

    pub fn info_sheet() -> InfoSheet {
        InfoSheet::default()
            .key_binding("<tab>", "Switch menu")
            .key_binding("<esc>", "Back/Clear")
            .key_binding("<:>", "Launch bar")
            .key_binding("<:q>", "Quit")
    }

    pub fn show_notification(
        &mut self,
        noty: Notification,
    ) -> anyhow::Result<ComponentReturn<AppMsg>> {
        let timeout = noty.timeout();
        self.footer.show_notification(noty);

        Ok(ComponentReturn::cmd(
            async move {
                tokio::time::sleep(Duration::from_secs(timeout)).await;
                Ok(vec![AppMsg::NontificationMsg(NotificationMsg::Clear).into()])
            }
            .boxed(),
        ))
    }

    pub fn clear_notification(&mut self) -> anyhow::Result<ComponentReturn<AppMsg>> {
        self.footer.clear_notification();
        Ok(ComponentReturn::empty())
    }

    pub fn change_sheet(&mut self) -> anyhow::Result<ComponentReturn<AppMsg>> {
        let component_sheet = match self.header.selected_menu() {
            Menu::Connectors => InfoSheet::default(),
            Menu::Assets => self.assets.info_sheet(),
            Menu::Policies => self.policies.info_sheet(),
            Menu::ContractDefinitions => self.contract_definitions.info_sheet(),
            Menu::ContractNegotiations => self.contract_negotiations.info_sheet(),
            Menu::ContractAgreements => self.contract_agreements.info_sheet(),
            Menu::TransferProcesses => self.transfer_processes.info_sheet(),
        };

        self.header.update_sheet(
            self.connectors
                .info_sheet()
                .merge(Self::info_sheet())
                .merge(component_sheet),
        );
        Ok(ComponentReturn::empty())
    }

    pub async fn handle_routing(&mut self, nav: Nav) -> anyhow::Result<ComponentReturn<AppMsg>> {
        self.launch_bar_visible = false;
        self.launch_bar.clear();
        self.header.set_selected_menu(nav);
        self.change_sheet()?;
        match self.header.selected_menu() {
            Menu::Connectors => {
                self.focus = AppFocus::ConnectorList;
                Ok(ComponentReturn::empty())
            }
            Menu::Assets => {
                self.focus = AppFocus::Assets;
                if let Some(connector) = self.connectors.selected() {
                    return Self::forward_init(
                        &mut self.assets,
                        connector.clone(),
                        AppMsg::AssetsMsg,
                    )
                    .await;
                }
                Ok(ComponentReturn::empty())
            }
            Menu::Policies => {
                self.focus = AppFocus::Policies;
                if let Some(connector) = self.connectors.selected() {
                    return Self::forward_init(
                        &mut self.policies,
                        connector.clone(),
                        AppMsg::PoliciesMsg,
                    )
                    .await;
                }
                Ok(ComponentReturn::empty())
            }
            Menu::ContractDefinitions => {
                self.focus = AppFocus::ContractDefinitions;
                if let Some(connector) = self.connectors.selected() {
                    return Self::forward_init(
                        &mut self.contract_definitions,
                        connector.clone(),
                        AppMsg::ContractDefinitions,
                    )
                    .await;
                }
                Ok(ComponentReturn::empty())
            }
            Menu::ContractAgreements => {
                self.focus = AppFocus::ContractAgreements;
                if let Some(connector) = self.connectors.selected() {
                    return Self::forward_init(
                        &mut self.contract_agreements,
                        connector.clone(),
                        AppMsg::ContractAgreements,
                    )
                    .await;
                }
                Ok(ComponentReturn::empty())
            }
            Menu::ContractNegotiations => {
                self.focus = AppFocus::ContractNegotiations;
                if let Some(connector) = self.connectors.selected() {
                    return Self::forward_init(
                        &mut self.contract_negotiations,
                        connector.clone(),
                        AppMsg::ContractNegotiations,
                    )
                    .await;
                }
                Ok(ComponentReturn::empty())
            }
            Menu::TransferProcesses => {
                self.focus = AppFocus::TransferProcesses;
                if let Some(connector) = self.connectors.selected() {
                    return Self::forward_init(
                        &mut self.transfer_processes,
                        connector.clone(),
                        AppMsg::TransferProcesses,
                    )
                    .await;
                }
                Ok(ComponentReturn::empty())
            }
        }
    }
}

#[async_trait::async_trait]
impl Component for App {
    type Msg = AppMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        let main = self.main_layout(rect);

        self.header.view(f, main[0]);
        self.launch_bar.view(f, main[1]);

        match self.header.selected_menu() {
            Menu::Connectors => self.connectors.view(f, main[2]),
            Menu::Assets => self.assets.view(f, main[2]),
            Menu::Policies => self.policies.view(f, main[2]),
            Menu::ContractDefinitions => self.contract_definitions.view(f, main[2]),
            Menu::ContractNegotiations => self.contract_negotiations.view(f, main[2]),
            Menu::ContractAgreements => self.contract_agreements.view(f, main[2]),
            Menu::TransferProcesses => self.transfer_processes.view(f, main[2]),
        }

        self.footer.view(f, main[3]);
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<AppMsg>> {
        match msg.take() {
            AppMsg::ConnectorsMsg(m) => {
                Self::forward_update::<_, ConnectorsComponent>(
                    &mut self.connectors,
                    m.into(),
                    AppMsg::ConnectorsMsg,
                )
                .await
            }
            AppMsg::ShowLaunchBar => {
                self.launch_bar_visible = true;
                self.focus = AppFocus::LaunchBar;
                Ok(ComponentReturn::empty())
            }
            AppMsg::HideLaunchBar => {
                self.launch_bar.clear();
                self.launch_bar_visible = false;
                self.focus = AppFocus::ConnectorList;
                Ok(ComponentReturn::empty())
            }
            AppMsg::LaunchBarMsg(m) => {
                Self::forward_update(&mut self.launch_bar, m.into(), AppMsg::LaunchBarMsg).await
            }
            AppMsg::AssetsMsg(m) => {
                Self::forward_update(&mut self.assets, m.into(), AppMsg::AssetsMsg).await
            }
            AppMsg::PoliciesMsg(m) => {
                Self::forward_update(&mut self.policies, m.into(), AppMsg::PoliciesMsg).await
            }
            AppMsg::ContractDefinitions(m) => {
                Self::forward_update(
                    &mut self.contract_definitions,
                    m.into(),
                    AppMsg::ContractDefinitions,
                )
                .await
            }
            AppMsg::ContractNegotiations(m) => {
                Self::forward_update(
                    &mut self.contract_negotiations,
                    m.into(),
                    AppMsg::ContractNegotiations,
                )
                .await
            }
            AppMsg::ContractAgreements(m) => {
                Self::forward_update(
                    &mut self.contract_agreements,
                    m.into(),
                    AppMsg::ContractAgreements,
                )
                .await
            }
            AppMsg::TransferProcesses(m) => {
                Self::forward_update(
                    &mut self.transfer_processes,
                    m.into(),
                    AppMsg::TransferProcesses,
                )
                .await
            }
            AppMsg::HeaderMsg(m) => {
                Self::forward_update(&mut self.header, m.into(), AppMsg::HeaderMsg).await
            }
            AppMsg::RoutingMsg(nav) => self.handle_routing(nav).await,
            AppMsg::ChangeSheet => self.change_sheet(),
            AppMsg::NontificationMsg(NotificationMsg::Show(noty)) => self.show_notification(noty),
            AppMsg::NontificationMsg(NotificationMsg::Clear) => self.clear_notification(),
        }
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        let msg = match self.focus {
            AppFocus::ConnectorList => {
                Self::forward_event(&mut self.connectors, evt.clone(), AppMsg::ConnectorsMsg)?
            }
            AppFocus::LaunchBar => {
                Self::forward_event(&mut self.launch_bar, evt.clone(), AppMsg::LaunchBarMsg)?
            }
            AppFocus::Assets => {
                Self::forward_event(&mut self.assets, evt.clone(), AppMsg::AssetsMsg)?
            }
            AppFocus::Policies => {
                Self::forward_event(&mut self.policies, evt.clone(), AppMsg::PoliciesMsg)?
            }
            AppFocus::ContractDefinitions => Self::forward_event(
                &mut self.contract_definitions,
                evt.clone(),
                AppMsg::ContractDefinitions,
            )?,
            AppFocus::ContractNegotiations => Self::forward_event(
                &mut self.contract_negotiations,
                evt.clone(),
                AppMsg::ContractNegotiations,
            )?,
            AppFocus::ContractAgreements => Self::forward_event(
                &mut self.contract_agreements,
                evt.clone(),
                AppMsg::ContractAgreements,
            )?,
            AppFocus::TransferProcesses => Self::forward_event(
                &mut self.transfer_processes,
                evt.clone(),
                AppMsg::TransferProcesses,
            )?,
        };

        let msg = if msg.is_empty() {
            Self::forward_event(&mut self.header, evt.clone(), AppMsg::HeaderMsg)?
        } else {
            msg
        };

        if msg.is_empty() {
            if let ComponentEvent::Event(Event::Key(key)) = evt {
                if key.kind == event::KeyEventKind::Press {
                    return Ok(Self::handle_key(key));
                }
            }
        } else {
            return Ok(msg);
        }

        Ok(vec![])
    }
}

impl App {
    fn main_layout(&self, rect: Rect) -> Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(10),
                    Constraint::Percentage(if self.launch_bar_visible { 5 } else { 0 }),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(rect)
    }

    fn handle_key(key: event::KeyEvent) -> Vec<ComponentMsg<AppMsg>> {
        match key.code {
            KeyCode::Char(':') => vec![(AppMsg::ShowLaunchBar.into())],
            _ => vec![],
        }
    }
}
