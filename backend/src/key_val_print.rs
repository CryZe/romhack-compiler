pub enum MessageKind {
    Warning,
    Error,
}

pub trait KeyValPrint {
    fn print(&self, kind: Option<MessageKind>, key: &str, val: &str);
}

pub struct DontPrint;

impl KeyValPrint for DontPrint {
    fn print(&self, _kind: Option<MessageKind>, _key: &str, _val: &str) {}
}
