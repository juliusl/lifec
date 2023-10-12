use std::fmt::Debug;

use futures_util::TryStreamExt;
use futures_util::StreamExt;

use anyhow::anyhow;

use crate::plugin::ThunkContext;

/// Struct for a top-level node,
///
#[derive(Clone)]
pub struct Operation {
    name: String,
    tag: Option<String>,
    context: Option<ThunkContext>,
}

impl Operation {
    /// Creates a new operation,
    ///
    pub fn new(name: impl Into<String>, tag: Option<String>) -> Self {
        Self {
            name: name.into(),
            tag,
            context: None,
        }
    }

    /// Returns the address to use w/ this operation,
    /// 
    pub fn address(&self) -> String {
        if let Some(tag) = self.tag.as_ref() {
            format!("{}#{}", self.name, tag)
        } else {
            self.name.to_string()
        }
    }

    /// Binds operation to a thunk context,
    /// 
    pub fn bind(&mut self, context: ThunkContext) {
        self.context = Some(context);
    }

    /// Executes the operation,
    /// 
    pub async fn execute(&self) -> anyhow::Result<ThunkContext> {
        if let Some(context) = self.context.clone() {
            let node = reality::Node(context.target.storage.clone());

            node
                .stream_attributes()
                .map(|a| Ok(a))
                .try_fold(
                    context,
                    move |mut tc, a| async move {
                        tc.set_attribute(a);
                        let previous = tc.clone();
                        match tc.call().await {
                            Ok(tc) => {
                                if let Some(tc) = tc {
                                    Ok(tc)
                                } else {
                                    Ok(previous)
                                }
                            }
                            Err(err) => Err(err),
                        }
                    },
                )
                .await
        } else {
            Err(anyhow!("Could not execute operation, "))
        }
        
    }
}

impl Debug for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Operation")
            .field("name", &self.name)
            .field("tag", &self.tag)
            .finish()
    }
}
