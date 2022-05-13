use lifec::{Runtime};
use lifec::editor::{App, RuntimeEditor};
use osprocess::Process;

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

    RuntimeEditor::start_editor(Some(RuntimeEditor::from(runtime.parse_event("{ setup;; }"))));
}
