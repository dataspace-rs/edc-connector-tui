use std::fmt::Debug;

use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{block::Title, Block, Borders, Row, Table, TableState},
    Frame,
};
pub mod msg;

use crate::types::info::InfoSheet;

use self::msg::{TableLocalMsg, TableMsg};

use super::{Component, ComponentEvent, ComponentMsg, ComponentReturn};

pub type OnSelect<T, M> = Box<dyn Fn(&T) -> M + Send + Sync>;

pub struct UiTable<T: TableEntry, M> {
    name: String,
    pub elements: Vec<T>,
    table_state: TableState,
    on_select: Option<OnSelect<T, M>>,
    show_block: bool,
}

impl<T: TableEntry + Debug, M> Debug for UiTable<T, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiTable")
            .field("name", &self.name)
            .field("elements", &self.elements)
            .field("table_state", &self.table_state)
            .finish()
    }
}

impl<T: TableEntry, M> Default for UiTable<T, M> {
    fn default() -> Self {
        Self {
            name: String::new(),
            elements: vec![],
            table_state: TableState::default().with_selected(0),
            on_select: None,
            show_block: false,
        }
    }
}

pub trait TableEntry {
    fn row(&self) -> Row;
    fn headers() -> Row<'static>;
}

#[async_trait::async_trait]
impl<T: TableEntry + Send, M: Send + 'static> Component for UiTable<T, M> {
    type Msg = TableMsg<M>;
    type Props = ();

    fn view(&mut self, f: &mut Frame, area: Rect) {
        let rows = self
            .elements
            .iter()
            .map(TableEntry::row)
            .collect::<Vec<_>>();

        let mut table = Table::default()
            .rows(rows)
            .header(T::headers())
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));

        if self.show_block {
            let styled_text =
                Span::styled(format!(" {} ", self.name), Style::default().fg(Color::Red));
            let block = Block::default()
                .title(Title::from(styled_text).alignment(Alignment::Center))
                .borders(Borders::ALL);
            table = table.block(block)
        }

        f.render_stateful_widget(table, area, &mut self.table_state);
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            TableMsg::Local(TableLocalMsg::MoveDown) => self.move_down(),
            TableMsg::Local(TableLocalMsg::MoveUp) => self.move_up(),
            TableMsg::Outer(_) => {}
        };

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

impl<T: TableEntry, M> UiTable<T, M> {
    pub fn new(name: String) -> Self {
        Self {
            name,
            elements: vec![],
            table_state: TableState::default().with_selected(0),
            on_select: None,
            show_block: false,
        }
    }

    pub fn info_sheet(&self) -> InfoSheet {
        InfoSheet::default()
            .key_binding("<j/down>", "Down")
            .key_binding("<k/down>", "Up")
    }

    pub fn with_elements(name: String, elements: Vec<T>, show_block: bool) -> Self {
        Self {
            name,
            elements,
            table_state: TableState::default().with_selected(0),
            on_select: None,
            show_block,
        }
    }
    pub fn on_select(mut self, cb: impl Fn(&T) -> M + Send + Sync + 'static) -> Self {
        self.on_select = Some(Box::new(cb));
        self
    }

    fn handle_key(&self, key: KeyEvent) -> Vec<ComponentMsg<TableMsg<M>>> {
        match key.code {
            KeyCode::Enter => {
                if let Some(cb) = self.on_select.as_ref() {
                    if let Some(idx) = self.table_state.selected() {
                        if let Some(element) = self.elements.get(idx) {
                            vec![ComponentMsg(TableMsg::Outer(cb(element)))]
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                vec![(ComponentMsg(TableLocalMsg::MoveDown.into()))]
            }
            KeyCode::Char('k') | KeyCode::Up => vec![(ComponentMsg(TableLocalMsg::MoveUp.into()))],
            _ => vec![],
        }
    }

    fn move_up(&mut self) {
        let new_pos = match self.table_state.selected() {
            Some(0) => self.elements.len() - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.table_state.select(Some(new_pos))
    }

    fn move_down(&mut self) {
        let new_pos = match self.table_state.selected() {
            Some(i) if i == self.elements.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.table_state.select(Some(new_pos))
    }
}
