use std::path::PathBuf;

use reality::Block;
use specs::{Component, VecStorage};
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
///     * Tenant can be added to the host, .world/{host}/{tenant}
///     * A tenant can have several paths, .world/{host}/{tenant}/{path}
/// - Authenticating entities
///     * By default when setting up a workspace host, a keypair is used to sign subsequent tenant dirs
///     * Each tenant dir will have a keypair to sign path dirs
///     * Each path dir will have it's own key to validate information it provides
///     * When a plugin runs, it will need to
/// - Orchestrating multiple hosts,
///     * Within a host directory, each tenant will have a seperate Host instance
///     * Within the context of the workspace, each dir with a .runmd file is considered a host
///     * A Host is the owner of the path hosts within it's directory
///     * To tell that a path was added by a tenant, the tenant will sign a file that authenticates each path host
///
#[derive(Clone, Debug, Component)]
#[storage(VecStorage)]
pub struct Workspace {
    /// Work directory for this workspace context,
    work_dir: PathBuf,
    /// Root directory
    root: Option<PathBuf>,
    /// Name of the host,
    host: String,
    /// Name of the tenant,
    tenant: Option<String>,
    /// Name of the path,
    path: Option<String>,
}

impl Workspace {
    /// Returns a new workspace for host,
    ///
    pub fn new(host: impl AsRef<str>, root: Option<PathBuf>) -> Self {
        let work_dir = root
            .clone()
            .unwrap_or(PathBuf::from("."))
            .join(".world")
            .join(host.as_ref());

        Self {
            work_dir: work_dir.to_path_buf(),
            root,
            host: host.as_ref().to_string(),
            tenant: None,
            path: None,
        }
    }

    /// Returns the identity uri for the current workspace context for a block,
    ///
    pub fn identity_uri(&self, block: &Block) -> Option<String> {
        match (self.host.as_str(), self.tenant.as_ref(), self.path.as_ref()) {
            (host, Some(tenant), None) if !block.name().is_empty() => Some(format!(
                "{}.{}.{tenant}.{host}",
                block.name(),
                block.symbol()
            )),
            (host, Some(tenant), None) if block.name().is_empty() => {
                Some(format!("{}.control.{tenant}.{host}", block.symbol()))
            }
            (host, Some(tenant), Some(path)) if !block.name().is_empty() => Some(format!(
                "{}.{}.{tenant}.{host}/{path}",
                block.name(),
                block.symbol()
            )),

            (host, Some(tenant), Some(path)) if block.name().is_empty() => {
                Some(format!("{}.control.{tenant}.{host}/{path}", block.symbol()))
            }
            _ => None,
        }
    }

    /// Get a tenant from the workspace,
    ///
    pub fn tenant(&self, tenant: impl AsRef<str>) -> Self {
        let work_dir = self
            .root
            .clone()
            .unwrap_or(PathBuf::from("."))
            .join(".world")
            .join(self.host.as_str())
            .join(tenant.as_ref());

        Self {
            work_dir,
            root: self.root.clone(),
            host: self.host.to_string(),
            tenant: Some(tenant.as_ref().to_string()),
            path: None,
        }
    }

    /// Get a path from the workspace,
    ///
    pub fn path(&self, path: impl AsRef<str>) -> Option<Self> {
        if let Some(tenant) = self.tenant.as_ref() {
            let work_dir = self
                .root
                .clone()
                .unwrap_or(PathBuf::from("."))
                .join(".world")
                .join(self.host.as_str())
                .join(tenant.as_str())
                .join(path.as_ref());

            Some(Self {
                work_dir,
                root: self.root.clone(),
                host: self.host.to_string(),
                tenant: Some(tenant.to_string()),
                path: Some(path.as_ref().to_string()),
            })
        } else {
            event!(Level::ERROR, "Trying to create a path without a tenant");
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

    let workspace = Workspace::new("lifec.io", None);

    assert_eq!(&PathBuf::from(".world/lifec.io"), workspace.work_dir());

    let tenant = workspace.tenant("test");
    assert_eq!(&PathBuf::from(".world/lifec.io/test"), tenant.work_dir());
    assert_eq!(
        Some("workspace.control.test.lifec.io".to_string()),
        tenant.identity_uri(parser.get_block("", "workspace"))
    );

    let path = tenant
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
