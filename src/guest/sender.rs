use std::{
    fs::File,
    path::{Path, PathBuf},
};

use crate::prelude::NodeCommand;

use super::Guest;

pub trait Sender {
    fn send_commands(&self, output_dir: impl AsRef<Path>) -> bool;
}

impl Sender for Guest {
    fn send_commands(&self, output_dir: impl AsRef<Path>) -> bool {
        let commands_dir = output_dir.as_ref();
        let control = commands_dir.join("control");
        let frames = commands_dir.join("frames");
        let blob = commands_dir.join("blob");
        std::fs::create_dir_all(&commands_dir).expect("should be able to create dirs");

        let command_exists = control.exists() && frames.exists() && blob.exists();
        if !command_exists && self.encode_commands() {
            fn write_stream(name: PathBuf) -> impl FnOnce() -> File {
                move || {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(name)
                        .ok()
                        .unwrap()
                }
            }

            self.update_protocol(|protocol| {
                protocol.send::<NodeCommand, _, _>(
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
