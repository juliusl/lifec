
use crate::prelude::*;

/// Simple plugin that hosts a file and waits for a connection,
///
#[derive(Default)]
pub struct Publish;

impl Plugin for Publish {
    fn symbol() -> &'static str {
        "publish"
    }

    fn call(context: &mut ThunkContext) -> Option<AsyncContext> {
        context.task(|mut cancel_source| {
            let mut tc = context.clone();
            async move {
                tc.assign_addresses().await;

                if let Some(file_path) = tc.state().find_symbol("publish") {
                    let mut file = tokio::fs::OpenOptions::new()
                        .read(true)
                        .open(&file_path)
                        .await
                        .expect("should be able to read file");

                    match tc.enable_listener(&mut cancel_source).await {
                        Some((mut stream, remote_address)) => {
                            event!(Level::DEBUG, "Remote addr {remote_address} is connecting");
                            stream.writable().await.expect("should be writeable");

                            match tokio::io::copy(&mut file, &mut stream).await {
                                Ok(written) => {
                                    event!(Level::DEBUG, "Total bytes {written}");
                                }
                                Err(err) => {
                                    event!(Level::ERROR, "Could not write to stream {err}");
                                }
                            }
                        }
                        None => {
                            event!(
                                Level::WARN,
                                "Exiting from tcp listener before connection was accepted"
                            )
                        }
                    }
                }

                Some(tc)
            }
        })
    }
}

impl BlockObject for Publish {
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
            .require("publish")
            .optional("address")
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}

mod tests {
    use crate::prelude::*;
    
    #[derive(Default)]
    struct Test;

    impl Project for Test {
        fn interpret(_: &specs::World, _: &reality::Block) {}
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_publish() {
        use std::path::PathBuf;

        // Define an engine that calls the plugin
        let mut host = Host::load_content::<Test>(
            r#"
    ``` test
    + .engine
    : .start publish
    : .exit
    ```

    ``` publish test
    : tcp .symbol 127.0.0.1:49579

    + .runtime
    : .publish examples/io/.runmd
    ```
    "#,
        );

        // Start the engine w/ the publish plugin
        let mut dispatcher = host.prepare::<Test>();

        let engine = host.find_start("test").expect("should have a test block");
        host.start_event(engine);

        // Spawn a task to connect and read from the publish plugin
        let task = tokio::spawn(async {
            let mut context = ThunkContext::default();
            context
                .state_mut()
                .with_symbol("address", "127.0.0.1:49579");
            context.readln_stream().await
        });


        // This drives the event runtime a bit so things start up
        let mut count = 0;
        loop {
            dispatcher.dispatch(host.world());
            host.world_mut().maintain();
            count += 1;
            if count > 10 {
                break;
            }
        }

        // Wait for the read to complete
        let received = task.await.ok().expect("should have value");
        let tocompare = tokio::fs::read_to_string("examples/io/.runmd")
            .await
            .expect("should be able to read");

        assert_eq!(received.trim(), tocompare.trim());

        // Clean up nested runtime
        host.exit();
    }
}
