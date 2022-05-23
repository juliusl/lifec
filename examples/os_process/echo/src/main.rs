use lifec::{editor::*, editor::FileEditor, Runtime};
use lifec::plugins::Process;

fn main() {
    let mut runtime = Runtime::<Process>::default().with_call("print_results", |s, _| {
        let output = String::from_utf8(s.stdout.clone()).ok();

        if let Some(output) = output {
            println!("{}", output);
        }

        (s.clone(), "{ exit;; }".to_string())
    });

    // let mut runtime = runtime
    //     .with_attribute(Attribute::new(1, "node::on", Value::TextBuffer("".to_string())))
    //     .with_attribute(Attribute::new(1, "node::call", Value::TextBuffer("".to_string())))
    //     .with_attribute(Attribute::new(1, "node::dispatch", Value::TextBuffer("".to_string())))
    //     .with_attribute(Attribute::new(1, "enable node editor", Value::Bool(true)))
    //     .with_attribute(Attribute::new(0, "enable node editor", Value::Bool(true)))
    //     .with_attribute(Attribute::new(0, "node::on", Value::TextBuffer("".to_string())))
    //     .with_attribute(Attribute::new(0, "node::call", Value::TextBuffer("".to_string())))
    //     .with_attribute(Attribute::new(0, "node::dispatch", Value::TextBuffer("".to_string())));

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
    let mut file_editor = FileEditor::new();
    let mut attr_editor = AttributeEditor::new();
    open_editor_with(
        "OS Process",
        runtime.parse_event("{ setup;; }"),
        vec![Section::new(
            <Process as App>::name(),
            <Process as SectionExtension<Process>>::show_extension,
            Process::default(),
        )
        .with_text("command", "")
        .with_symbol("file::name::echo.json")
        .with_symbol("file::name::echo.toml")
        .enable_app_systems()
        ],
        |w| {
            AttributeEditor::configure_app_world(w);
            NodeEditor::configure_app_world(w);
        },
        |_| {},
        move |w, ui| {
            let file_editor = &mut file_editor;
            file_editor.extend_app_world(w, ui);

            let attr_editor = &mut attr_editor;
            attr_editor.extend_app_world(w, ui);

            let event_editor = &mut event_editor;
            event_editor.extend_app_world(w, ui);

            let node_editor = &mut node_editor;
            node_editor.extend_app_world(w, ui);
        },
    );
}