#![forbid(unsafe_code)]

mod output;

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "Usage: minilua [OPTIONS] [FILE|-]

Minify Lua or Luau source. With no FILE, read stdin and write stdout.
Files default to a sibling named <stem>.min.lua. Literal spelling is preserved.

Options:
  -o, --output FILE  Write to FILE instead of the default destination
      --stdout       Write file results to stdout
  -f, --overwrite    Replace an existing output file, never the input
  -q, --quiet        Suppress file statistics
      --no-banner    Accepted for compatibility; no banner is printed
  -h, --help         Print help
  -v, --version      Print version
      --             Treat remaining arguments as filenames
";

#[derive(Default)]
struct Config {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    stdout: bool,
    overwrite: bool,
    quiet: bool,
}

enum Action {
    Run(Config),
    Help,
    Version,
}

fn parse(args: impl Iterator<Item = OsString>) -> Result<Action, String> {
    let mut config = Config::default();
    let mut args = args.peekable();
    let mut positional = false;
    while let Some(arg) = args.next() {
        if !positional {
            match arg.to_str() {
                Some("--") => {
                    positional = true;
                    continue;
                }
                Some("-h" | "--help") => return Ok(Action::Help),
                Some("-v" | "--version") => return Ok(Action::Version),
                Some("--stdout") => {
                    config.stdout = true;
                    continue;
                }
                Some("-f" | "--overwrite") => {
                    config.overwrite = true;
                    continue;
                }
                Some("-q" | "--quiet") => {
                    config.quiet = true;
                    continue;
                }
                Some("--no-banner") => continue,
                Some("-o" | "--output") => {
                    let value = args.next().ok_or("--output requires a file path")?;
                    if config.output.replace(value.into()).is_some() {
                        return Err("--output may only be specified once".into());
                    }
                    continue;
                }
                _ if arg.as_encoded_bytes().starts_with(b"-") && arg != "-" => {
                    return Err(format!("unknown option: {}", arg.to_string_lossy()));
                }
                _ => {}
            }
        }
        if config.input.replace(arg.into()).is_some() {
            return Err("only one input is currently supported".into());
        }
    }
    if config.stdout && config.output.is_some() {
        return Err("--stdout and --output cannot be combined".into());
    }
    Ok(Action::Run(config))
}

fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let input = config.input.filter(|p| p.as_os_str() != "-");
    let mut source = Vec::new();
    if let Some(path) = &input {
        source = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    } else {
        if io::stdin().is_terminal() {
            return Err("no input provided; pass a file or pipe source through stdin".into());
        }
        io::stdin().read_to_end(&mut source)?;
    }
    let minified = miniluar::minify(&source)?;
    let destination = if config.stdout {
        None
    } else {
        config.output.or_else(|| {
            input.as_ref().map(|path| {
                let mut stem = path.file_stem().unwrap_or_default().to_os_string();
                if let Some(name) = stem.to_str().and_then(|s| s.strip_suffix(".min")) {
                    stem = name.into();
                }
                stem.push(".min.lua");
                path.with_file_name(stem)
            })
        })
    };
    if let Some(path) = destination {
        if let Some(input) = &input
            && output::same_file(input, &path)?
        {
            return Err("output aliases the input; refusing to replace source".into());
        }
        output::write(&path, &minified, config.overwrite)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        if !config.quiet {
            eprintln!(
                "{}: {} -> {} bytes",
                path.display(),
                source.len(),
                minified.len()
            );
        }
    } else {
        io::stdout().lock().write_all(&minified)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let result = match parse(env::args_os().skip(1)) {
        Ok(Action::Help) => io::stdout()
            .lock()
            .write_all(HELP.as_bytes())
            .map_err(Into::into),
        Ok(Action::Version) => {
            writeln!(io::stdout().lock(), "minilua {}", env!("CARGO_PKG_VERSION"))
                .map_err(Into::into)
        }
        Ok(Action::Run(config)) => run(config),
        Err(error) => Err(error.into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("minilua: {error}");
            ExitCode::FAILURE
        }
    }
}
