use std::collections::HashMap;

use edc_connector_client::types::query::Query;

use crate::{components::table::msg::TableMsg, widgets::form::msg::FormMsg};

use super::{filter::FilterMsg, resource::msg::ResourceMsg};

/// The (flattened) values of an action form.
pub type ActionInput = HashMap<String, String>;

#[derive(Debug)]
pub enum ResourcesMsg<T, R> {
    ResourceSelected(T),
    ResourceFetched {
        request_id: u64,
        resource: R,
    },
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
    ResourcesFetched {
        request_id: u64,
        resources: Vec<T>,
    },
    ResourcesFetchFailed {
        request_id: u64,
        error: String,
    },
    /// Open the form to create a resource.
    ShowAdd,
    /// Open the form pre-filled with the selected resource.
    ShowEdit,
    /// Ask for confirmation before deleting the selected resource.
    ShowDelete,
    ConfirmDelete,
    /// Close the open form or confirmation popup.
    Cancel,
    FormMsg(FormMsg<T>),
    /// The add/edit form was confirmed.
    Submitted(T),
    /// Run the resource action bound to the key.
    ShowAction(char),
    ActionFormMsg(FormMsg<ActionInput>),
    RunAction {
        action: usize,
        target: T,
        input: ActionInput,
    },
    /// A create/update/delete/action call succeeded; `reselect` reloads the detail view.
    Saved {
        msg: String,
        reselect: Option<T>,
    },
    OperationFailed(String),
    /// A key was swallowed by an open popup.
    Noop,
}
