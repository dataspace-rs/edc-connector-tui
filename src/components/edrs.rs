use crate::components::resources::Field;

use super::{
    resources::{msg::ResourcesMsg, DrawableResource, FieldValue, ResourcesComponent},
    table::TableEntry,
};
use edc_connector_client::types::{data_address::DataAddress, edr::EndpointDataReferenceEntry};
use ratatui::widgets::Row;

pub type EdrsMsg = ResourcesMsg<EdrMetadataEntry, EdrEntry>;
pub type EdrsComponent = ResourcesComponent<EdrMetadataEntry, EdrEntry>;

#[derive(Debug, Clone)]
pub struct EdrMetadataEntry(EndpointDataReferenceEntry);

#[derive(Debug, Clone)]
pub struct EdrEntry(String, DataAddress);

impl EdrMetadataEntry {
    pub fn new(edr_entry: EndpointDataReferenceEntry) -> EdrMetadataEntry {
        EdrMetadataEntry(edr_entry)
    }

    pub fn id(&self) -> &str {
        self.0.transfer_process_id()
    }
}

impl TableEntry for EdrMetadataEntry {
    fn row(&self) -> Row {
        Row::new(vec![
            self.0.transfer_process_id().to_string(),
            self.0.asset_id().to_string(),
            self.0.agreement_id().to_string(),
            self.0.provider_id().to_string(),
            self.0
                .contract_negotiation_id()
                .map(String::to_string)
                .unwrap_or_default(),
            format!("{}", self.0.created_at()),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec![
            "TRANSFER_PROCESS_ID",
            "ASSET_ID",
            "AGREEMENT_ID",
            "PROVIDER_ID",
            "NEGOTIATION_ID",
            "CREATED_AT",
        ])
    }
}

impl EdrEntry {
    pub fn new(id: String, data_address: DataAddress) -> Self {
        Self(id, data_address)
    }
}

impl DrawableResource for EdrEntry {
    fn id(&self) -> &str {
        &self.0
    }

    fn title() -> &'static str {
        "Edrs"
    }

    fn fields(&self) -> Vec<super::resources::Field> {
        let mut fields = vec![Field::new(
            "id".to_string(),
            FieldValue::Str(self.0.to_string()),
        )];

        fields.push(Field::new(
            "data_address".to_string(),
            FieldValue::Json(serde_json::to_string_pretty(&self.1).unwrap()),
        ));

        fields
    }
}
