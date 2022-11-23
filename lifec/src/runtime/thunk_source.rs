use crate::prelude::*;

/// Thunk source returned by a runtime, that can be used to schedule events w/ a world
/// 
/// Catalogs aspects of the underlying plugin driving the event's thunk.
/// 
#[derive(Debug, Clone)]
pub struct ThunkSource {
    /// The event struct component this source returns,
    /// 
    thunk: Thunk,
    /// Description of the plugin,
    /// 
    _description: Option<String>, 
    /// Caveats of the plugin,
    /// 
    _caveats: Option<String>, 
}

impl ThunkSource {
    pub fn thunk(&self) -> Thunk {
        self.thunk
    }
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
            _description: {
                let description = P::description();
                if description.is_empty() {
                    None 
                } else {
                    Some(description.to_string())
                }
            },
            _caveats: {
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
