use std::{path::PathBuf, rc::Rc, time::Duration};
mod action;
mod admin;
mod fetch;
pub mod model;
mod msg;

use crossterm::event::{self, Event, KeyCode};
use futures::FutureExt;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::{
    components::{
        admin::{
            cached_documents::{self, CachedDocumentsComponent},
            cel_expressions::{self, CelExpressionsComponent},
            dataspace_profiles::{self, DataspaceProfilesComponent},
            dcp_scopes::{self, DcpScopesComponent},
            participants::{self, ParticipantsComponent},
            schema_validators::{self, SchemaValidatorsComponent},
        },
        agreements::ContractAgreementsComponent,
        assets::AssetsComponent,
        connectors::ConnectorsComponent,
        contract_definitions::ContractDefinitionsComponent,
        contract_negotiations::ContractNegotiationsComponent,
        dataplanes::DataPlanesComponent,
        edrs::EdrsComponent,
        footer::Footer,
        header::HeaderComponent,
        launch_bar::LaunchBar,
        policies::PolicyDefinitionsComponent,
        resources::{ListMode, ResourceAction},
        transfer_processes::TransferProcessesComponent,
        Component, ComponentEvent, ComponentMsg, ComponentReturn, Notification, NotificationMsg,
    },
    config::Config,
    types::{
        connector::Connector,
        info::InfoSheet,
        nav::{Menu, Nav},
    },
};

use self::{model::AppFocus, msg::AppMsg};

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
    participants: ParticipantsComponent,
    dataspace_profiles: DataspaceProfilesComponent,
    cel_expressions: CelExpressionsComponent,
    cached_documents: CachedDocumentsComponent,
    dcp_scopes: DcpScopesComponent,
    schema_validators: SchemaValidatorsComponent,
    launch_bar: LaunchBar,
    launch_bar_visible: bool,
    focus: AppFocus,
    header: HeaderComponent,
    footer: Footer,
}

