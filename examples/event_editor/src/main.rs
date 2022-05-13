use atlier::system::default_start_editor_1080p;
use atlier::system::App;
use carddeck::Dealer;
use lifec::editor::RuntimeEditor;

fn main() {
    default_start_editor_1080p::<RuntimeEditor<Dealer>>(
        "event",
        |ui, state: &RuntimeEditor<Dealer>, imnode_editor| {
            RuntimeEditor::<Dealer>::show(ui, state, imnode_editor)
        },
    );
}
