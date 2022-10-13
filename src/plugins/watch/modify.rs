use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use atlier::system::Value;
use logos::Logos;
use notify::{event::ModifyKind, event::DataChange, event::MetadataKind, event::RenameMode, EventKind};
use reality::SpecialAttribute;

/// Enumeration of `modify` event kinds
///
#[derive(Logos)]
pub enum Modify {
    #[token("data", |_| ModifyKind::Data(DataChange::Any))]
    Data(ModifyKind),
    #[token("size", |_| ModifyKind::Data(DataChange::Size))]
    Size(ModifyKind),
    #[token("content", |_| ModifyKind::Data(DataChange::Content))]
    Content(ModifyKind),
    #[token("data other", |_| ModifyKind::Data(DataChange::Other))]
    DataOther(ModifyKind),
    #[token("metadata", |_| ModifyKind::Metadata(MetadataKind::Any))]
    Metadata(ModifyKind),
    #[token("access_time", |_| ModifyKind::Metadata(MetadataKind::AccessTime))]
    AccessTime(ModifyKind),
    #[token("write_time", |_| ModifyKind::Metadata(MetadataKind::WriteTime))]
    WriteTime(ModifyKind),
    #[token("permissions", |_| ModifyKind::Metadata(MetadataKind::Permissions))]
    Permissions(ModifyKind),
    #[token("ownership", |_| ModifyKind::Metadata(MetadataKind::Ownership))]
    Ownership(ModifyKind),
    #[token("extended", |_| ModifyKind::Metadata(MetadataKind::Extended))]
    Extended(ModifyKind),
    #[token("metadata other", |_| ModifyKind::Metadata(MetadataKind::Other))]
    MetadataOther(ModifyKind),
    #[token("name", |_| ModifyKind::Name(RenameMode::Any))]
    Name(ModifyKind),
    #[token("name to", |_| ModifyKind::Name(RenameMode::To))]
    NameTo(ModifyKind),
    #[token("name from", |_| ModifyKind::Name(RenameMode::From))]
    NameFrom(ModifyKind),
    #[token("name both", |_| ModifyKind::Name(RenameMode::Both))]
    NameBoth(ModifyKind),
    #[token("name other", |_| ModifyKind::Name(RenameMode::Other))]
    NameOther(ModifyKind),
    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

impl SpecialAttribute for Modify {
    fn ident() -> &'static str {
        "modify"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        if !content.as_ref().is_empty() {
            let entity = parser
                .last_child_entity()
                .expect("should be installed by the watch plugin");

            let mut hasher = DefaultHasher::default();
            let hasher = &mut hasher;
            if let Some(token) = Modify::lexer(content.as_ref()).next() {
                match token {
                    Modify::Data(k) |
                    Modify::Size(k) |
                    Modify::Content(k) |
                    Modify::DataOther(k) |
                    Modify::Metadata(k) |
                    Modify::AccessTime(k) |
                    Modify::WriteTime(k) |
                    Modify::Permissions(k) |
                    Modify::Ownership(k) |
                    Modify::Extended(k) |
                    Modify::MetadataOther(k) |
                    Modify::Name(k) |
                    Modify::NameTo(k) |
                    Modify::NameFrom(k) |
                    Modify::NameBoth(k) |
                    Modify::NameOther(k) => EventKind::Modify(k),
                    Modify::Error => EventKind::Modify(ModifyKind::Any),
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
