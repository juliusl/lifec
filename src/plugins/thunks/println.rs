use crate::plugins::Plugin;
use super::ThunkContext;
use specs::Component;
use specs::storage::DenseVecStorage;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

#[derive(Default, Component)]
#[storage(DenseVecStorage)]
pub struct Println;

impl Plugin<ThunkContext> for Println {
    fn symbol() -> &'static str {
        "println"
    }

    fn description() -> &'static str {
        "Can be used to debug attributes passed as input to this thunk."
    }

    fn call_with_context(context: &mut ThunkContext, _: Option<Handle>) -> Option<JoinHandle<()>> {
        context.accept("thunk", |a| {
            a.is_stable()
        });

        context
            .as_ref()
            .iter_attributes()
            .map(|a| (a.name(), a.value()))
            .for_each(|(name, value)| {
                println!("{}: {}", name, value);
            });

        context.publish(|a| a.add_bool_attr("printed", true));

        None
    }
}
