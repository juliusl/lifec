mod next;
pub use next::Next;

mod engine;
pub use engine::Engine;
pub use engine::compile_runmd_engine;
use reality::v2::Compiler;

/// Compiles V2 framework,
/// 
pub fn compile(compiler: &mut Compiler) -> reality::Result<()> {
    let _ = compile_runmd_engine(compiler)?;

    Ok(())
}