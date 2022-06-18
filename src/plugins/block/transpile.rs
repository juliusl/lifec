use std::fs;

use crate::plugins::Plugin;

use super::BlockContext;

pub struct Transpile;

impl Plugin<BlockContext> for Transpile {
    fn symbol() -> &'static str {
        "transpile"
    }

    fn call_with_context(context: &mut BlockContext) {
        if let Some(output_file) = context.get_block("file") {
            if let Some(path) = output_file.find_text("runmd_path") {
                if let Some(content) = context.transpile().ok() {
                    match fs::write(path, content) {
                        Ok(_) => {
    
                        }, 
                        Err(_) => {
                            
                        }
                    }
                }
            }
        }
    }
}
