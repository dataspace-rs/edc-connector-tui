use super::text::TextFieldMsg;

#[derive(Debug)]
pub enum FormMsg {
    Local(FormLocalMsg),
}

#[derive(Debug)]
pub enum FormLocalMsg {
    MoveDown,
    MoveUp,
}

pub enum FieldMsg {
    Text(TextFieldMsg),
}
