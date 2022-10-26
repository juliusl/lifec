use crate::prelude::*;

/// Thunk source returned by a runtime, that can be used to schedule events w/ a world
/// 
/// Catalogs aspects of the underlying plugin driving the event's thunk.
/// 
#[derive(Debug)]
pub struct ThunkSource {
    /// The event struct component this source returns,
    /// 
    thunk: Thunk,
    /// Description of the plugin,
    /// 
    description: Option<String>, 
    /// Caveats of the plugin,
    /// 
    caveats: Option<String>, 
}

impl ThunkSource {
    /// Returns a new event source,
    /// 
    pub fn new<P>() -> Self 
    where 
        P: Plugin + Default + Send
    {
        ThunkSource {
            thunk: Thunk::from_plugin::<P>(),
            description: {
                let description = P::description();
                if description.is_empty() {
                    None 
                } else {
                    Some(description.to_string())
                }
            },
            caveats: {
                let caveats = P::caveats();
                if caveats.is_empty() {
                    None 
                } else {
                    Some(caveats.to_string())
                }
            },
        }
    }
}

impl Clone for ThunkSource {
    fn clone(&self) -> Self {
        Self { 
            thunk: self.thunk.clone(),
            description: self.description.clone(),
            caveats: self.caveats.clone(),
        }
    }
}
