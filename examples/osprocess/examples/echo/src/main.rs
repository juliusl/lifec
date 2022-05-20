use lifec::editor::{open_editor_with, App, Section, SectionExtension};
use lifec::Runtime;
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

    open_editor_with(
        "OS Process",
        runtime.parse_event("{ setup;; }"),
        vec![Section::new(
            <Process as App>::name(),
            <Process as SectionExtension<Process>>::show_extension,
            Process::default(),
        )
        .with_text("command", "")
        .with_text("arg::--help", "")],
        |_| {},
        |_| {},
        |_, _| {},
    );
}
