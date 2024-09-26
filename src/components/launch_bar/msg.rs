use tui_textarea::Input;

use crate::types::nav::Nav;

#[derive(Debug)]
pub enum LaunchBarMsg {
    AppendCommand(Input),
    Quit,
    NavTo(Nav),
    Error(String),
    Loop,
    Esc,
}
