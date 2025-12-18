use std::{rc::Rc, time::Duration};
mod action;
mod fetch;
pub mod model;
mod msg;

use crossterm::event::{self, Event, KeyCode};
use edc_connector_client::{Auth, EdcConnectorClient, OAuth2Config};
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
        contract_negotiations::ContractNegotiationsComponent, dataplanes::DataPlanesComponent,
        edrs::EdrsComponent, footer::Footer, header::HeaderComponent, launch_bar::LaunchBar,
        policies::PolicyDefinitionsComponent, transfer_processes::TransferProcessesComponent,
        Component, ComponentEvent, ComponentMsg, ComponentReturn, Notification, NotificationMsg,
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
    edrs: EdrsComponent,
    dataplanes: DataPlanesComponent,
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
            AuthKind::OAuth {
                client_id,
                secret_alias,
                token_url,
            } => {
                let entry =
                    Entry::new(SERVICE, secret_alias).and_then(|entry| entry.get_password());

                match entry {
                    Ok(pwd) => {
                        let cfg = OAuth2Config::builder()
                            .client_id(client_id)
                            .client_secret(pwd)
                            .token_url(token_url)
                            .build();

                        match Auth::oauth(cfg) {
                            Ok(oauth) => (ConnectorStatus::Connected, oauth),
                            Err(_) => (
                                ConnectorStatus::Custom(format!(
                                    "Failed to initialize OAuth2 for alias {}",
                                    secret_alias
                                )),
                                Auth::NoAuth,
                            ),
                        }
                    }
                    Err(_err) => (
                        ConnectorStatus::Custom(format!(
                            "Secret not found for alias {}",
                            secret_alias
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
            .version(cfg.version().clone().into())
            .with_auth(auth)
            .maybe_participant_context(cfg.participant_context_id())
            .build()
            .unwrap();
        Connector::new(cfg, client, status)
    }

    pub fn init_with_connectors(connectors: Vec<Connector>) -> App {
        let connectors = ConnectorsComponent::new(connectors);

        let sheet = connectors.info_sheet().merge(Self::info_sheet());

        App {
            connectors,
            policies: PolicyDefinitionsComponent::default()
                .on_fetch(Self::fetch_policies)
                .on_single_fetch(Self::identity),
            assets: AssetsComponent::default()
                .on_fetch(Self::fetch_assets)
                .on_single_fetch(Self::identity),
            contract_definitions: ContractDefinitionsComponent::default()
                .on_fetch(Self::fetch_contract_definitions)
                .on_single_fetch(Self::identity),
            contract_negotiations: ContractNegotiationsComponent::default()
                .on_fetch(Self::fetch_contract_negotiations)
                .on_single_fetch(Self::identity),
            contract_agreements: ContractAgreementsComponent::default()
                .on_fetch(Self::fetch_contract_agreements)
                .on_single_fetch(Self::identity),
            transfer_processes: TransferProcessesComponent::default()
                .on_fetch(Self::fetch_transfer_processes)
                .on_single_fetch(Self::identity),
            edrs: EdrsComponent::default()
                .on_fetch(Self::fetch_edrs)
                .on_single_fetch(Self::single_edr),
            dataplanes: DataPlanesComponent::default()
                .on_fetch(Self::fetch_dataplanes)
                .on_single_fetch(Self::identity),
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
            .key_binding("<tab>", "Next menu")
            .key_binding("<tab+shift>", "Prev menu")
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
            Self::clear_notification_cmd(timeout).boxed(),
        ))
    }

    async fn clear_notification_cmd(timeout: u64) -> anyhow::Result<Vec<ComponentMsg<AppMsg>>> {
        tokio::time::sleep(Duration::from_secs(timeout)).await;
        Ok(vec![AppMsg::NontificationMsg(NotificationMsg::Clear).into()])
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
            Menu::Edrs => self.edrs.info_sheet(),
            Menu::DataPlanes => self.dataplanes.info_sheet(),
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
        match (self.header.selected_menu(), self.connectors.selected()) {
            (Menu::Connectors, _) => {
                self.focus = AppFocus::ConnectorList;
                Ok(ComponentReturn::empty())
            }
            (Menu::Assets, Some(connector)) => {
                self.focus = AppFocus::Assets;
                Self::forward_init(&mut self.assets, connector.clone(), AppMsg::AssetsMsg).await
            }
            (Menu::Policies, Some(connector)) => {
                self.focus = AppFocus::Policies;
                Self::forward_init(&mut self.policies, connector.clone(), AppMsg::PoliciesMsg).await
            }
            (Menu::ContractDefinitions, Some(connector)) => {
                self.focus = AppFocus::ContractDefinitions;
                Self::forward_init(
                    &mut self.contract_definitions,
                    connector.clone(),
                    AppMsg::ContractDefinitions,
                )
                .await
            }
            (Menu::ContractAgreements, Some(connector)) => {
                self.focus = AppFocus::ContractAgreements;
                Self::forward_init(
                    &mut self.contract_agreements,
                    connector.clone(),
                    AppMsg::ContractAgreements,
                )
                .await
            }
            (Menu::ContractNegotiations, Some(connector)) => {
                self.focus = AppFocus::ContractNegotiations;
                Self::forward_init(
                    &mut self.contract_negotiations,
                    connector.clone(),
                    AppMsg::ContractNegotiations,
                )
                .await
            }
            (Menu::TransferProcesses, Some(connector)) => {
                self.focus = AppFocus::TransferProcesses;
                Self::forward_init(
                    &mut self.transfer_processes,
                    connector.clone(),
                    AppMsg::TransferProcesses,
                )
                .await
            }
            (Menu::Edrs, Some(connector)) => {
                self.focus = AppFocus::Edrs;
                Self::forward_init(&mut self.edrs, connector.clone(), AppMsg::Edrs).await
            }
            (Menu::DataPlanes, Some(connector)) => {
                self.focus = AppFocus::DataPlanes;
                Self::forward_init(&mut self.dataplanes, connector.clone(), AppMsg::DataPlanes)
                    .await
            }
            (_, None) => Ok(ComponentReturn::empty()),
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
            Menu::Edrs => self.edrs.view(f, main[2]),
            Menu::DataPlanes => self.dataplanes.view(f, main[2]),
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
            AppMsg::Edrs(m) => Self::forward_update(&mut self.edrs, m.into(), AppMsg::Edrs).await,
            AppMsg::DataPlanes(m) => {
                Self::forward_update(&mut self.dataplanes, m.into(), AppMsg::DataPlanes).await
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
            AppFocus::Edrs => Self::forward_event(&mut self.edrs, evt.clone(), AppMsg::Edrs)?,
            AppFocus::DataPlanes => {
                Self::forward_event(&mut self.dataplanes, evt.clone(), AppMsg::DataPlanes)?
            }
        };

        if !msg.is_empty() {
            return Ok(msg);
        }

        let header_msg = Self::forward_event(&mut self.header, evt.clone(), AppMsg::HeaderMsg)?;

        if header_msg.is_empty() {
            if let ComponentEvent::Event(Event::Key(key)) = evt {
                if key.kind == event::KeyEventKind::Press {
                    return Ok(Self::handle_key(key));
                }
            }
            Ok(vec![])
        } else {
            Ok(header_msg)
        }
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
