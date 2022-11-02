
use reality::{BlockObject, BlockProperties};
use tracing::{event, Level};

use super::{NodeCommand, Plugin, AttributeIndex};

use crate::plugins::protocol_prelude::*;

pub struct Dispatch;

impl Plugin for Dispatch {
    fn symbol() -> &'static str {
        "dispatch"
    }

    fn call(context: &mut super::ThunkContext) -> Option<super::AsyncContext> {
        context.task(|_| {
            let tc = context.clone();
            async {
                let tokio_file = |name| async move {
                    tokio::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(name)
                        .await
                        .ok()
                        .unwrap()
                };

                let mut protocol = Protocol::empty();

                protocol
                    .send_async::<NodeCommand, _, _>(
                        stream("control", tokio_file),
                        stream("rames", tokio_file),
                        stream("blob", tokio_file),
                    )
                    .await;

                Some(tc)
            }
        })
    }
}

#[derive(Default)]
pub struct Listen; 

impl Plugin for Listen {
    fn symbol() -> &'static str {
        "listen"
    }

    fn call(context: &mut super::ThunkContext) -> Option<super::AsyncContext> {
        context.task(|_| {
            let tc = context.clone();
            async {

                let listen_dir = tc.search().find_symbol("listen").expect("should have a listen symbol");
                let work_dir = tc.work_dir().expect("should have a work dir").join(listen_dir);
                let cleanup_dir = work_dir.clone();

                match tokio::fs::create_dir_all(&work_dir).await {
                    Ok(_) => {},
                    Err(err) => {
                        event!(Level::ERROR, "Could create work dir {:?}, {err}", &work_dir);
                    },
                }

                let tokio_file = |name| async move {
                    let path = work_dir.join(name);
                    match tokio::fs::OpenOptions::new()
                        .read(true)
                        .open(&path)
                        .await {
                            Ok(file) => {
                                file
                            },
                            Err(err) => {
                                panic!("Error opening file {:?} {err}", &path);
                            },
                        }
                };

                let mut protocol = Protocol::empty();

                protocol
                    .receive_async::<NodeCommand, _, _>(
                        stream("control", tokio_file.clone()),
                        stream("frames", tokio_file.clone()),
                        stream("blob", tokio_file.clone()),
                    )
                    .await;

                for command in protocol.decode::<NodeCommand>() {
                    tc.dispatch_node_command(command);
                }
                std::fs::remove_file(cleanup_dir.join("control")).ok();
                std::fs::remove_file(cleanup_dir.join("frames")).ok();
                std::fs::remove_file(cleanup_dir.join("blob")).ok();

                Some(tc)
            }
        })
    }
}

impl BlockObject for Listen {
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
            .require("listen")
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}
 