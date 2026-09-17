use crate::types::nav::Workspace;

#[derive(Debug)]
pub enum HeaderMsg {
    NextTab,
    PrevTab,
    /// Switch to `Some(workspace)`, or toggle between the two with `None`.
    SwitchWorkspace(Option<Workspace>),
}
