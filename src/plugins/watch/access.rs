use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use atlier::system::Value;
use logos::Logos;
use notify::{event::AccessKind, event::AccessMode, EventKind};
use reality::SpecialAttribute;

/// Enumeration of `access` event kinds
///
#[derive(Logos)]
pub enum Access {
    #[token("read", |_| AccessKind::Read)]
    Read(AccessKind),
    #[token("open", |_| AccessKind::Open(AccessMode::Any))]
    Open(AccessKind),
    #[token("open exec", |_| AccessKind::Open(AccessMode::Execute))]
    OpenExecute(AccessKind),
    #[token("open read", |_| AccessKind::Open(AccessMode::Read))]
    OpenRead(AccessKind),
    #[token("open write", |_| AccessKind::Open(AccessMode::Write))]
    OpenWrite(AccessKind),
    #[token("open other", |_| AccessKind::Open(AccessMode::Other))]
    OpenOther(AccessKind),
    #[token("close", |_| AccessKind::Close(AccessMode::Any))]
    Close(AccessKind),
    #[token("close exec", |_| AccessKind::Close(AccessMode::Execute))]
    CloseExecute(AccessKind),
    #[token("close read", |_| AccessKind::Close(AccessMode::Read))]
    CloseRead(AccessKind),
    #[token("close write", |_| AccessKind::Close(AccessMode::Write))]
    CloseWrite(AccessKind),
    #[token("close other", |_| AccessKind::Close(AccessMode::Other))]
    CloseOther(AccessKind),
    #[token("other", |_| AccessKind::Other)]
    Other(AccessKind),
    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

impl SpecialAttribute for Access {
    fn ident() -> &'static str {
        "access"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        if !content.as_ref().is_empty() {
            let entity = parser
                .last_child_entity()
                .expect("should be installed by the watch plugin");

            let mut hasher = DefaultHasher::default();
            let hasher = &mut hasher;
            if let Some(token) = Access::lexer(content.as_ref()).next() {
                match token {
                    Access::Read(k)
                    | Access::Open(k)
                    | Access::OpenExecute(k)
                    | Access::OpenRead(k)
                    | Access::OpenWrite(k)
                    | Access::OpenOther(k)
                    | Access::Close(k)
                    | Access::CloseExecute(k)
                    | Access::CloseRead(k)
                    | Access::CloseWrite(k)
                    | Access::CloseOther(k)
                    | Access::Other(k) => EventKind::Access(k),
                    Access::Error => EventKind::Access(AccessKind::Any),
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
