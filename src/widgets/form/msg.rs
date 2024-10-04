use super::text::TextFieldMsg;

#[derive(Debug)]
pub enum FormMsg {
    Local(FormLocalMsg),
}

#[derive(Debug)]
pub enum FormLocalMsg {
    MoveDown,
    MoveUp,
    FieldMsg(FieldMsg),
}

#[derive(Debug)]
pub enum FieldMsg {
    Text(TextFieldMsg),
}
