use lifec::plugins::{Process, Thunk};
use lifec::{editor::*, AttributeGraph, Runtime, RuntimeDispatcher, RuntimeState};

fn main() {
    let mut node_editor = NodeEditor::<Process>::new();
    node_editor.with_thunk::<Process>();
    node_editor.with_thunk::<Println>();
    node_editor.with_thunk::<WriteFiles>();

    let mut cargo_build = Process::default();

    let initial_setup = r#"
    #
    # Example of configuring node editor for Process type 
    # 
    define command      node
    define subcommands  node
    #
    # Set initial values for edit fields
    #
    edit   command::node         command .TEXT cargo
    edit   subcommands::node subcommands .TEXT build
    #
    # Enable node editor for section (default closed)
    #
    add enable_node_editor .BOOL false
    "#;

    cargo_build
        .as_mut()
        .batch_mut(initial_setup)
        .expect("should be able to configure process state");

    let mut runtime = Runtime::from(&mut cargo_build);
    let process_section = Section::new(
        <Process as App>::name(),
        AttributeGraph::default(),
        <Process as SectionExtension<Process>>::show_extension,
        cargo_build,
    )
    .enable_app_systems();

    open_editor_with(
        "OS Process",
        runtime.parse_event("{ setup;; }"),
        vec![process_section],
        |w| {
            EventEditor::configure_app_world(w);
            AttributeEditor::configure_app_world(w);
            NodeEditor::<Process>::configure_app_world(w);
        },
        |_| {},
        move |w, ui| {
            // ui.show_demo_window(&mut true);

            let node_editor = &mut node_editor;
            node_editor.extend_app_world(w, ui);
        },
    );
}

struct Println;

impl Thunk for Println {
    fn symbol() -> &'static str {
        "println"
    }

    fn call_with_context(context: &mut lifec::plugins::ThunkContext) {
        context
            .as_ref()
            .iter_attributes()
            .map(|a| (a.name(), a.value()))
            .for_each(|(name, value)| {
                println!("{}: {}", name, value);
            });

        let dispatcher = context.as_mut();
        dispatcher
            .dispatch_mut("define println returns")
            .expect("should be able to define returns symbol");
        dispatcher
            .dispatch_mut("edit println::returns println::returns .BOOL true")
            .expect("should be able to edit the transient value");
    }
}

struct WriteFiles;

impl Thunk for WriteFiles {
    fn symbol() -> &'static str {
        "write_files"
    }

    fn call_with_context(context: &mut lifec::plugins::ThunkContext) {
        context
            .as_ref()
            .iter_attributes()
            .map(|a| (a.name(), a.value()))
            .for_each(|(file_name, value)| {
                if let Value::BinaryVector(content) = value {
                    if let None = std::fs::write(file_name.replace("::", "."), content).ok() {
                        eprintln!("did not write test.out");
                    }
                } else {
                    eprintln!("skipping write file for: {:?}", (file_name, value));
                }
            });
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