use edc_connector_client::types::dataplane::DataPlaneInstance;
use ratatui::widgets::Row;

use crate::components::resources::FieldValue;

use super::{
    resources::{msg::ResourcesMsg, DrawableResource, Field, ResourcesComponent},
    table::TableEntry,
};

#[derive(Debug, Clone)]
pub struct DataPlaneEntry(DataPlaneInstance);

impl DataPlaneEntry {
    pub fn new(data_plane: DataPlaneInstance) -> Self {
        Self(data_plane)
    }
}

pub type DataPlaneMsg = ResourcesMsg<DataPlaneEntry, DataPlaneEntry>;
pub type DataPlanesComponent = ResourcesComponent<DataPlaneEntry, DataPlaneEntry>;

impl TableEntry for DataPlaneEntry {
    fn row(&self) -> Row {
        Row::new(vec![
            self.0.id().to_string(),
            format!("{:}", self.0.url()),
            format!("{:?}", self.0.state()),
            format!("{:?}", self.0.allowed_transfer_types()),
            format!("{:?}", self.0.allowed_source_types()),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec![
            "ID",
            "URL",
            "STATE",
            "ALLOWED_TRANSFER_TYPES",
            "ALLOWED_SOURCE_TYPES",
        ])
    }
}

impl DrawableResource for DataPlaneEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "Dataplanes"
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field::string("id", self.0.id()),
            Field::string("url", format!("{:?}", self.0.url())),
            Field::string("state", format!("{:?}", self.0.state())),
            Field::string(
                "allowedTransferTypes",
                format!("{:?}", self.0.allowed_transfer_types()),
            ),
            Field::string(
                "allowedSourceTypes",
                format!("{:?}", self.0.allowed_source_types()),
            ),
            Field::string(
                "allowedDestTypes",
                format!("{:?}", self.0.allowed_dest_types()),
            ),
            Field::new(
                "properties".to_string(),
                FieldValue::Json(serde_json::to_string_pretty(&self.0.properties()).unwrap()),
            ),
        ]
    }
}
