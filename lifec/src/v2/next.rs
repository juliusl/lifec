use reality::v2::prelude::*;

use crate::engine::Transition;

#[thunk]
#[async_trait]
pub trait Next
{
    /// Calls the event and returns the next transition,
    /// 
    async fn next(&self) -> Result<Transition>;
}
