use crate::components::table::msg::TableMsg;

use super::resource::msg::ResourceMsg;

#[derive(Debug)]
pub enum ResourcesMsg<T, R> {
    ResourceSelected(T),
    ResourceFetched(R),
    Back,
    NextPage,
    PrevPage,
    RefreshPage,
    TableEvent(TableMsg<Box<ResourcesMsg<T, R>>>),
    ResourceMsg(ResourceMsg),
    ResourcesFetched(Vec<T>),
    ResourcesFetchFailed(String),
}
