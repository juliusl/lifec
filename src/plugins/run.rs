use std::marker::PhantomData;

use reality::{BlockObject, BlockProperties};

use super::{AttributeIndex, Host, Plugin, Project};

use crate::prelude::WorkspaceOperation;

/// Plugin to execute an operation defined within the root runmd of a workspace
///
#[derive(Default)]
pub struct Run<P>(PhantomData<P>)
where
    P: Project + Default;

impl<P> Plugin for Run<P>
where
    P: Project + Default,
{
    fn symbol() -> &'static str {
        "run"
    }

    fn description() -> &'static str {
        "Runs an operation defined within the root of a workspace"
    }

    fn caveats() -> &'static str {
        "Will compile a short-lived host to execute the operation. The plugins available are based on the Project."
    }

    fn call(context: &super::ThunkContext) -> Option<super::AsyncContext> {
        context.task(|cancel_source| {
            let tc = context.clone();
            async move {
                if let Some(root) = tc.workspace() {
                    let mut operation = tc
                        .search()
                        .find_symbol("run")
                        .expect("should have an operation name");

                    if operation.ends_with("}") && operation.starts_with("{") {
                        if let Some(formatted) = tc
                            .search()
                            .find_symbol(operation.trim_start_matches("{").trim_end_matches("}"))
                        {
                            operation = formatted;
                        }
                    }

                    let world = P::compile_workspace(root, [].iter());
                    let mut host = Host::from(world);
                    let _ = host.prepare::<P>();

                    // TODO: This might need some polish
                    let tag = root.iter_tags().next();

                    let result = {
                        let mut workspace_oeprations =
                            host.world().system_data::<WorkspaceOperation>();

                        if let Some(mut operation) = workspace_oeprations.execute_operation(
                            operation,
                            tag.cloned(),
                            Some(&tc),
                        ) {
                            operation.task(cancel_source).await
                        } else {
                            None
                        }
                    };

                    host.exit();
                    result
                } else {
                    Some(tc)
                }
            }
        })
    }
}

impl<P> BlockObject for Run<P>
where
    P: Project + Default,
{
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default().require("run")
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}
