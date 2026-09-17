use super::{row::RowMsg, select::SelectFieldMsg, text::TextFieldMsg};

#[derive(Debug)]
pub enum FormMsg<M> {
    Local(FormLocalMsg),
    Outer(M),
}

#[derive(Debug)]
pub enum FormLocalMsg {
    /// Focus the row below (or the confirm button).
    MoveDown,
    /// Focus the row above.
    MoveUp,
    /// Focus the next field, walking through the fields of a row first.
    Next,
    /// Focus the previous field, walking through the fields of a row first.
    Prev,
    Submit,
    FieldMsg(FieldMsg),
}

#[derive(Debug)]
pub enum FieldMsg {
    Text(TextFieldMsg),
    Row(RowMsg),
    Select(SelectFieldMsg),
}
