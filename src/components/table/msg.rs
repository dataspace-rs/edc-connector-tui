#[derive(Debug)]
pub enum TableMsg<T> {
    Local(TableLocalMsg),
    Outer(T),
}

#[derive(Debug)]
pub enum TableLocalMsg {
    MoveUp,
    MoveDown,
}

impl<T> From<TableLocalMsg> for TableMsg<T> {
    fn from(value: TableLocalMsg) -> Self {
        TableMsg::Local(value)
    }
}
