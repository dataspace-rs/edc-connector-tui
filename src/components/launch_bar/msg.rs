use tui_input::InputRequest;

use crate::types::nav::Nav;

#[derive(Debug)]
pub enum LaunchBarMsg {
    AppendCommand(InputRequest),
    Quit,
    NavTo(Nav),
    Error(String),
    Loop,
    Esc,
}
