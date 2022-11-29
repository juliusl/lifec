use rust_embed::RustEmbed;
use std::path::PathBuf;

use crate::prelude::*;

/// Handles unpacking resources for a `RustEmbed` source
///
pub struct Resources(
    /// folder prefix
    pub &'static str,
);

impl Resources {
    /// Reads a string from file
    ///
    /// If the file doesn't exist, unpacks the resource from an embedded resource if it exists.
    pub async fn read_string<C>(&self, tc: &ThunkContext, src: &String) -> Option<String>
    where
        C: RustEmbed,
    {
        self.unpack_resource::<C>(&tc, &src).await;

        match tokio::fs::read_to_string(&src).await {
            Ok(content) => Some(content),
            Err(err) => {
                event!(Level::ERROR, "error reading file {src}, {err}");
                None
            }
        }
    }

    /// Reads binary content from a file
    ///
    /// If the file doesn't exist, unpacks the resource from an embedded resource if it exists.
    pub async fn read_binary<C>(&self, tc: &ThunkContext, src: &String) -> Option<Vec<u8>>
    where
        C: RustEmbed,
    {
        self.unpack_resource::<C>(&tc, &src).await;

        match tokio::fs::read(&src).await {
            Ok(content) => Some(content),
            Err(err) => {
                event!(Level::ERROR, "error reading file {src}, {err}");
                None
            }
        }
    }

    /// Reads String from a file, from the src path specified by an attribute
    pub async fn read_string_from<C>(
        &self,
        tc: &ThunkContext,
        attribute_name: &String,
    ) -> Option<String>
    where
        C: RustEmbed,
    {
        if let Some(src) = tc.state().find_symbol(attribute_name) {
            self.read_string::<C>(tc, &src).await
        } else {
            None
        }
    }

    /// Reads binary from a file, from the src path specified by an attribute
    pub async fn read_binary_from<C>(
        &self,
        tc: &ThunkContext,
        attribute_name: &String,
    ) -> Option<Vec<u8>>
    where
        C: RustEmbed,
    {
        if let Some(src) = tc.state().find_symbol(attribute_name) {
            self.read_binary::<C>(tc, &src).await
        } else {
            None
        }
    }

    /// Checks if a resource exists locally, if not unpacks the resource from the binary.
    pub async fn unpack_resource<C>(&self, tc: &ThunkContext, src: &String)
    where
        C: RustEmbed,
    {
        let Self(prefix) = self;

        let path = PathBuf::from(&src);
        if let Some(parent) = path.parent() {
            match tokio::fs::create_dir_all(parent).await {
                Ok(_) => {
                    tc.status(format!("Using embedded resource {prefix} for {src}"))
                        .await;
                }
                Err(err) => {
                    event!(Level::ERROR, "error loading resource {prefix} {err}");
                }
            }
        }

        if !path.exists() {
            if let Some(resource) = C::get(src.trim_start_matches(&format!("{prefix}/"))) {
                match tokio::fs::write(&src, resource.data).await {
                    Ok(_) => {
                        tc.status(format!("Loaded embedded resource {prefix} for {src}"))
                            .await;
                    }
                    Err(err) => {
                        event!(Level::ERROR, "error loading resource {prefix} {err}");
                    }
                }
            }
        }
    }
}
