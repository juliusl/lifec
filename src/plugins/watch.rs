use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use atlier::system::Value;
use logos::Logos;
use notify::{
    recommended_watcher, Config, ErrorKind, Event, EventHandler, PollWatcher, RecommendedWatcher,
    Watcher,
};
use reality::{BlockObject, BlockProperties};
use tokio::select;
use tracing::{event, Level};

use crate::{AttributeIndex, Plugin};

use super::TimerSettings;

/// This plugin can watch for a file event, and then return
///
/// This plugin includes additional custom events that can be used to configure the event to look for.
///
/// # Example
/// ```runmd
/// + .runtime
/// : .watch    main.rs
///
/// # Configure what events to watch for. Corresponds with notify's event kind hierarchy,
/// # Defaults to any if none of these attributes are used,
///
/// # TODO : .access <leave empty or read | open | close>
/// # TODO : .create <leave empty or file | folder>
/// # TODO : .modify <leave empty or data | data size | data content | metadata | name>
/// # TODO : .remove <leave empty or file | folder>
///
/// # Optional `notify` specific attributes
/// : .poll_interval 1 sec
/// : .compare_contents
/// ```
///
/// # TODO
/// - Support 3rd tier of enum types,
///
pub struct Watch;

impl Plugin for Watch {
    fn symbol() -> &'static str {
        "watch"
    }

    fn description() -> &'static str {
        "Watches for a file event and returns. Wrapper over notify crate."
    }

    fn caveats() -> &'static str {
        "`notify` is mostly cross plat, but might run into issues on emulated environments"
    }

    fn call(context: &crate::ThunkContext) -> Option<crate::AsyncContext> {
        context.task(|cancel_source| {
            let mut tc = context.clone();
            async {
                // TODO -- Should I use a sync::watch instead?
                let (tx, mut rx) = tokio::sync::mpsc::channel(1);

                let mut config =
                    Config::default().with_compare_contents(tc.is_enabled("compare_contents"));

                if let Some(duration) = tc
                    .state()
                    .find_float("poll_interval")
                    .and_then(|f| Some(Duration::from_secs_f32(f)))
                {
                    config = config.with_poll_interval(duration);
                }

                let event_handler = move |e| match e {
                    Ok(event) => match event {
                        Event { kind, paths, .. } => {
                            // match kind {
                            //     notify::EventKind::Any => todo!(),
                            //     notify::EventKind::Access(k) => {
                            //         match k {
                            //             notify::event::AccessKind::Any => todo!(),
                            //             notify::event::AccessKind::Read => todo!(),
                            //             notify::event::AccessKind::Open(_) => todo!(),
                            //             notify::event::AccessKind::Close(_) => todo!(),
                            //             notify::event::AccessKind::Other => todo!(),
                            //         }
                            //     },
                            //     notify::EventKind::Create(c) => {
                            //         match c {
                            //             notify::event::CreateKind::Any => todo!(),
                            //             notify::event::CreateKind::File => todo!(),
                            //             notify::event::CreateKind::Folder => todo!(),
                            //             notify::event::CreateKind::Other => todo!(),
                            //         }
                            //     },
                            //     notify::EventKind::Modify(m) => {
                            //         match m {
                            //             notify::event::ModifyKind::Any => todo!(),
                            //             notify::event::ModifyKind::Data(_) => todo!(),
                            //             notify::event::ModifyKind::Metadata(_) => todo!(),
                            //             notify::event::ModifyKind::Name(_) => todo!(),
                            //             notify::event::ModifyKind::Other => todo!(),
                            //         }
                            //     },
                            //     notify::EventKind::Remove(r) => {
                            //         match r {
                            //             notify::event::RemoveKind::Any => todo!(),
                            //             notify::event::RemoveKind::File => todo!(),
                            //             notify::event::RemoveKind::Folder => todo!(),
                            //             notify::event::RemoveKind::Other => todo!(),
                            //         }
                            //     },
                            //     notify::EventKind::Other => todo!(),
                            // }

                            event!(Level::DEBUG, "File event found, {:?}", kind);
                            match tx.try_send(paths) {
                                Ok(_) => {
                                    event!(Level::TRACE, "Sent watch event");
                                }
                                Err(err) => {
                                    event!(Level::ERROR, "Could not propagate watch event {err}");
                                }
                            }
                        }
                    },
                    Err(err) => {
                        event!(
                            Level::ERROR,
                            "Event handler for file watcher received an error {err}"
                        );
                    }
                };

                let file = tc
                    .state()
                    .find_symbol("watch")
                    .expect("should have a file path value");

                // Creating a reference so the watcher doesn't get dropped while we wait
                // for the event that we're looking for
                let _watcher = match Self::watch(
                    event_handler,
                    config,
                    &file,
                    tc.is_enabled("use_fallback"),
                ) {
                    (Ok(_), watcher) => {
                        event!(Level::INFO, "Started listening to file, {file}");
                        watcher
                    }
                    (Err(err), watcher) => {
                        event!(Level::ERROR, "Could not watch file {file}, {err}");
                        watcher
                    }
                };

                select! {
                    paths = rx.recv() => {
                        match paths {
                            Some(paths) => {
                                for path in paths {
                                    tc.state_mut()
                                        .with_symbol(
                                            "path",
                                            path.to_str().expect("should be a string").to_string()
                                        );
                                }
                            },
                            None => {
                                event!(Level::ERROR, "Did not receive any paths");
                            },
                        }
                    },
                    _ = cancel_source => {
                        event!(Level::WARN, "Watch plugin is being cancelled");
                    }
                }

                Some(tc)
            }
        })
    }

    fn compile(parser: &mut reality::AttributeParser) {
        // Check if there would be an error using the recommended watcher
        match notify::RecommendedWatcher::new(|_| {}, Config::default()) {
            Ok(_) => {}
            Err(err) => match &err.kind {
                ErrorKind::Io(err) if err.raw_os_error() == Some(38) => {
                    let child_entity = parser
                        .last_child_entity()
                        .expect("should have a child entity");
                    parser.define_child(child_entity, "use_fallback", true);
                }
                _ => {}
            },
        }

        parser.add_custom_with("poll_interval", |p, content| {
            let child_entity = p.last_child_entity().expect("should have child entity");

            match TimerSettings::lexer(&content).next() {
                Some(TimerSettings::Duration(duration)) => {
                    p.define_child(child_entity, "poll_interval", Value::Float(duration))
                }
                _ => {
                    event!(
                        Level::ERROR,
                        "Could not parse poll_interval setting, {content}"
                    );
                }
            }
        });

        parser.add_custom_with("compare_contents", |p, _| {
            let child_entity = p.last_child_entity().expect("should have a child entity");
            p.define_child(child_entity, "compare_contents", true);
        })
    }
}

