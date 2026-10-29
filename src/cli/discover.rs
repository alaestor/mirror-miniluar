use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::Result;
use super::paths::{self, FileId};

pub fn discover(inputs: &[PathBuf], recursive: bool) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut paths = HashSet::new();
    let mut identities = HashSet::new();
    for input in inputs {
        let candidates = if input.is_dir() {
            directory(input, recursive)?
        } else {
            // Keep missing literal paths so execution reports the read error
            // and can still process the rest of the batch.
            vec![input.clone()]
        };
        for candidate in candidates {
            let path = paths::resolve(&candidate)?;
            if !paths.insert(path.clone()) {
                continue;
            }
            if let Some(id) = FileId::read(&path)?
                && !identities.insert(id)
            {
                continue;
            }
            found.push(path);
        }
    }
    if found.is_empty() {
        return Err("No Lua/Luau files found".into());
    }
    Ok(found)
}

fn directory(root: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut sources = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let path = entry.path();
            if kind.is_dir() && recursive {
                pending.push(path);
            } else if (kind.is_file() || (kind.is_symlink() && path.is_file()))
                && path
                    .extension()
                    .is_some_and(|ext| ext == "lua" || ext == "luau")
            {
                sources.push(path);
            }
        }
    }
    sources.sort();
    Ok(sources)
}
