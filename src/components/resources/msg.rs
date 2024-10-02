use edc_connector_client::types::query::Query;

use crate::components::table::msg::TableMsg;

use super::{filter::FilterMsg, resource::msg::ResourceMsg};

#[derive(Debug)]
pub enum ResourcesMsg<T, R> {
    ResourceSelected(T),
    ResourceFetched(R),
    Back,
    NextPage,
    PrevPage,
    RefreshPage,
    ShowFilters,
    HideFilters,
    ChangeQuery(Query),
    TableMsg(TableMsg<Box<ResourcesMsg<T, R>>>),
    FilterMsg(FilterMsg<Box<ResourcesMsg<T, R>>>),
    ResourceMsg(ResourceMsg),
    ResourcesFetched(Vec<T>),
    ResourcesFetchFailed(String),
}
