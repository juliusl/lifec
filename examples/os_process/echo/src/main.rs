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

    runtime
        .on("{ setup;; }")
        .dispatch("echo", "{ after_echo;; }")
        .args(&["--o", "hello world"]);

    runtime.on("{ after_echo;; }").call("print_results");

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
        ],
        |_| {},
        |_| {},
        move |w, ui| {
            let file_editor = &mut file_editor;
            file_editor.extend_app_world(w, ui);

            let attr_editor = &mut attr_editor;
            attr_editor.extend_app_world(w, ui);

            let event_editor = &mut event_editor;
            event_editor.extend_app_world(w, ui);
        },
    );
}
