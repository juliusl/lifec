use reality::{BlockObject, BlockProperties, AttributeParser};

use super::{Plugin, AttributeIndex};

/// Pointer struct for "define" plugin,
/// 
/// This plugin simply allows for defining attributes in the context,
/// 
#[derive(Default)]
pub struct Define;

impl Plugin for Define {
    fn symbol() -> &'static str {
        "define"
    }

    fn description() -> &'static str {
        "Defines an attribute in state for subsequent plugins"
    }

    fn caveats() -> &'static str {
        "Expects the input to be an attribute expression"
    }

    fn call(context: &mut super::ThunkContext) -> Option<super::AsyncContext> {
        context.task(|_| {
            let mut tc = context.clone();

            async move {
                if let Some(expression) = tc.search().find_symbol("define") {
                    let mut parser = AttributeParser::default();
                    parser.set_id(tc.entity_id());
                    parser.parse(expression);
    
                    if let Some(attribute) = parser.next() {
                        tc.add_attribute(attribute);
                    }
                }

                tc.copy_previous();
                Some(tc)
            }
        })
    }
}

impl BlockObject for Define {
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}