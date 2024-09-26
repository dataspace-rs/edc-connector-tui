use crate::{components::table::msg::TableMsg, types::connector::Connector};

#[derive(Debug)]
pub enum ConnectorsMsg {
    TableEvent(TableMsg<Box<ConnectorsMsg>>),
    ConnectorSelected(Connector),
}
