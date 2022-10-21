use atlier::system::Value;
use reality::SpecialAttribute;

/// Special attribute to customize properties from the root block of a workspace,
/// 
pub struct Config;

impl SpecialAttribute for Config {
    fn ident() -> &'static str {
        "config"
    }

    /// Parses a set of properties to insert into state,
    /// 
    /// Content is a uri expression that resolves to the graph that will be configured,
    /// 
    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        if let Some(name) = parser.name() {
            if name != "config" {
                parser.set_name(format!("{name}.config"));
                parser.set_value(Value::Symbol(content.as_ref().to_string()));
            }
        }
    }
}
