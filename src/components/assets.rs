use crate::components::resources::Field;

use super::{
    resources::{msg::ResourcesMsg, DrawableResource, FieldValue, ResourcesComponent},
    table::TableEntry,
};
use edc_connector_client::types::{asset::Asset, data_address::DataAddress};
use ratatui::widgets::Row;

pub type AssetsMsg = ResourcesMsg<AssetEntry, AssetEntry>;
pub type AssetsComponent = ResourcesComponent<AssetEntry, AssetEntry>;

#[derive(Debug, Clone)]
pub struct AssetEntry(Asset);

impl AssetEntry {
    pub fn new(asset: Asset) -> AssetEntry {
        AssetEntry(asset)
    }
}

impl AssetEntry {
    // `data_address` is deprecated upstream in favour of `dataplane_metadata`,
    // but connectors exposing the v3 management API still return it.
    #[allow(deprecated)]
    fn data_address(&self) -> Option<&DataAddress> {
        self.0.data_address()
    }
}

impl TableEntry for AssetEntry {
    fn row(&self) -> Row<'_> {
        let properties = serde_json::to_string(self.0.properties()).unwrap();
        let private_properties = serde_json::to_string(self.0.private_properties()).unwrap();
        let data_address = serde_json::to_string(&self.data_address()).unwrap();
        let dataplane_metadata = serde_json::to_string(&self.0.dataplane_metadata()).unwrap();
        Row::new(vec![
            self.0.id().to_string(),
            properties,
            private_properties,
            data_address,
            dataplane_metadata,
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec![
            "ID",
            "PROPERTIES",
            "PRIVATE PROPERTIES",
            "DATA ADDRESS",
            "DATAPLANE METADATA",
        ])
    }
}

impl DrawableResource for AssetEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "Assets"
    }

    fn fields(&self) -> Vec<super::resources::Field> {
        let mut fields = vec![Field::new(
            "id".to_string(),
            FieldValue::Str(self.0.id().to_string()),
        )];

        fields.push(Field::new(
            "properties".to_string(),
            FieldValue::Json(serde_json::to_string_pretty(&self.0.properties()).unwrap()),
        ));

        fields.push(Field::new(
            "private_properties".to_string(),
            FieldValue::Json(serde_json::to_string_pretty(&self.0.private_properties()).unwrap()),
        ));
        fields.push(Field::new(
            "data_address".to_string(),
            FieldValue::Json(serde_json::to_string_pretty(&self.data_address()).unwrap()),
        ));
        fields.push(Field::new(
            "dataplane_metadata".to_string(),
            FieldValue::Json(serde_json::to_string_pretty(&self.0.dataplane_metadata()).unwrap()),
        ));

        fields
    }
}
