use crate::components::{Action, ActionHandler, ComponentMsg};

use super::{model::AppFocus, msg::AppMsg, App};

impl ActionHandler for App {
    type Msg = AppMsg;
    fn handle_action(
        &mut self,
        action: crate::components::Action,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        match (&self.focus, action) {
            (AppFocus::LaunchBar, Action::Esc) => Ok(vec![AppMsg::HideLaunchBar.into()]),
            (_, Action::NavTo(nav)) => Ok(vec![AppMsg::RoutingMsg(nav).into()]),
            (_, Action::ChangeSheet) => Ok(vec![AppMsg::ChangeSheet.into()]),
            (_, Action::Notification(noty)) => Ok(vec![AppMsg::NontificationMsg(
                crate::components::NotificationMsg::Show(noty),
            )
            .into()]),
            _ => Ok(vec![]),
        }
    }
}
