use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Resolve existing symlinks, then normalize the not-yet-existing tail.
/// This lets preflight detect aliases without creating output directories.
pub fn resolve(path: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut ancestor = absolute.as_path();
    let mut tail = Vec::new();
    let mut resolved = loop {
        match fs::canonicalize(ancestor) {
            Ok(path) => break path,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // A dangling symlink is not a missing directory we may create.
                if fs::symlink_metadata(ancestor).is_ok() {
                    return Err(e);
                }
                tail.push(ancestor.components().next_back().ok_or(e)?);
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| io::Error::other("no existing parent"))?;
            }
            Err(e) => return Err(e),
        }
    };
    for component in tail.into_iter().rev() {
        match component {
            Component::ParentDir => {
                resolved.pop();
            }
            Component::CurDir => {}
            _ => resolved.push(component.as_os_str()),
        }
    }
    Ok(resolved)
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct FileId {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(not(unix))]
    path: PathBuf,
}

impl FileId {
    pub fn read(path: &Path) -> io::Result<Option<Self>> {
        let metadata = match fs::metadata(path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(Some(Self {
                device: metadata.dev(),
                inode: metadata.ino(),
            }))
        }
        #[cfg(not(unix))]
        {
            let _ = metadata;
            Ok(Some(Self {
                path: fs::canonicalize(path)?,
            }))
        }
    }
}
