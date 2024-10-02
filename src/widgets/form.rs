use crossterm::event::{Event, KeyEvent};
use msg::FormMsg;
use ratatui::{layout::Rect, Frame};

use crate::components::{Component, ComponentEvent, ComponentMsg, ComponentReturn};
pub mod msg;

pub struct Form {}

impl Default for Form {
    fn default() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Component for Form {
    type Msg = FormMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, area: Rect) {}

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        Ok(ComponentReturn::empty())
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        match evt {
            ComponentEvent::Event(Event::Key(key)) => Ok(self.handle_key(key)),
            _ => Ok(vec![]),
        }
    }
}

impl Form {
    pub fn new(name: String) -> Self {
        Self {}
    }

    fn handle_key(&self, key: KeyEvent) -> Vec<ComponentMsg<FormMsg>> {
        vec![]
    }
}
