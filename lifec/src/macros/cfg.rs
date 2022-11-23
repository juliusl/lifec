
macro_rules! cfg_editor {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "editor")]
            $item
        )*
    }
}
