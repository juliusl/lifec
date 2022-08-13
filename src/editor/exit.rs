use crate::plugins::Engine;

pub struct Exit;

impl Engine for Exit {
    fn event_symbol() -> &'static str {
        "exit"
    }
}