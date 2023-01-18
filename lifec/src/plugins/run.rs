use std::marker::PhantomData;

use reality::{BlockObject, BlockProperties};

use super::{Host, Plugin, Project};

use crate::prelude::{AsyncContext, AttributeIndex, Operations, ThunkContext};

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

    fn call(context: &mut ThunkContext) -> Option<AsyncContext> {
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

                    let world = P::compile_workspace(root, [].iter(), None);
                    let mut host = Host::from(world);
                    let _ = host.prepare::<P>();

                    let result = {
                        let workspace_oeprations = host.world().system_data::<Operations>();

                        if let Some(mut operation) = workspace_oeprations.execute_operation(
                            operation,
                            root.tag().cloned(),
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
