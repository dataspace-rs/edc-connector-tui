use super::form::{ConnectorFormMsg, ConnectorFormOutput};
use crate::{components::table::msg::TableMsg, types::connector::Connector};

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum ConnectorsMsg {
    TableEvent(TableMsg<Box<ConnectorsMsg>>),
    ConnectorSelected(Box<Connector>),
    /// Open the form to add a connector.
    ShowAdd,
    /// Open the form pre-filled with the highlighted connector.
    ShowEdit,
    /// Ask for confirmation before deleting the highlighted connector.
    ShowDelete,
    ConfirmDelete,
    /// Close the form or the confirmation popup without changes.
    Cancel,
    FormMsg(ConnectorFormMsg),
    /// The form was confirmed with valid values.
    Submitted(Box<ConnectorFormOutput>),
    /// A key consumed by a popup, so the app does not treat it as a global key.
    Noop,
}
