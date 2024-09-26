use std::{collections::VecDeque, sync::Arc, time::Duration};

use crossterm::event;
use ratatui::{backend::Backend, Terminal};
use tokio::sync::Mutex;

use crate::components::{Action, ActionHandler, Component, ComponentEvent, ComponentMsg};

pub struct Runner<C: Component> {
    tick_rate: Duration,
    component: C,
}

impl<C: Component + ActionHandler<Msg = <C as Component>::Msg> + Send> Runner<C> {
    pub fn new(tick_rate: Duration, component: C) -> Self {
        Self {
            tick_rate,
            component,
        }
    }

    pub async fn run(&mut self, mut terminal: Terminal<impl Backend>) -> anyhow::Result<()> {
        terminal.clear()?;

        let mut should_quit = false;
        let async_msgs = Arc::new(Mutex::new(
            VecDeque::<ComponentMsg<<C as Component>::Msg>>::new(),
        ));
        loop {
            if should_quit {
                break;
            }
            terminal.draw(|frame| self.component.view(frame, frame.area()))?;

            let mut msgs = VecDeque::new();
            let mut guard = async_msgs.lock().await;

            while let Some(m) = guard.pop_front() {
                msgs.push_front(m);
            }
            drop(guard);

            if event::poll(self.tick_rate)? {
                let evt = event::read()?;
                let event_msgs = self
                    .component
                    .handle_event(ComponentEvent::Event(evt))?
                    .into_iter()
                    .collect::<Vec<_>>();

                for m in event_msgs {
                    msgs.push_back(m);
                }
            };

            while let Some(msg) = msgs.pop_front() {
                let actions = {
                    let ret = self.component.update(msg).await?;

                    for m in ret.msgs {
                        msgs.push_back(m);
                    }

                    for c in ret.cmds {
                        let inner_async_msg = async_msgs.clone();
                        tokio::task::spawn(async move {
                            for m in c.await.unwrap() {
                                let mut msg_guard = inner_async_msg.lock().await;
                                msg_guard.push_back(m);
                            }
                        });
                    }

                    ret.actions
                };

                for a in actions {
                    should_quit = should_quit || matches!(a, Action::Quit);
                    for m in self.component.handle_action(a)? {
                        msgs.push_back(m)
                    }
                }
            }
        }

        Ok(())
    }
}
