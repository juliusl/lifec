use rust_embed::RustEmbed;
use std::path::PathBuf;

use crate::plugins::ThunkContext;

/// Handles unpacking resources for a `RustEmbed` source
pub struct Resources(&'static str);

impl Resources {
    /// Reads a string from file, from the src path specified by an attribute
    ///
    /// If the file doesn't exist, unpacks the resource from an embedded resource if it exists.
    pub async fn read_string<C>(
        &self,
        tc: &ThunkContext,
        attribute_name: impl AsRef<str>,
    ) -> Option<String>
    where
        C: RustEmbed,
    {
        if let Some(src) = tc.as_ref().find_text(attribute_name) {
            self.unpack_resource::<C>(&tc, &src).await;

            match tokio::fs::read_to_string(&src).await {
                Ok(content) => Some(content),
                Err(err) => {
                    eprintln!("error reading file {src}, {err}");
                    None
                }
            }
        } else {
            None
        }
    }

    /// Reads binary content from a file, with the src specified by a text attribute.
    /// 
    /// If the file doesn't exist, unpacks the resource from an embedded resource if it exists.
    pub async fn read_binary<C>(
        &self,
        tc: &ThunkContext,
        attribute_name: impl AsRef<str>,
    ) -> Option<Vec<u8>>
    where
        C: RustEmbed,
    {
        if let Some(src) = tc.as_ref().find_text(attribute_name) {
            self.unpack_resource::<C>(&tc, &src).await;

            match tokio::fs::read(&src).await {
                Ok(content) => Some(content),
                Err(err) => {
                    eprintln!("error reading file {src}, {err}");
                    None
                }
            }
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
                    tc.update_status_only(format!("Using embedded resource {prefix} for {src}"))
                        .await;
                }
                Err(err) => {
                    eprintln!("error loading resource {prefix} {err}");
                }
            }
        }

        if !path.exists() {
            if let Some(resource) = C::get(src.trim_start_matches(&format!("{prefix}/"))) {
                match tokio::fs::write(&src, resource.data).await {
                    Ok(_) => {
                        tc.update_status_only(format!(
                            "Loaded embedded resource {prefix} for {src}"
                        ))
                        .await;
                    }
                    Err(err) => {
                        eprintln!("error loading resource {prefix} {err}");
                    }
                }
            }
        }
    }
}
