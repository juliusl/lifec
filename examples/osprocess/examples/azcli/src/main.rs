use lifec::{Event, Runtime, App};
use editor::RuntimeEditor;
use osprocess::Process;

fn main() {
    let mut runtime = Runtime::<Process>::default()
        .with_call_args("az::vm::create", |s, _| {
            let flags = s.parse_flags();
            let variables = s.parse_variables();

            println!("{:?}", flags);
            println!("{:?}", variables);

            (s.get_state(), Event::exit().to_string())
        });

    runtime.on("{ setup;; }")
        .dispatch("echo", "{ make_vm;; }");

    runtime.on("{ make_vm;; }").call("az::vm::create").args(&[
        "$RESOURCE_GROUP=dev-rg;",
        "--resource-group",
        "$RESOURCE_GROUP",
        "--name",
        "$VM_NAME",
        "--image",
        "$IMAGE",
        "--location",
        "$LOCATION",
        "--admin-username",
        "$ADMIN_USERNAME",
        "--public-ip-sku",
        "Standard",
        "--public-ip-address-dns-name",
        "$VM_NAME-$DEV_ID",
        "--generate-ssh-keys",
        "--size",
        "Standard_D4s_v3",
        "--nic-delete-option",
        "delete",
        "--os-disk-delete-option",
        "delete",
        "--custom-data",
        "user-data",
        "--tags",
        "'dev_id=$DEV_ID orgv=342 source_driver=gcm'",
    ]);

    RuntimeEditor::start_editor(Some(RuntimeEditor::from(runtime)));
}
