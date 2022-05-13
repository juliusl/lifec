
use carddeck::Dealer;
use lifec::editor::{App, RuntimeEditor};

fn main() {
    RuntimeEditor::start_editor(
        Some(
            RuntimeEditor::from(
                RuntimeEditor::<Dealer>::default())));

}
