use atlier::system::default_start_editor_1080p;
use atlier::system::App;
use carddeck::Dealer;
use lifec::editor::EditorRuntime;

fn main() {
    default_start_editor_1080p::<EditorRuntime<Dealer>>(
        "event",
        |ui, state: &EditorRuntime<Dealer>, imnode_editor| {
            EditorRuntime::<Dealer>::show(ui, state, imnode_editor)
        },
    );
}
