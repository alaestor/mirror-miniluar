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
    if overwrite {
        match fs::metadata(path) {
            Ok(metadata) => file.set_permissions(metadata.permissions())?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
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
