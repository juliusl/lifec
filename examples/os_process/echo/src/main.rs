use lifec::plugins::{Process, Project};
use lifec::{editor::*, Runtime};

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

    runtime.on("{ after_echo;; }").call("print_results");

    let mut runtime = runtime.ensure_call("print_results", None, None);

    let mut node_editor = NodeEditor::new();
    node_editor.add_thunk("println", |v| {
        let echo = format!("{:?}", v);

        println!("{}", echo);

        v.insert(format!("thunk::{}::output::", "println"), Value::Bool(true));
    });

    node_editor.add_thunk("broadcast", |v| {
        if let Some((_, value)) = v.clone().iter().filter(|(_, v)| match v {
            Value::Empty => false,
            _ => true
        }).last() {
            v.insert(format!("thunk::{}::output::", "broadcast"), value.clone());
        }
    });

    node_editor.add_thunk("print_results", |inputs| {
        let mut runtime = Runtime::<Process>::default().with_call("print_results", |s, _| {
            let output = String::from_utf8(s.stdout.clone()).ok();
    
            if let Some(output) = output {
                println!("{}", output);
            }
    
            (s.clone(), "{ exit;; }".to_string())
        });

        for (name, value) in inputs.iter() {
            runtime.attribute(Attribute::new(0, name, value.clone()));
        }

        if let Some(state) = runtime.after_call("print_results", None, None).current() {
            inputs.insert("output".to_string(), Value::TextBuffer(format!("{}", state)));
        }
    });

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
        .with_bool("enable node editor", true)
        .with_empty("node::name")
        .with_empty("node::other name")
        .with_text("node::cool name", "julius")
        .with_text("node::other cool name", "liu")
        .enable_app_systems()],
        |w| {
            EventEditor::configure_app_world(w);
            AttributeEditor::configure_app_world(w);
            NodeEditor::configure_app_world(w);
        },
        |_| {},
        move |w, ui| {

            let project = &mut project;
            project.extend_app_world(w, ui);

            let attr_editor = &mut attr_editor;
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

                        if let Some(_) =
                            std::fs::write("projects.json.bzip2.json", decompressed).ok()
                        {
                            println!("decompressed");
                        }
                    }
                    Err(_) => {}
                }
            }
        },
    );
}
