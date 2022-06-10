mod process;
pub use process::Process;

mod project;
pub use project::Project;
pub use project::Document;

mod thunks;
pub use thunks::ThunkContext;
pub use thunks::Println;
pub use thunks::WriteFiles;

mod render;
pub use render::Render;

use crate::AttributeGraph;

pub trait Plugin<T> 
where
    T: AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph>
{
  /// Returns the symbol name for this thunk, to reference call by name
  fn symbol() -> &'static str;
    
  fn description() -> &'static str {
      ""
  }

  /// Transforms attribute graph into a thunk context and calls call_with_context
  /// Updates graph afterwards.
  fn call(attributes: &mut AttributeGraph) {
      use crate::RuntimeState;
    
      let mut context = T::from(attributes.clone());
      let context = &mut context;
      Self::call_with_context(context);

      *attributes = attributes.merge_with(context.as_ref());
  }

  fn call_with_context(context: &mut T);
}