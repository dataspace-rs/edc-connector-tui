use ratatui::{layout::Rect, widgets::Row, Frame};

use crate::types::{connector::Connector, info::InfoSheet, nav::Nav};

use self::msg::ConnectorsMsg;

use super::{
    table::{msg::TableMsg, TableEntry, UiTable},
    Action, Component, ComponentEvent, ComponentMsg, ComponentReturn,
};

pub mod msg;

pub type ConnectorsTable = UiTable<ConnectorEntry, Box<ConnectorsMsg>>;

#[derive(Debug, Default)]
pub struct ConnectorsComponent {
    table: ConnectorsTable,
    selected: Option<Connector>,
}

#[derive(Debug)]
pub struct ConnectorEntry(Connector);

impl TableEntry for ConnectorEntry {
    fn row(&self) -> Row<'_> {
        Row::new(vec![
            self.0.config().name(),
            self.0.config().address(),
            match self.0.config().version() {
                crate::config::ConnectorApiVersion::V3 => "v3",
                crate::config::ConnectorApiVersion::V4 => "v4",
            },
            self.0.config().auth().kind(),
            self.0.status().as_str(),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec!["NAME", "ADDRESS", "API VERSION", "AUTH", "STATUS"])
    }
}

#[async_trait::async_trait]
impl Component for ConnectorsComponent {
    type Msg = ConnectorsMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        self.table.view(f, rect);
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            ConnectorsMsg::ConnectorSelected(connector) => {
                self.selected = Some(connector.clone());
                Ok(ComponentReturn::action(Action::NavTo(Nav::AssetsList)))
            }
            ConnectorsMsg::TableEvent(table) => {
                Self::forward_update::<_, ConnectorsTable>(
                    &mut self.table,
                    table.into(),
                    ConnectorsMsg::TableEvent,
                )
                .await
            }
        }
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        Self::forward_event(&mut self.table, evt, |msg| match msg {
            TableMsg::Local(table) => ConnectorsMsg::TableEvent(TableMsg::Local(table)),
            TableMsg::Outer(outer) => *outer,
        })
    }
}

impl ConnectorsComponent {
    pub fn new(connectors: Vec<Connector>) -> Self {
        let selected = connectors.first().cloned();
        Self {
            table: ConnectorsTable::with_elements(
                "Connectors".to_string(),
                connectors.into_iter().map(ConnectorEntry).collect(),
                true,
            )
            .on_select(|connector| Box::new(ConnectorsMsg::ConnectorSelected(connector.0.clone()))),
            selected,
        }
    }

    pub fn selected(&self) -> Option<&Connector> {
        self.selected.as_ref()
    }

    pub fn info_sheet(&self) -> InfoSheet {
        if let Some(c) = self.selected.as_ref() {
            InfoSheet::default()
                .info("Connector Name", c.config().name())
                .info("Connector Address", c.config().address())
        } else {
            InfoSheet::default()
                .info("Connector Name", "n/a")
                .info("Connector Address", "n/a")
        }
    }
}
