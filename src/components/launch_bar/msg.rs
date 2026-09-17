use tui_input::InputRequest;

use crate::types::nav::{Nav, Workspace};

#[derive(Debug)]
pub enum LaunchBarMsg {
    AppendCommand(InputRequest),
    Quit,
    NavTo(Nav),
    SwitchWorkspace(Workspace),
    Error(String),
    Loop,
    Esc,
}
