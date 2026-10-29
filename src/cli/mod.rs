mod args;
mod discover;
mod paths;
mod plan;

use std::ffi::OsString;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};

use crate::output;
use args::{Action, Config};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn execute(arguments: impl Iterator<Item = OsString>) -> Result<()> {
    match args::parse(arguments)? {
        Action::Help => io::stdout().lock().write_all(args::HELP.as_bytes())?,
        Action::Version => writeln!(io::stdout().lock(), "minilua {}", env!("CARGO_PKG_VERSION"))?,
        Action::Run(config) => run(config)?,
    }
    Ok(())
}

fn run(config: Config) -> Result<()> {
    if config.inputs.is_empty() || config.inputs[0].as_os_str() == "-" {
        return stdin(&config);
    }
    let inputs = discover::discover(&config.inputs, config.recursive)?;
    let jobs = plan::plan(inputs, &config)?;
    let mut failures = 0;
    for job in jobs {
        if let Err(error) = file(&job, &config) {
            eprintln!("minilua: {}: {error}", job.input.display());
            failures += 1;
        }
    }
    if failures > 0 {
        return Err(format!("{failures} input(s) failed").into());
    }
    Ok(())
}

fn stdin(config: &Config) -> Result<()> {
    if io::stdin().is_terminal() {
        return Err("no input provided; pass a file or pipe source through stdin".into());
    }
    let mut source = Vec::new();
    io::stdin().read_to_end(&mut source)?;
    let minified = miniluar::minify_with_options(&source, config.minify)?;
    publish(
        &source,
        &minified,
        config.output.as_deref(),
        config.overwrite,
        config.quiet,
    )
}

fn file(job: &plan::Job, config: &Config) -> Result<()> {
    let source = fs::read(&job.input).map_err(|e| format!("cannot read: {e}"))?;
    let minified = miniluar::minify_with_options(&source, config.minify)?;
    publish(
        &source,
        &minified,
        job.output.as_deref(),
        config.overwrite || job.replacing_source,
        config.quiet,
    )
}

fn publish(
    source: &[u8],
    minified: &[u8],
    destination: Option<&std::path::Path>,
    overwrite: bool,
    quiet: bool,
) -> Result<()> {
    if let Some(path) = destination {
        output::write(path, minified, overwrite)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        if !quiet {
            eprintln!(
                "{}: {} -> {} bytes",
                path.display(),
                source.len(),
                minified.len()
            );
        }
    } else {
        io::stdout().lock().write_all(minified)?;
    }
    Ok(())
}
