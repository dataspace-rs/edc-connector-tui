use crate::{
    types::nav::{Nav, Workspace},
    widgets::text_input::TextInput,
};

use self::msg::LaunchBarMsg;
use super::{Action, Component, ComponentEvent, ComponentMsg, ComponentReturn, Notification};
use crossterm::event::{Event, KeyCode};
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders},
    Frame,
};
use tui_input::{backend::crossterm::to_input_request, Input, InputRequest};
pub mod msg;

pub static PROMPT: &str = " $> ";

#[derive(Debug, Default)]
pub struct LaunchBar {
    input: Input,
}

#[async_trait::async_trait]
impl Component for LaunchBar {
    type Msg = LaunchBarMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, rect: Rect) {
        f.render_widget(
            TextInput::new(&self.input)
                .prefix(PROMPT)
                .block(Block::default().borders(Borders::all())),
            rect,
        );
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            LaunchBarMsg::AppendCommand(request) => {
                self.input.handle(request);
                Ok(ComponentReturn::empty())
            }
            LaunchBarMsg::Quit => Ok(ComponentReturn::action(Action::Quit)),
            LaunchBarMsg::Esc => Ok(ComponentReturn::action(Action::Esc)),
            LaunchBarMsg::NavTo(nav) => Ok(ComponentReturn::action(Action::NavTo(nav))),
            LaunchBarMsg::SwitchWorkspace(ws) => {
                Ok(ComponentReturn::action(Action::SwitchWorkspace(ws)))
            }
            LaunchBarMsg::Error(err) => Ok(ComponentReturn::action(Action::Notification(
                Notification::error(err),
            ))),
            LaunchBarMsg::Loop => Ok(ComponentReturn::empty()),
        }
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        let ComponentEvent::Event(Event::Key(key)) = evt else {
            return Ok(vec![]);
        };

        let current = self.input.value();

        match key.code {
            KeyCode::Char('q') if current.is_empty() => Ok(vec![
                LaunchBarMsg::AppendCommand(InputRequest::InsertChar('q')).into(),
                LaunchBarMsg::AppendCommand(InputRequest::InsertChar('!')).into(),
            ]),
            KeyCode::Tab => Ok(vec![LaunchBarMsg::Loop.into()]),
            KeyCode::Enter if current == "q!" => Ok(vec![LaunchBarMsg::Quit.into()]),
            KeyCode::Enter if !current.is_empty() => {
                if let Ok(ws) = current.parse::<Workspace>() {
                    return Ok(vec![LaunchBarMsg::SwitchWorkspace(ws).into()]);
                }
                match current.parse::<Nav>() {
                    Ok(nav) => Ok(vec![LaunchBarMsg::NavTo(nav).into()]),
                    Err(err) => Ok(vec![LaunchBarMsg::Error(err.to_string()).into()]),
                }
            }
            KeyCode::Enter | KeyCode::Esc => Ok(vec![LaunchBarMsg::Esc.into()]),
            _ => Ok(to_input_request(&Event::Key(key))
                .map(|request| LaunchBarMsg::AppendCommand(request).into())
                .into_iter()
                .collect()),
        }
    }
}

impl LaunchBar {
    pub fn clear(&mut self) {
        self.input.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn key(code: KeyCode) -> ComponentEvent {
        ComponentEvent::Event(Event::Key(KeyEvent::from(code)))
    }

    fn msgs(bar: &mut LaunchBar, code: KeyCode) -> Vec<LaunchBarMsg> {
        bar.handle_event(key(code))
            .unwrap()
            .into_iter()
            .map(|m| m.take())
            .collect()
    }

    #[tokio::test]
    async fn typing_q_on_empty_bar_expands_to_quit_command() {
        let mut bar = LaunchBar::default();
        let expanded = msgs(&mut bar, KeyCode::Char('q'));
        assert!(matches!(
            expanded.as_slice(),
            [
                LaunchBarMsg::AppendCommand(InputRequest::InsertChar('q')),
                LaunchBarMsg::AppendCommand(InputRequest::InsertChar('!')),
            ]
        ));

        for m in expanded {
            bar.update(m.into()).await.unwrap();
        }
        assert_eq!(bar.input.value(), "q!");
        assert!(matches!(
            msgs(&mut bar, KeyCode::Enter).as_slice(),
            [LaunchBarMsg::Quit]
        ));
    }

    #[tokio::test]
    async fn enter_parses_navigation_and_clear_resets() {
        let mut bar = LaunchBar::default();
        for c in "assets".chars() {
            for m in msgs(&mut bar, KeyCode::Char(c)) {
                bar.update(m.into()).await.unwrap();
            }
        }
        assert_eq!(bar.input.value(), "assets");
        assert!(matches!(
            msgs(&mut bar, KeyCode::Enter).as_slice(),
            [LaunchBarMsg::NavTo(Nav::AssetsList)]
        ));

        bar.clear();
        assert_eq!(bar.input.value(), "");
        assert!(matches!(
            msgs(&mut bar, KeyCode::Enter).as_slice(),
            [LaunchBarMsg::Esc]
        ));
    }

    #[test]
    fn backspace_and_unknown_command() {
        let mut bar = LaunchBar {
            input: Input::from("nope"),
        };
        assert!(matches!(
            msgs(&mut bar, KeyCode::Backspace).as_slice(),
            [LaunchBarMsg::AppendCommand(InputRequest::DeletePrevChar)]
        ));
        assert!(matches!(
            msgs(&mut bar, KeyCode::Enter).as_slice(),
            [LaunchBarMsg::Error(_)]
        ));
    }
}
