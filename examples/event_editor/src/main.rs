use atlier::system::default_start_editor_1080p;
use carddeck::Dealer;
use editor::{App, RuntimeEditor};

fn main() {
    default_start_editor_1080p::<RuntimeEditor<Dealer>>(
        "event",
        |ui, state: &RuntimeEditor<Dealer>, imnode_editor| {
            RuntimeEditor::<Dealer>::show(ui, state, imnode_editor)
        },
    );
}
