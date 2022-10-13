use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use atlier::system::Value;
use logos::Logos;
use notify::{event::RemoveKind, EventKind};
use reality::SpecialAttribute;

/// Enumeration of `remove` event kinds
///
#[derive(Logos)]
pub enum Remove {
    #[token("file", |_| RemoveKind::File)]
    File(RemoveKind),
    #[token("folder", |_| RemoveKind::Folder)]
    Folder(RemoveKind),
    #[token("other", |_| RemoveKind::Other)]
    Other(RemoveKind),
    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

impl SpecialAttribute for Remove {
    fn ident() -> &'static str {
        "remove"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        if !content.as_ref().is_empty() {
            let entity = parser
                .last_child_entity()
                .expect("should be installed by the watch plugin");

            let mut hasher = DefaultHasher::default();
            let hasher = &mut hasher;
            if let Some(token) = Remove::lexer(content.as_ref()).next() {
                match token {
                    Remove::File(file) => EventKind::Remove(file),
                    Remove::Folder(folder) => EventKind::Remove(folder),
                    Remove::Other(other) => EventKind::Remove(other),
                    Remove::Error => EventKind::Remove(RemoveKind::Any),
                }
                .hash(hasher);

                parser.define_child(
                    entity,
                    "event_kind",
                    Value::BinaryVector(hasher.finish().to_be_bytes().to_vec()),
                )
            }
        }
    }
}
