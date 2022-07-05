use crate::plugins::Engine;

pub struct Exit;

impl Engine for Exit {
    fn event_name() -> &'static str {
        "exit"
    }
}