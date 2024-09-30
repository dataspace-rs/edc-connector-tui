use crate::{
    components::{
        agreements::ContractAgreementsMsg, assets::AssetsMsg, connectors::msg::ConnectorsMsg,
        contract_definitions::ContractDefinitionsMsg,
        contract_negotiations::ContractNegotiationMsg, header::msg::HeaderMsg,
        launch_bar::msg::LaunchBarMsg, policies::PoliciesMsg,
        transfer_processes::TransferProcessMsg, NotificationMsg,
    },
    types::nav::Nav,
};

#[derive(Debug)]
pub enum AppMsg {
    ConnectorsMsg(ConnectorsMsg),
    ShowLaunchBar,
    HideLaunchBar,
    LaunchBarMsg(LaunchBarMsg),
    AssetsMsg(AssetsMsg),
    PoliciesMsg(PoliciesMsg),
    ContractDefinitions(ContractDefinitionsMsg),
    ContractNegotiations(ContractNegotiationMsg),
    ContractAgreements(ContractAgreementsMsg),
    TransferProcesses(TransferProcessMsg),
    HeaderMsg(HeaderMsg),
    RoutingMsg(Nav),
    NontificationMsg(NotificationMsg),
    ChangeSheet,
}
