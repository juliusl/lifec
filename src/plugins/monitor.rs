use reality::{BlockObject, BlockProperties};
use tracing::{event, Level};

use crate::engine::Performance;
use crate::plugins::protocol_prelude::*;

use super::{Plugin, AttributeIndex};

/// Plugin to read performance data from protocol,
/// 
#[derive(Default)]
pub struct Monitor;

impl Plugin for Monitor {
    fn symbol() -> &'static str {
        "monitor"
    }

    fn call(context: &mut super::ThunkContext) -> Option<super::AsyncContext> {
        context.task(|_| {
            let tc = context.clone();
            async {
                let listen_dir = tc
                    .search()
                    .find_symbol("monitor")
                    .expect("should have a listen symbol");
                let work_dir = tc
                    .work_dir()
                    .expect("should have a work dir")
                    .join(listen_dir)
                    .join("performance");
                let cleanup_dir = work_dir.clone();

                match tokio::fs::create_dir_all(&work_dir).await {
                    Ok(_) => {}
                    Err(err) => {
                        event!(Level::ERROR, "Could create work dir {:?}, {err}", &work_dir);
                    }
                }

                let tokio_file = |name| async move {
                    let path = work_dir.join(name);
                    match tokio::fs::OpenOptions::new().read(true).open(&path).await {
                        Ok(file) => file,
                        Err(err) => {
                            panic!("Error opening file {:?} {err}", &path);
                        }
                    }
                };

                let mut protocol = Protocol::empty();

                protocol
                    .receive_async::<Performance, _, _>(
                        stream("control", tokio_file.clone()),
                        stream("frames", tokio_file.clone()),
                        stream("blob", tokio_file.clone()),
                    )
                    .await;

                for performance in protocol.decode::<Performance>() {
                    // TODO
                    tokio::fs::write("test_out", format!("{:#?}", performance)).await.ok();
                }
                tokio::fs::remove_file(cleanup_dir.join("control")).await.ok();
                tokio::fs::remove_file(cleanup_dir.join("frames")).await.ok();
                tokio::fs::remove_file(cleanup_dir.join("blob")).await.ok();

                Some(tc)
            }
        })
    }
}

impl BlockObject for Monitor {
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
            .require("monitor")
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}