# Development

Run these checks from the repository root:

```sh
nix develop
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Execution tests require Lua 5.4, supplied by the shell. They fail rather than skip
when Lua is absent. Luau interpolation currently has lexical tests only.

`nix build` builds the CLI and runs tests. `nix run . -- --help` lists implemented
options. For a quick smoke test:

```sh
printf 'print ( 42 ) -- comment\n' | cargo run --quiet -- -
```

The minifier handles bytes, not UTF-8 text. Keep literal spelling intact until a
transformation has runtime tests for the supported dialects. Lexical errors fail
before output is written; a successful minification is not syntax validation.
The crate forbids unsafe code and has no third-party Rust dependencies.

Take the first item in `.agents/TODO.md` and delete it when finished. The Python
reference and its regression tests live in `../fork-MiniLua`. Tests should prove
observable behavior, including cases where a tempting transformation is wrong.
