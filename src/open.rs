pub use atlier::system::{App, Extension};
pub use specs::System;


/// Opens a new window w/ the provided App and Extension
pub fn open<A, E>(
    title: &str,
    app: A,
    extension: E
) 
where
    A: App + for<'c> System<'c>,
    E: Extension + 'static
{
    use atlier::system::open_window;
    let &[width, height] = A::window_size();

    open_window(
        title,
        width.into(),
        height.into(),
        app,
        extension
    )
}