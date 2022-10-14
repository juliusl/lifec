use reality::SpecialAttribute;

/// Special attribute for control blocks,
/// 
/// # Overview
/// This allows a control block engine branch to authn utilities.
/// It will ensure that a keypair is available when the world is being compiled, and
/// available before any plugins are run.
/// 
/// # Example Usage
/// ```runmd
/// <``` test>
/// : .control_key
/// <```>
/// ```
/// 
pub struct ControlKey;

impl SpecialAttribute for ControlKey {
    fn ident() -> &'static str {
        "control_key"
    }

    fn parse(_parser: &mut reality::AttributeParser, _: impl AsRef<str>) {
        todo!()
    }
}

