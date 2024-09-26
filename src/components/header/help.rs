use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Styled},
    text::{Line, Span},
    widgets::{Block, List},
    Frame,
};

use crate::{components::StatelessComponent, types::info::InfoSheet};

#[derive(Default)]
pub struct InfoComponent {}

#[async_trait::async_trait]
impl StatelessComponent for InfoComponent {
    type Props = InfoSheet;

    fn view(&mut self, props: &Self::Props, f: &mut Frame, rect: Rect) {
        let layout =
            Layout::horizontal(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
                .split(rect);

        self.view_info(props, f, layout[0]);
        self.view_key_bindings(props, f, layout[1])
    }
}

impl InfoComponent {
    fn view_info(&self, props: &InfoSheet, f: &mut Frame, rect: Rect) {
        let max = props
            .iter_info()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(0);

        let list = props
            .iter_info()
            .map(|(name, value)| {
                let padding = max + 2 - name.len();
                Line::from(vec![
                    name.to_string()
                        .set_style(Style::default().fg(Color::Yellow)),
                    Span::raw(format!("{:<padding$}", ":"))
                        .set_style(Style::default().fg(Color::Yellow)),
                    Span::raw(value),
                ])
            })
            .collect::<List>()
            .block(Block::default());

        f.render_widget(list, rect)
    }

    fn view_key_bindings(&self, props: &InfoSheet, f: &mut Frame, rect: Rect) {
        let layout = Layout::horizontal(vec![
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(rect);
        let max = props
            .iter_key_bindings()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(0);

        let lines = props
            .iter_key_bindings()
            .map(|(name, value)| {
                let padding = max + 2 - name.len();
                Line::from(vec![
                    name.to_string()
                        .set_style(Style::default().fg(Color::Magenta)),
                    Span::raw(format!("{:<padding$}", ""))
                        .set_style(Style::default().fg(Color::Magenta)),
                    Span::raw(value),
                ])
            })
            .collect::<Vec<Line>>();

        let lists = lines
            .chunks(5)
            .map(|chunks| chunks.iter().map(Clone::clone).collect::<List>());

        for (idx, list) in lists.enumerate() {
            f.render_widget(list, layout[idx])
        }
    }
}
