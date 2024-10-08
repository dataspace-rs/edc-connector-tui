use super::{button::ButtonMsg, row::RowMsg, text::TextFieldMsg};

#[derive(Debug)]
pub enum FormMsg {
    Local(FormLocalMsg),
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
    Button(ButtonMsg<Box<FieldMsg>>),
    Form(Box<FormLocalMsg>),
}
