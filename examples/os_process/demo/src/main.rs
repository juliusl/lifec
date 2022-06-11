use std::sync::Once;

use lifec::plugins::{
    self, Println, Process, NodeContext, RenderComponent, RenderNodeInput, RenderNodeAttribute, RenderNodeOutput,
    WriteFiles, Render,
};
use lifec::{editor::*, AttributeGraph, Runtime, RuntimeDispatcher};
use lifec::plugins::Node;

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
        cargo_build.as_ref().clone(),
        |s, ui| {
            s.state.show_editor(ui);
        },
        cargo_build,
    )
    .enable_app_systems();

    let mut cargo_build = Process::default();
    cargo_build
        .as_mut()
        .from_file(".runmd")
        .expect("could not load state");

    let node = Node::new();

    open_editor_with(
        "Demo",
        runtime.parse_event("{ setup;; }"),
        vec![process_section],
        move |w| {
            NodeEditor::<Process>::configure_app_world(w);

            w.register::<RenderComponent>();
            w.register::<NodeContext>();
            w.register::<RenderNodeAttribute>();
            w.register::<RenderNodeOutput>();
            w.register::<RenderNodeInput>();

            let mut demo = AttributeGraph::default();
            if demo.from_file("demo.runmd").is_ok() {
                w.create_entity()
                    .with(demo)
                    .with(RenderComponent(|g, ui| {
                        g.edit_attr_table(ui);
                    }))
                    .with(NodeContext::default())
                    .with(RenderNodeAttribute::new(|g, ui| {
                        ui.text("node attr");
                    }))
                    .with(RenderNodeOutput::new(|g, ui| {
                        ui.text("node output");
                    }))
                    .build();
            }
        },
        |_| {},
         move |w, ui| {
            // ui.show_demo_window(&mut true);

            let node_editor = &mut node_editor;
            node_editor.extend_app_world(w, ui);

            // render.run(w);

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
