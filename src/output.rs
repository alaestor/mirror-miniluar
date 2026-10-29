//! Stage complete output beside its destination, then publish without truncating it.
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

struct Temporary(PathBuf);

impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub fn write(path: &Path, bytes: &[u8], overwrite: bool) -> io::Result<()> {
    let metadata = if overwrite {
        match fs::metadata(path) {
            Ok(metadata) => Some(metadata),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        }
    } else {
        None
    };
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let (temporary, mut file) = (0..128)
        .find_map(|_| {
            let temp = parent.join(format!(
                ".miniluar-{}-{}.tmp",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            // Set the mode at creation, before source bytes or a readable file
            // descriptor can escape. New outputs deliberately remain private.
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&temp) {
                Ok(file) => Some(Ok((Temporary(temp), file))),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => None,
                Err(e) => Some(Err(e)),
            }
        })
        .unwrap_or_else(|| {
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "cannot allocate temporary output",
            ))
        })?;
    file.write_all(bytes)?;
    if let Some(metadata) = metadata {
        restore_metadata(&file, &metadata)?;
    }
    // Surface delayed write failures before publishing. This is not a guarantee
    // of crash durability, which would also require syncing the parent directory.
    file.sync_all()?;
    drop(file);
    if overwrite {
        fs::rename(&temporary.0, path)?;
    } else {
        // hard_link is an atomic no-clobber operation, unlike exists()+rename().
        // Both paths are in the same directory and therefore on the same device.
        fs::hard_link(&temporary.0, path)?;
    }
    Ok(())
}

fn restore_metadata(file: &fs::File, metadata: &fs::Metadata) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, fchown};
        let staged = file.metadata()?;
        if staged.uid() != metadata.uid() || staged.gid() != metadata.gid() {
            // A mode has meaning only with the original owner and group. If we
            // cannot retain them, leave the destination intact rather than
            // publish source with a different set of readers.
            fchown(file, Some(metadata.uid()), Some(metadata.gid()))?;
        }
    }
    // Changing ownership may clear set-ID bits, so restore the mode afterward.
    file.set_permissions(metadata.permissions())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    #[test]
    fn ownership_is_restored_or_rejected_before_permissions_are_widened() {
        let path = std::env::temp_dir().join(format!(
            "miniluar-ownership-test-{}.tmp",
            std::process::id()
        ));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        let _temporary = Temporary(path);
        // Usually root-owned and inaccessible to an unprivileged chown. This
        // metadata can be restored successfully in privileged test runs.
        let expected = fs::metadata("/").unwrap();
        match restore_metadata(&file, &expected) {
            Ok(()) => {
                let actual = file.metadata().unwrap();
                assert_eq!(actual.uid(), expected.uid());
                assert_eq!(actual.gid(), expected.gid());
                assert_eq!(actual.mode() & 0o7777, expected.mode() & 0o7777);
            }
            Err(error) => {
                // Sandboxed user namespaces can report unmapped IDs as invalid
                // rather than deny permission to change ownership.
                assert!(
                    matches!(
                        error.kind(),
                        io::ErrorKind::PermissionDenied | io::ErrorKind::InvalidInput
                    ),
                    "unexpected ownership restoration error: {error}"
                );
                assert_eq!(file.metadata().unwrap().mode() & 0o077, 0);
            }
        }
    }
}