impl App {
    pub fn init_with_connectors(connectors: Vec<Connector>, config_path: Option<PathBuf>) -> App {
        let connectors = ConnectorsComponent::new(connectors, config_path);

        let sheet = connectors.info_sheet().merge(Self::info_sheet());
        let mut header = HeaderComponent::with_sheet(sheet);
        header.set_version(connectors.selected_version());

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
            participants: ParticipantsComponent::default()
                .on_fetch(Self::fetch_participants)
                .on_single_fetch(Self::participant_detail)
                .list_mode(ListMode::Paged)
                .editable(participants::form)
                .on_create(Self::create_participant)
                .on_update(Self::update_participant)
                .on_delete(|e| e.inner().id().to_string(), Self::delete_participant)
                .action(ResourceAction::with_form(
                    'l',
                    "Associate profiles",
                    participants::profiles_form,
                    Self::associate_participant_profiles,
                ))
                .action(ResourceAction::with_form(
                    'c',
                    "Edit config",
                    participants::config_form,
                    Self::save_participant_config,
                )),
            dataspace_profiles: DataspaceProfilesComponent::default()
                .on_fetch(Self::fetch_dataspace_profiles)
                .on_single_fetch(Self::identity)
                .editable(dataspace_profiles::form)
                .on_create(Self::create_dataspace_profile)
                .on_update(Self::update_dataspace_profile)
                .on_delete(
                    |e| e.inner().name().to_string(),
                    Self::delete_dataspace_profile,
                ),
            cel_expressions: CelExpressionsComponent::default()
                .on_fetch(Self::fetch_cel_expressions)
                .on_single_fetch(Self::identity)
                .editable(cel_expressions::form)
                .on_create(Self::create_cel_expression)
                .on_update(Self::update_cel_expression)
                .on_delete(|e| e.inner().id().to_string(), Self::delete_cel_expression)
                .action(ResourceAction::with_form(
                    't',
                    "Test expression",
                    cel_expressions::test_form,
                    Self::test_cel_expression,
                )),
            cached_documents: CachedDocumentsComponent::default()
                .on_fetch(Self::fetch_cached_documents)
                .on_single_fetch(Self::identity)
                .list_mode(ListMode::Plain)
                .editable(cached_documents::form)
                .on_create(Self::create_cached_document)
                .on_update(Self::update_cached_document)
                .on_delete(|e| e.inner().id().to_string(), Self::delete_cached_document)
                .action(ResourceAction::immediate(
                    'u',
                    "Refresh document",
                    Self::refresh_cached_document,
                )),
            dcp_scopes: DcpScopesComponent::default()
                .on_fetch(Self::fetch_dcp_scopes)
                .on_single_fetch(Self::identity)
                .editable(dcp_scopes::form)
                .on_create(Self::create_dcp_scope)
                .on_update(Self::update_dcp_scope)
                .on_delete(|e| e.inner().id().to_string(), Self::delete_dcp_scope),
            schema_validators: SchemaValidatorsComponent::default()
                .on_fetch(Self::fetch_schema_validators)
                .on_single_fetch(Self::identity)
                .list_mode(ListMode::Plain)
                .editable(schema_validators::form)
                .on_create(Self::create_schema_validator)
                .on_update(Self::update_schema_validator)
                .on_delete(
                    |e| e.inner().id().to_string(),
                    Self::delete_schema_validator,
                ),
            launch_bar: LaunchBar::default(),
            launch_bar_visible: false,
            focus: AppFocus::ConnectorList,
            footer: Footer::default(),
            header,
        }
    }

    pub fn init(cfg: Config, config_path: PathBuf) -> anyhow::Result<App> {
        let connectors = cfg
            .connectors
            .into_iter()
            .map(Connector::from_config)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(Self::init_with_connectors(connectors, Some(config_path)))
    }

    pub fn info_sheet() -> InfoSheet {
        InfoSheet::default()
            .key_binding("<tab>", "Next menu")
            .key_binding("<tab+shift>", "Prev menu")
            .key_binding("<ctrl+a>", "Admin/Ops")
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
        self.header.set_version(self.connectors.selected_version());
        let component_sheet = match self.header.selected_menu() {
            Menu::Connectors => self.connectors.key_bindings(),
            Menu::Assets => self.assets.info_sheet(),
            Menu::Policies => self.policies.info_sheet(),
            Menu::ContractDefinitions => self.contract_definitions.info_sheet(),
            Menu::ContractNegotiations => self.contract_negotiations.info_sheet(),
            Menu::ContractAgreements => self.contract_agreements.info_sheet(),
            Menu::TransferProcesses => self.transfer_processes.info_sheet(),
            Menu::Edrs => self.edrs.info_sheet(),
            Menu::DataPlanes => self.dataplanes.info_sheet(),
            Menu::Participants => self.participants.info_sheet(),
            Menu::DataspaceProfiles => self.dataspace_profiles.info_sheet(),
            Menu::CelExpressions => self.cel_expressions.info_sheet(),
            Menu::CachedDocuments => self.cached_documents.info_sheet(),
            Menu::DcpScopes => self.dcp_scopes.info_sheet(),
            Menu::SchemaValidators => self.schema_validators.info_sheet(),
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

        let version = self.connectors.selected_version();
        self.header.set_version(version);

        let menu: Menu = nav.into();
        if !menu.is_available_for(version) {
            return self.show_notification(Notification::error(format!(
                "{} is not available for connector API version {}",
                menu.name(),
                version.map(|v| v.as_str()).unwrap_or("n/a")
            )));
        }

        self.header.set_selected_menu(menu);
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
            (Menu::Participants, Some(connector)) => {
                self.focus = AppFocus::Participants;
                Self::forward_init(
                    &mut self.participants,
                    connector.clone(),
                    AppMsg::Participants,
                )
                .await
            }
            (Menu::DataspaceProfiles, Some(connector)) => {
                self.focus = AppFocus::DataspaceProfiles;
                Self::forward_init(
                    &mut self.dataspace_profiles,
                    connector.clone(),
                    AppMsg::DataspaceProfiles,
                )
                .await
            }
            (Menu::CelExpressions, Some(connector)) => {
                self.focus = AppFocus::CelExpressions;
                Self::forward_init(
                    &mut self.cel_expressions,
                    connector.clone(),
                    AppMsg::CelExpressions,
                )
                .await
            }
            (Menu::CachedDocuments, Some(connector)) => {
                self.focus = AppFocus::CachedDocuments;
                Self::forward_init(
                    &mut self.cached_documents,
                    connector.clone(),
                    AppMsg::CachedDocuments,
                )
                .await
            }
            (Menu::DcpScopes, Some(connector)) => {
                self.focus = AppFocus::DcpScopes;
                Self::forward_init(&mut self.dcp_scopes, connector.clone(), AppMsg::DcpScopes).await
            }
            (Menu::SchemaValidators, Some(connector)) => {
                self.focus = AppFocus::SchemaValidators;
                Self::forward_init(
                    &mut self.schema_validators,
                    connector.clone(),
                    AppMsg::SchemaValidators,
                )
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
            Menu::Participants => self.participants.view(f, main[2]),
            Menu::DataspaceProfiles => self.dataspace_profiles.view(f, main[2]),
            Menu::CelExpressions => self.cel_expressions.view(f, main[2]),
            Menu::CachedDocuments => self.cached_documents.view(f, main[2]),
            Menu::DcpScopes => self.dcp_scopes.view(f, main[2]),
            Menu::SchemaValidators => self.schema_validators.view(f, main[2]),
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
            AppMsg::Participants(m) => {
                Self::forward_update(&mut self.participants, m.into(), AppMsg::Participants).await
            }
            AppMsg::DataspaceProfiles(m) => {
                Self::forward_update(
                    &mut self.dataspace_profiles,
                    m.into(),
                    AppMsg::DataspaceProfiles,
                )
                .await
            }
            AppMsg::CelExpressions(m) => {
                Self::forward_update(&mut self.cel_expressions, m.into(), AppMsg::CelExpressions)
                    .await
            }
            AppMsg::CachedDocuments(m) => {
                Self::forward_update(
                    &mut self.cached_documents,
                    m.into(),
                    AppMsg::CachedDocuments,
                )
                .await
            }
            AppMsg::DcpScopes(m) => {
                Self::forward_update(&mut self.dcp_scopes, m.into(), AppMsg::DcpScopes).await
            }
            AppMsg::SchemaValidators(m) => {
                Self::forward_update(
                    &mut self.schema_validators,
                    m.into(),
                    AppMsg::SchemaValidators,
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
            AppFocus::Edrs => Self::forward_event(&mut self.edrs, evt.clone(), AppMsg::Edrs)?,
            AppFocus::DataPlanes => {
                Self::forward_event(&mut self.dataplanes, evt.clone(), AppMsg::DataPlanes)?
            }
            AppFocus::Participants => {
                Self::forward_event(&mut self.participants, evt.clone(), AppMsg::Participants)?
            }
            AppFocus::DataspaceProfiles => Self::forward_event(
                &mut self.dataspace_profiles,
                evt.clone(),
                AppMsg::DataspaceProfiles,
            )?,
            AppFocus::CelExpressions => Self::forward_event(
                &mut self.cel_expressions,
                evt.clone(),
                AppMsg::CelExpressions,
            )?,
            AppFocus::CachedDocuments => Self::forward_event(
                &mut self.cached_documents,
                evt.clone(),
                AppMsg::CachedDocuments,
            )?,
            AppFocus::DcpScopes => {
                Self::forward_event(&mut self.dcp_scopes, evt.clone(), AppMsg::DcpScopes)?
            }
            AppFocus::SchemaValidators => Self::forward_event(
                &mut self.schema_validators,
                evt.clone(),
                AppMsg::SchemaValidators,
            )?,
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
