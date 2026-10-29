use std::ffi::OsString;
use std::path::PathBuf;

pub const HELP: &str = "Usage: minilua [OPTIONS] [FILE|DIR ...|-]

Minify Lua or Luau source. With no input, read stdin and write stdout.
Files default to siblings named <stem>.min.lua. Literal spelling is preserved.
Globs are expanded by your shell, not by minilua. Directory results are sorted.
Directory symlinks are not traversed during recursive discovery.

Options:
  -o, --output FILE      Write one source to FILE
  -d, --output-dir DIR   Write batch results into DIR
      --suffix SUFFIX   Output suffix, default .min.lua
  -i, --in-place         Replace each input atomically
  -r, --recursive        Include nested directories
      --stdout           Write results to stdout instead of files
  -f, --overwrite        Replace existing outputs
  -q, --quiet            Suppress file statistics
      --no-comments      Keep comments
      --no-whitespace    Keep original whitespace
      --keep-newlines    Keep newline bytes while removing spaces
      --no-banner        Accepted for compatibility; no banner is printed
  -h, --help             Print help
  -v, --version          Print version
      --                 Treat remaining arguments as filenames
";

pub struct Config {
    pub minify: miniluar::Options,
    pub inputs: Vec<PathBuf>,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub suffix: OsString,
    pub in_place: bool,
    pub recursive: bool,
    pub stdout: bool,
    pub overwrite: bool,
    pub quiet: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            minify: miniluar::Options::default(),
            inputs: Vec::new(),
            output: None,
            output_dir: None,
            suffix: ".min.lua".into(),
            in_place: false,
            recursive: false,
            stdout: false,
            overwrite: false,
            quiet: false,
        }
    }
}

pub enum Action {
    Run(Config),
    Help,
    Version,
}

pub fn parse(args: impl Iterator<Item = OsString>) -> Result<Action, String> {
    let mut config = Config::default();
    let mut args = args.peekable();
    let mut positional = false;
    while let Some(arg) = args.next() {
        if positional || !arg.as_encoded_bytes().starts_with(b"-") || arg == "-" {
            config.inputs.push(arg.into());
            continue;
        }
        match arg.to_str() {
            Some("--") => positional = true,
            Some("-h" | "--help") => return Ok(Action::Help),
            Some("-v" | "--version") => return Ok(Action::Version),
            Some("--stdout") => config.stdout = true,
            Some("-f" | "--overwrite") => config.overwrite = true,
            Some("-q" | "--quiet") => config.quiet = true,
            Some("-i" | "--in-place") => config.in_place = true,
            Some("-r" | "--recursive") => config.recursive = true,
            Some("--no-banner") => {}
            Some("--no-comments") => config.minify.remove_comments = false,
            Some("--no-whitespace") => config.minify.remove_whitespace = false,
            Some("--keep-newlines") => config.minify.keep_newlines = true,
            Some("-o" | "--output" | "-d" | "--output-dir" | "--suffix") => {
                let value = args
                    .next()
                    .ok_or_else(|| format!("{} requires a value", arg.to_string_lossy()))?;
                match arg.to_str() {
                    Some("-o" | "--output") => set_path(&mut config.output, value, "--output")?,
                    Some("-d" | "--output-dir") => {
                        set_path(&mut config.output_dir, value, "--output-dir")?
                    }
                    _ => config.suffix = value,
                }
            }
            _ => return Err(format!("unknown option: {}", arg.to_string_lossy())),
        }
    }
    validate(&config)?;
    Ok(Action::Run(config))
}

fn set_path(slot: &mut Option<PathBuf>, value: OsString, option: &str) -> Result<(), String> {
    if slot.replace(value.into()).is_some() {
        return Err(format!("{option} may only be specified once"));
    }
    Ok(())
}

fn validate(config: &Config) -> Result<(), String> {
    if config.stdout && (config.output.is_some() || config.output_dir.is_some() || config.in_place)
    {
        return Err("--stdout cannot be combined with file output options".into());
    }
    if config.output_dir.is_some() && (config.output.is_some() || config.in_place) {
        return Err("--output-dir cannot be combined with --output or --in-place".into());
    }
    let stdin = config.inputs.is_empty() || config.inputs.iter().any(|p| p.as_os_str() == "-");
    if stdin && config.inputs.len() > 1 {
        return Err("stdin cannot be combined with file inputs".into());
    }
    if stdin && (config.in_place || config.output_dir.is_some()) {
        return Err("stdin requires --output or stdout, not --in-place or --output-dir".into());
    }
    if config.suffix.is_empty()
        || config
            .suffix
            .as_encoded_bytes()
            .iter()
            .any(|b| matches!(b, b'/' | b'\\'))
    {
        return Err("--suffix must be a nonempty filename suffix without separators".into());
    }
    Ok(())
}
