use lifec::plugins::demos::NodeDemo;
use lifec::{editor::*, AttributeGraph, Runtime};

fn main() {
    if let Some(file) = AttributeGraph::load_from_file(".runmd") {
        open(
            "demo",
            RuntimeEditor::new(Runtime::from(file)),
            NodeDemo::default(),
        );
    }
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
