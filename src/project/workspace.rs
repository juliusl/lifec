use std::{collections::HashMap, path::PathBuf};

use reality::Block;
use tracing::{event, Level};

mod create;
pub use create::Create;

/// Struct for managing a complex runmd project,
///
/// For small projects consisting of a single runmd file, the default Host is good enough.
/// For a more complex project with several parts, it is nice to have a few more utilities
/// to help manage the chaos. In addition, the utilities provided by this struct should also
/// be usable by simple files as well.
///
/// # Utilities
/// - Organizing the work directory for this workspace,
///     * The root is always .world/
///     * A workspace has one host, .world/{host}
///     * Containers can be added to the host, .world/{host}/{container}
///     * A container can have several paths, .world/{host}/{container}/{path}
/// - Authenticating entities
///     * By default when setting up a workspace host, a keypair is used to sign subsequent container dirs
///     * Each container dir will have a keypair to sign path dirs
///     * Each path dir will have it's own key to validate information it provides
///     * When a plugin runs, it will need to
/// - Orchestrating multiple hosts,
///     * Within a host directory, each container will have a seperate Host instance
///     * Within the context of the workspace, each dir with a .runmd file is considered a host
///     * A Host is the owner of the path hosts within it's directory
///     * To tell that a path was added by a container, the container will sign a file that authenticates each path host
///
#[derive(Clone, Debug)]
pub struct Workspace {
    /// Work directory for this workspace context,
    work_dir: PathBuf,
    /// Name of the host,
    host: String,
    /// Name of the container,
    container: Option<String>,
    /// Name of the path,
    path: Option<String>,
    /// Map of public keys,
    public_keys: HashMap<String, rsa::RsaPublicKey>,
}

impl Workspace {
    /// Returns a new workspace for host,
    ///
    pub fn new(host: impl AsRef<str>) -> Self {
        let work_dir = PathBuf::from(".world").join(host.as_ref());
        Self {
            work_dir,
            host: host.as_ref().to_string(),
            container: None,
            path: None,
            public_keys: HashMap::default(),
        }
    }

    /// Returns the identity uri for the current workspace context for a block,
    /// 
    pub fn identity_uri(&self, block: &Block) -> Option<String> {
        match (
            self.host.as_str(),
            self.container.as_ref(),
            self.path.as_ref(),
        ) {
            (host, Some(container), None) if !block.name().is_empty() => Some(format!(
                "{}.{}.{container}.{host}",
                block.name(),
                block.symbol()
            )),
            (host, Some(container), None) if block.name().is_empty() => {
                Some(format!("{}.control.{container}.{host}", block.symbol()))
            }
            (host, Some(container), Some(path)) if !block.name().is_empty() => Some(format!(
                "{}.{}.{container}.{host}/{path}",
                block.name(),
                block.symbol()
            )),

            (host, Some(container), Some(path)) if block.name().is_empty() => Some(format!(
                "{}.control.{container}.{host}/{path}",
                block.symbol()
            )),
            _ => None,
        }
    }

    /// Get a container from the workspace,
    ///
    pub fn container(&self, container: impl AsRef<str>) -> Self {
        let work_dir = PathBuf::from(".world")
            .join(self.host.as_str())
            .join(container.as_ref());

        Self {
            work_dir,
            host: self.host.to_string(),
            container: Some(container.as_ref().to_string()),
            path: None,
            public_keys: self.public_keys.clone(),
        }
    }

    /// Get a path from the workspace,
    ///
    pub fn path(&self, path: impl AsRef<str>) -> Option<Self> {
        if let Some(container) = self.container.as_ref() {
            let work_dir = PathBuf::from(".world")
                .join(self.host.as_str())
                .join(container.as_str())
                .join(path.as_ref());

            Some(Self {
                work_dir,
                host: self.host.to_string(),
                container: Some(container.to_string()),
                path: Some(path.as_ref().to_string()),
                public_keys: self.public_keys.clone(),
            })
        } else {
            event!(
                Level::ERROR,
                "Trying to create a path without a container set"
            );
            None
        }
    }

    /// Returns a path buf to the work dir,
    ///
    pub fn work_dir(&self) -> &PathBuf {
        &self.work_dir
    }
}

#[test]
fn test_workspace_paths() {
    use reality::Parser;

    let mut parser = Parser::new().parse(
        r#"
    ``` workspace
    ```

    ``` try workspace
    ```
    "#,
    );

    let workspace = Workspace::new("lifec.io");

    assert_eq!(&PathBuf::from(".world/lifec.io"), workspace.work_dir());

    let container = workspace.container("test");
    assert_eq!(&PathBuf::from(".world/lifec.io/test"), container.work_dir());
    assert_eq!(
        Some("workspace.control.test.lifec.io".to_string()),
        container.identity_uri(parser.get_block("", "workspace"))
    );

    let path = container
        .path("tester")
        .expect("should be able to create a path");
    assert_eq!(
        &PathBuf::from(".world/lifec.io/test/tester"),
        path.work_dir()
    );
    assert_eq!(
        Some("try.workspace.test.lifec.io/tester".to_string()),
        path.identity_uri(parser.get_block("try", "workspace"))
    );
}
