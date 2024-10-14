use super::{row::RowMsg, text::TextFieldMsg};

#[derive(Debug)]
pub enum FormMsg<M> {
    Local(FormLocalMsg),
    Outer(M),
}

#[derive(Debug)]
pub enum FormLocalMsg {
    MoveDown,
    MoveUp,
    Submit,
    FieldMsg(FieldMsg),
}

#[derive(Debug)]
pub enum FieldMsg {
    Text(TextFieldMsg),
    Row(RowMsg),
}
