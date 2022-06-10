use lifec::plugins::{Process, Println, WriteFiles};
use lifec::{editor::*, AttributeGraph, Runtime, RuntimeDispatcher};

fn main() {
    let mut node_editor = NodeEditor::<Process>::new();
    node_editor.with_thunk::<Process>();
    node_editor.with_thunk::<Println>();
    node_editor.with_thunk::<WriteFiles>();

    let mut cargo_build = Process::default();
    cargo_build
        .as_mut()
        .from_file(".runmd")
        .expect("could not load state");

    let mut runtime = Runtime::from(&mut cargo_build);
    let process_section = Section::new(
        <Process as App>::name(),
        AttributeGraph::default(),
        <Process as SectionExtension<Process>>::show_extension,
        cargo_build,
    )
    .enable_app_systems();

    let mut cargo_build = Process::default();
    cargo_build
        .as_mut()
        .from_file(".runmd")
        .expect("could not load state");

    open_editor_with(
        "Demo",
        runtime.parse_event("{ setup;; }"),
        vec![process_section],
        |w| {
            NodeEditor::<Process>::configure_app_world(w);
        },
        |_| {},
        move |w, ui| {
            // ui.show_demo_window(&mut true);
            let node_editor = &mut node_editor;
            node_editor.extend_app_world(w, ui);

            cargo_build.as_mut().edit_attr_table(ui);
        },
    );
}


// ui.same_line();
            // if ui.button("Compress state") {
            //     use compression::prelude::*;
            //     match std::fs::read(format!("{}.json", "projects")) {
            //         Ok(serialized) => {
            //             let compressed = serialized
            //                 .encode(&mut BZip2Encoder::new(9), Action::Finish)
            //                 .collect::<Result<Vec<_>, _>>()
            //                 .unwrap();

            //             if let Some(_) = std::fs::write("projects.json.bzip2", compressed).ok() {
            //                 println!("compressed");
            //             }
            //         }
            //         Err(_) => {}
            //     }
            // }

            // ui.same_line();
            // if ui.button("Decompress state") {
            //     use compression::prelude::*;
            //     match std::fs::read(format!("{}.json.bzip2", "projects")) {
            //         Ok(compressed) => {
            //             let decompressed = compressed
            //                 .decode(&mut BZip2Decoder::new())
            //                 .collect::<Result<Vec<_>, _>>()
            //                 .unwrap();

            //             if let Some(_) =
            //                 std::fs::write("projects.json.bzip2.json", decompressed).ok()
            //             {
            //                 println!("decompressed");
            //             }
            //         }
            //         Err(_) => {}
            //     }
            // }