# Development

Run these checks from the repository root:

```sh
nix develop
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Execution tests require Lua 5.1–5.4 and Luau. The shell supplies interpreter paths
through `LUA51`, `LUA52`, `LUA53`, `LUA54`, and `LUAU`. Missing runtimes fail tests
rather than silently skipping them.

`nix build` builds the CLI and runs tests. `nix run . -- --help` lists implemented
options. For a quick smoke test:

```sh
printf 'print ( 42 ) -- comment\n' | cargo run --quiet -- -
```

The minifier handles bytes, not UTF-8 text. Literal transformations need runtime
tests for every supported dialect. Use `--no-strings --no-numbers` when exact
literal spelling matters. Lexical errors fail before output is written; a
successful minification is not syntax validation.
Local renaming is opt-in (`--rename-locals`). Its binding parser supports untyped
Lua/Luau; types, casts, if-expressions, and backtick strings fail before output.
Names inspected through debugging or dynamically loaded code cannot be inferred;
leave renaming disabled, or use the library's `preserve_names` option.
The crate forbids unsafe code and has no third-party Rust dependencies.

Take the first item in `.agents/TODO.md` and delete it when finished. The Python
reference and its regression tests live in `../fork-MiniLua`. Tests should prove
observable behavior, including cases where a tempting transformation is wrong.
