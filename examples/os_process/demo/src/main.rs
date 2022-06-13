use lifec::plugins::{demos::WriteFilesDemo, Node};
use lifec::{editor::*};

fn main() {
    // let mut node_editor = NodeEditor::<Process>::new();
    // node_editor.with_thunk::<Process>();
    // node_editor.with_thunk::<Println>();
    // node_editor.with_thunk::<WriteFiles>();

    // let mut cargo_build = Process::default();
    // cargo_build
    //     .as_mut()
    //     .from_file(".runmd")
    //     .expect("could not load state");

    open(
        "demo",
        move |world, dispatcher| {
            Node::configure_app_systems(dispatcher);
            Node::configure_app_world(world);
            WriteFilesDemo::configure_app_world(world);
            WriteFilesDemo::configure_app_systems(dispatcher);
        },
        WriteFilesDemo {},
    )
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
