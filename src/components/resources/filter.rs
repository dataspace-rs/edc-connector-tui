use edc_connector_client::types::query::Query;
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{block::Title, Block, Borders, Clear},
    Frame,
};
use tui_textarea::TextArea;

use crate::components::Component;

use super::{msg::ResourcesMsg, resource::msg::ResourceMsg};

pub struct Filter {
    query: Query,
    limit: TextArea<'static>,
    sort_field: TextArea<'static>,
    sort_order: TextArea<'static>,
    focus: Focus,
}

#[derive(PartialEq, Eq)]
enum Focus {
    Limit,
    SortField,
    SortOrder,
}

impl Filter {
    pub fn new(query: Query) -> Self {
        let mut limit = TextArea::default();
        limit.insert_str(format!("{}", query.limit()));
        Self {
            query,
            limit,
            sort_field: TextArea::default(),
            sort_order: TextArea::default(),
            focus: Focus::Limit,
        }
    }

    pub fn query(&self) -> &Query {
        &self.query
    }

    pub fn next_page(&mut self) {
        self.query = self
            .query
            .to_builder()
            .offset(self.query.offset() + self.query.limit())
            .build();
    }

    pub fn prev_page(&mut self) {
        self.query = self
            .query
            .to_builder()
            .offset(self.query.offset() - self.query.limit())
            .build();
    }

    fn popup_area(&self, area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }

    fn configure_text_area(text_area: &mut TextArea<'static>, title: &'static str, selected: bool) {
        text_area.set_block(Block::default().borders(Borders::all()).title(title));
        text_area.set_cursor_line_style(Style::default());

        if !selected {
            text_area.set_cursor_style(Style::default());
        }
    }
}

pub enum FilterMsg {}

impl Component for Filter {
    type Msg = FilterMsg;
    type Props = Query;

    fn view(&mut self, f: &mut Frame, _rect: Rect) {
        let area = f.area();

        let styled_text = Span::styled(" Filters ", Style::default().fg(Color::Red));
        let block = Block::default()
            .title(Title::from(styled_text).alignment(Alignment::Center))
            .borders(Borders::ALL);
        let area = self.popup_area(area, 30, 50);

        let content = block.inner(area);
        f.render_widget(Clear, area); //this clears out the background
        f.render_widget(block, area);

        let layouts = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(content);

        Self::configure_text_area(&mut self.limit, "Offset", self.focus == Focus::Limit);
        Self::configure_text_area(
            &mut self.sort_field,
            "Sort Field",
            self.focus == Focus::SortField,
        );
        Self::configure_text_area(
            &mut self.sort_order,
            "Sort Order",
            self.focus == Focus::SortOrder,
        );

        f.render_widget(&self.limit, layouts[0]);
        f.render_widget(&self.sort_field, layouts[1]);
        f.render_widget(&self.sort_order, layouts[2]);
    }
}
