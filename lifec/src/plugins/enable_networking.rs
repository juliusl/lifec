use crate::Plugin;

/// Plugin to enable networking,
/// 
pub struct EnableNetworking;

impl Plugin for EnableNetworking {
    fn symbol() -> &'static str {
        "enable_networking"
    }

    fn description() -> &'static str {
        "Plugin to enable networking appliances for a thunk context"
    }

    fn call(context: &crate::ThunkContext) -> Option<crate::AsyncContext> {
        context.task(|_| {
            let mut tc = context.clone();
            async move {
                // Assign-addresses to context
                tc.assign_addresses().await;
                
                Some(tc)
            }
        })
    }
}