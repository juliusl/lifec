use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use logos::Logos;
use notify::{event::CreateKind, EventKind};
use reality::SpecialAttribute;
use reality::Value;

/// Enumeration of `create` event kinds
///
#[derive(Logos)]
pub enum Create {
    #[token("file", |_| CreateKind::File)]
    File(CreateKind),
    #[token("folder", |_| CreateKind::Folder)]
    Folder(CreateKind),
    #[token("other", |_| CreateKind::Other)]
    Other(CreateKind),
    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

impl SpecialAttribute for Create {
    fn ident() -> &'static str {
        "create"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        if !content.as_ref().is_empty() {
            let entity = parser
                .last_child_entity()
                .expect("should be installed by the watch plugin");

            let mut hasher = DefaultHasher::default();
            let hasher = &mut hasher;
            if let Some(token) = Create::lexer(content.as_ref()).next() {
                match token {
                    Create::File(file) => EventKind::Create(file),
                    Create::Folder(folder) => EventKind::Create(folder),
                    Create::Other(other) => EventKind::Create(other),
                    Create::Error => EventKind::Create(CreateKind::Any),
                }
                .hash(hasher);

                let hash = hasher.finish();
                let [a, b] = bytemuck::cast::<u64, [i32; 2]>(hash);
                parser.define_child(
                    entity,
                    "event_kind",
                    Value::IntPair(a, b),
                )
            }
        }
    }
}
