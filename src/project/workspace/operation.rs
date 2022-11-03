use crate::engine::Adhoc;
use crate::prelude::Runtime;
use atlier::system::Value;
use reality::SpecialAttribute;
use specs::WorldExt;

/// Special attribute to define an operation in the root block for the workspace,
///
pub struct Operations;

impl SpecialAttribute for Operations {
    fn ident() -> &'static str {
        "operation"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        Runtime::parse(parser, "");
        let world = parser.world().expect("should have world").clone();
        let mut adhocs = world.write_component::<Adhoc>();
        let operation_entity = world.entities().create();
        let name = content.as_ref().to_string();

        let tag = if let Some(tag) = parser.name() {
            if tag != "operation" {
                let tag = format!("{tag}.operation");
                parser.set_name(&tag);
                tag.to_string()
            } else {
                tag.to_string()
            }
        } else {
            panic!("parser should have had a name");
        };

        parser.set_id(operation_entity.id() as u32);
        parser.define("name", Value::Symbol(name.to_string()));
        adhocs
            .insert(operation_entity, Adhoc { name, tag })
            .expect("should be able to insert component");
    }
}
