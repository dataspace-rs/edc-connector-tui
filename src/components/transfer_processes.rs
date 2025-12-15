use edc_connector_client::types::transfer_process::TransferProcess;
use ratatui::widgets::Row;

use crate::components::resources::FieldValue;

use super::{
    resources::{msg::ResourcesMsg, DrawableResource, Field, ResourcesComponent},
    table::TableEntry,
};

#[derive(Debug, Clone)]
pub struct TransferProcessEntry(TransferProcess);

impl TransferProcessEntry {
    pub fn new(transfer_process: TransferProcess) -> Self {
        Self(transfer_process)
    }
}

pub type TransferProcessMsg = ResourcesMsg<TransferProcessEntry, TransferProcessEntry>;
pub type TransferProcessesComponent =
    ResourcesComponent<TransferProcessEntry, TransferProcessEntry>;

impl TableEntry for TransferProcessEntry {
    fn row(&self) -> Row<'_> {
        let private_properties = serde_json::to_string(self.0.private_properties()).unwrap();
        Row::new(vec![
            self.0.id().to_string(),
            format!("{:?}", self.0.kind()),
            format!("{:?}", self.0.state()),
            self.0.transfer_type().to_string(),
            self.0.asset_id().to_string(),
            self.0.contract_id().to_string(),
            private_properties,
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec![
            "ID",
            "TYPE",
            "STATE",
            "TRANSFER_TYPE",
            "ASSET_ID",
            "CONTRACT_AGREEMENT_ID",
            "PRIVATE_PROPERTIES",
        ])
    }
}

impl DrawableResource for TransferProcessEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "Transfer Processes"
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field::string("id", self.0.id()),
            Field::string("type", format!("{:?}", self.0.kind())),
            Field::string("state", format!("{:?}", self.0.state())),
            Field::string("transferType", self.0.transfer_type()),
            Field::string("asset_id", self.0.asset_id()),
            Field::string("contract_agreement_id", self.0.contract_id()),
            Field::string(
                "correlation_id",
                self.0.correlation_id().cloned().unwrap_or_default(),
            ),
            Field::new(
                "callback_addresses".to_string(),
                FieldValue::Json(
                    serde_json::to_string_pretty(&self.0.callback_addresses()).unwrap(),
                ),
            ),
            Field::new(
                "private_properties".to_string(),
                FieldValue::Json(
                    serde_json::to_string_pretty(&self.0.private_properties()).unwrap(),
                ),
            ),
        ]
    }
}
