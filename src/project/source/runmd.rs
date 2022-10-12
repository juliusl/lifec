
/// Reality does not define any type of special handling for .runmd files. This struct aims to 
/// add additional patterns to make authoring files ergonomic. 
/// 
/// The first consideration we will make is the file name of a runmd file. If a runmd file has no names, then
/// control blocks must be explicitly declared, and the root block can be defined. 
/// 
/// If a runmd file has a name, then the root block found in this file will implicitly be a control block, w/ the file name as it's symbol.
/// For example, given a file test.runmd with the following contents, 
/// 
/// ```runmd
/// <```>
/// + .engine
/// : .event print
/// : .exit
/// <```>
/// 
/// <``` print>
/// + .runtime
/// : .println hello world
/// <```>
/// ```
/// 
/// When this is loaded via this struct, the root block is interpreted to be ``` test, and the control block is interpreted to be ``` print test. This means
/// a runmd file w/ a name cannot define or configure the root block. 
/// 
/// On the other hand, given a file .runmd with the same contents, this would be interpreted as is, since no file name is available to interpret as the symbol.
/// 
/// This allows more complicated projects to split up abstractions into seperate files. When the project is compiled, these files can be concatenated 
/// in a more predictable way, as if the file was defined in one document.
/// 
pub struct RunmdFile {
    /// This is the file name, and will be used as the implicit symbol for all blocks found in the file.
    /// If a block has both a name and a symbol, it will be ignored when this struct is consumed into Source,
    /// and then compiled into a block.
    /// 
    symbol: String,
    /// This is the parent directory or container name for this file.
    /// 
    parent: String,
}

