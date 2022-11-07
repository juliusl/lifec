use std::{fs::File, path::{PathBuf, Path}};

use crate::{
    engine::Performance,
    prelude::{Journal, NodeStatus},
};

use super::Guest;

pub trait Monitor {
    fn update_performance(&self, output_dir: impl AsRef<Path>) -> bool;
    fn update_status(&self, output_dir: impl AsRef<Path>) -> bool;
    fn update_journal(&self, output_dir: impl AsRef<Path>) -> bool;
}

impl Monitor for Guest {
    fn update_performance(&self, output_dir: impl AsRef<Path>) -> bool {
        let performance_dir = output_dir.as_ref().join("performance");
        let control = performance_dir.join("control");
        let frames = performance_dir.join("frames");
        let blob = performance_dir.join("blob");
        std::fs::create_dir_all(&performance_dir).expect("should be able to create dirs");

        let performance_exists = control.exists() && frames.exists() && blob.exists();
        if !performance_exists && self.encode_performance() {
            fn write_stream(path: PathBuf) -> impl FnOnce() -> File {
                move || {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(&path)
                        .ok()
                        .unwrap()
                }
            }

            self.update_protocol(|protocol| {
                protocol.send::<Performance, _, _>(
                    write_stream(control),
                    write_stream(frames),
                    write_stream(blob),
                );
                true
            })
        } else {
            false
        }
    }

    fn update_status(&self, output_dir: impl AsRef<Path>) -> bool {
        let status_dir = output_dir.as_ref().join("status");
        let control = status_dir.join("control");
        let frames = status_dir.join("frames");
        let blob = status_dir.join("blob");
        std::fs::create_dir_all(&status_dir).expect("should be able to create dirs");

        let status_exists = control.exists() && frames.exists() && blob.exists();
        if !status_exists && self.encode_status() {
            fn write_stream(path: PathBuf) -> impl FnOnce() -> File {
                move || {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(&path)
                        .ok()
                        .unwrap()
                }
            }

            self.update_protocol(|protocol| {
                protocol.send::<NodeStatus, _, _>(
                    write_stream(control),
                    write_stream(frames),
                    write_stream(blob),
                );
                true
            })
        } else {
            false
        }
    }

    fn update_journal(&self, output_dir: impl AsRef<Path>) -> bool {
        let journal_dir = output_dir.as_ref().join("journal");
        let control = journal_dir.join("control");
        let frames = journal_dir.join("frames");
        let blob = journal_dir.join("blob");
        std::fs::create_dir_all(&journal_dir).expect("should be able to create dirs");

        let journal_exists = control.exists() && frames.exists() && blob.exists();
        if !journal_exists && self.encode_journal() {
            fn write_stream(path: PathBuf) -> impl FnOnce() -> File {
                move || {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(&path)
                        .ok()
                        .unwrap()
                }
            }

            self.update_protocol(|protocol| {
                protocol.send::<Journal, _, _>(
                    write_stream(control),
                    write_stream(frames),
                    write_stream(blob),
                );
                true
            })
        } else {
            false
        }
    }
}
