use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::Result;
use super::args::Config;
use super::paths::{self, FileId};

pub struct Job {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub replacing_source: bool,
}

pub fn plan(inputs: Vec<PathBuf>, config: &Config) -> Result<Vec<Job>> {
    if config.output.is_some() && inputs.len() != 1 {
        return Err("--output requires exactly one input file".into());
    }
    let jobs = inputs
        .into_iter()
        .map(|input| {
            let output = destination(&input, config);
            Job {
                input,
                output,
                replacing_source: false,
            }
        })
        .collect::<Vec<_>>();
    if config.stdout {
        return Ok(jobs);
    }
    preflight(jobs, config.in_place)
}

fn destination(input: &Path, config: &Config) -> Option<PathBuf> {
    if config.stdout {
        return None;
    }
    if let Some(output) = &config.output {
        return Some(output.clone());
    }
    if config.in_place {
        return Some(input.into());
    }
    let mut stem = input.file_stem().unwrap_or_default().to_os_string();
    if let Some(name) = stem.to_str().and_then(|s| s.strip_suffix(".min")) {
        stem = name.into();
    }
    stem.push(&config.suffix);
    Some(
        config
            .output_dir
            .as_ref()
            .map_or_else(|| input.with_file_name(&stem), |dir| dir.join(&stem)),
    )
}

fn preflight(mut jobs: Vec<Job>, in_place: bool) -> Result<Vec<Job>> {
    let mut sources = HashMap::new();
    let mut source_ids = HashMap::new();
    for (index, job) in jobs.iter().enumerate() {
        sources.insert(job.input.clone(), index);
        if let Some(id) = FileId::read(&job.input)? {
            source_ids.insert(id, index);
        }
    }
    let mut outputs = HashSet::new();
    let mut output_ids = HashSet::new();
    for (index, job) in jobs.iter_mut().enumerate() {
        let Some(destination) = &job.output else {
            continue;
        };
        let resolved = paths::resolve(destination)?;
        let identity = FileId::read(destination)?;
        job.replacing_source = in_place && resolved == job.input;
        let source = sources
            .get(&resolved)
            .or_else(|| identity.as_ref().and_then(|id| source_ids.get(id)));
        if source.is_some_and(|&source| source != index || !job.replacing_source) {
            return Err(format!(
                "output collision: {} aliases an input",
                destination.display()
            )
            .into());
        }
        if !outputs.insert(resolved) || identity.is_some_and(|id| !output_ids.insert(id)) {
            return Err(format!(
                "output collision: {} has multiple sources",
                destination.display()
            )
            .into());
        }
    }
    Ok(jobs)
}
