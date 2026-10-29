//! Stage complete output beside its destination, then publish without truncating it.
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub fn same_file(input: &Path, output: &Path) -> io::Result<bool> {
    let output_meta = match fs::metadata(output) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let input_meta = fs::metadata(input)?;
        Ok(input_meta.dev() == output_meta.dev() && input_meta.ino() == output_meta.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = output_meta;
        Ok(fs::canonicalize(input)? == fs::canonicalize(output)?)
    }
}

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
            match OpenOptions::new().write(true).create_new(true).open(&temp) {
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