impl BlockObject for Watch {
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
            .require("watch")
            .optional("poll_interval")
            .optional("compare_contents")
            .optional("create")
            .optional("access")
            .optional("modify")
            .optional("remove")
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }

    // TODO - Implement documentation
    // TODO - Implement returns
}

impl Watch {
    /// Selects and initializes the watcher
    ///
    fn watch<'a>(
        event_handler: impl EventHandler,
        config: Config,
        path: impl AsRef<Path>,
        use_fallback: bool,
    ) -> (notify::Result<()>, Watchers) {
        if use_fallback {
            let mut watcher = PollWatcher::new(event_handler, config)
                .expect("should be able to create poll watcher");

            (
                watcher.watch(path.as_ref(), notify::RecursiveMode::NonRecursive),
                Watchers::Fallback(watcher, Arc::new(path.as_ref().to_owned())),
            )
        } else {
            let mut watcher = recommended_watcher(event_handler)
                .expect("should be able to create recommended watcher");
            watcher
                .configure(config)
                .expect("should be able to configure");
            (
                watcher.watch(path.as_ref(), notify::RecursiveMode::NonRecursive),
                Watchers::Recommended(watcher, Arc::new(path.as_ref().to_owned())),
            )
        }
    }
}

enum Watchers {
    Recommended(RecommendedWatcher, Arc<PathBuf>),
    Fallback(PollWatcher, Arc<PathBuf>),
}

impl Drop for Watchers {
    fn drop(&mut self) {
        match self {
            Watchers::Recommended(rec, path) => {
                rec.unwatch(path).ok();
            }
            Watchers::Fallback(fallb, path) => {
                fallb.unwatch(path).ok();
            }
        }
    }
}
