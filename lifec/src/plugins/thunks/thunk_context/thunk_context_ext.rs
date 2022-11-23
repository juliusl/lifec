use crate::prelude::*;

/// Attribute Index interface for thunk context,
///
/// Find values will also search previous state, if current state is missing the value
///
impl AttributeIndex for ThunkContext {
    fn entity_id(&self) -> u32 {
        self.state().entity_id()
    }

    fn hash_code(&self) -> u64 {
        self.state().hash_code()
    }

    fn find_value(&self, with_name: impl AsRef<str>) -> Option<reality::Value> {
        if let Some(value) = self.state().find_value(with_name.as_ref()) {
            Some(value)
        } else if let Some(value) = self
            .previous()
            .and_then(|p| p.find_value(with_name.as_ref()))
        {
            Some(value)
        } else {
            None
        }
    }

    fn find_values(&self, with_name: impl AsRef<str>) -> Vec<reality::Value> {
        let values = self.state().find_values(with_name.as_ref());

        match self.previous() {
            Some(previous) if values.is_empty() => previous.find_values(with_name.as_ref()),
            _ => values,
        }
    }

    fn add_attribute(&mut self, attr: reality::Attribute) {
        self.state_mut().add_attribute(attr);
    }

    fn replace_attribute(&mut self, attr: reality::Attribute) {
        self.state_mut().replace_attribute(attr);
    }

    fn values(&self) -> std::collections::BTreeMap<String, Vec<reality::Value>> {
        self.state().values()
    }

    fn properties(&self) -> &BlockProperties {
        self.state().properties()
    }

    fn properties_mut(&mut self) -> &mut BlockProperties {
        self.state_mut().properties_mut()
    }

    fn control_values(&self) -> &std::collections::BTreeMap<String, Value> {
        self.state().control_values()
    }
}
