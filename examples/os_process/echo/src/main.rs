use lifec::{editor::*, Runtime};
use lifec::plugins::{Process, Project};

fn main() {
    let mut runtime = Runtime::<Process>::default().with_call("print_results", |s, _| {
        let output = String::from_utf8(s.stdout.clone()).ok();

        if let Some(output) = output {
            println!("{}", output);
        }

        (s.clone(), "{ exit;; }".to_string())
    });

    let runtime = &mut runtime;
    runtime
        .on("{ setup;; }")
        .dispatch("echo", "{ after_echo;; }")
        .args(&["--o", "hello world"]);

    runtime
        .on("{ after_echo;; }")
        .call("print_results");

    let mut node_editor = NodeEditor::new();
    let mut event_editor = EventEditor::new();
    let mut attr_editor = AttributeEditor::new();
    let mut project = Project::default();
    open_editor_with(
        "OS Process",
        runtime.parse_event("{ setup;; }"),
        vec![Section::new(
            <Process as App>::name(),
            <Process as SectionExtension<Process>>::show_extension,
            Process::default(),
        )
        .with_text("command", "")
        .enable_app_systems()
        ],
        |w| {
            EventEditor::configure_app_world(w);
            AttributeEditor::configure_app_world(w);
            NodeEditor::configure_app_world(w);
        },
        |_| {
        },
        move |w, ui| {
            let project = &mut project;
            project.extend_app_world(w, ui);

            let attr_editor  = &mut attr_editor;
            attr_editor.extend_app_world(w, ui);

            let event_editor = &mut event_editor;
            event_editor.extend_app_world(w, ui);

            let node_editor = &mut node_editor;
            node_editor.extend_app_world(w, ui);

            ui.same_line();
            if ui.button("Compress state") { 
                use compression::prelude::*;
                match std::fs::read(format!("{}.json", "projects")) {
                    Ok(serialized) => { 
                        let compressed = serialized
                            .encode(&mut BZip2Encoder::new(9), Action::Finish)
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap();
                        
                        if let Some(_) = std::fs::write("projects.json.bzip2", compressed).ok() {
                            println!("compressed");
                        }
                    }
                    Err(_) => {}
                }
            }
            
            ui.same_line();
            if ui.button("Decompress state") { 
                use compression::prelude::*;
                match std::fs::read(format!("{}.json.bzip2", "projects")) {
                    Ok(compressed) => { 
                        let decompressed = compressed
                            .decode(&mut BZip2Decoder::new())
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap();
                        
                        if let Some(_) = std::fs::write("projects.json.bzip2.json", decompressed).ok() {
                            println!("decompressed");
                        }
                    }
                    Err(_) => {}
                }
            }
        },
    );
}
