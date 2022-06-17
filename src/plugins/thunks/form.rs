use specs::Component;

use crate::plugins::Plugin;
use specs::storage::DenseVecStorage;
use super::ThunkContext;


#[derive(Component, Default, Clone)]
#[storage(DenseVecStorage)]
pub struct Form;

impl Plugin<ThunkContext> for Form {
    fn symbol() -> &'static str {
        "form"
    }

    fn call_with_context(_: &mut ThunkContext) {
    }
}