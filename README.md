<div align="center">

```
   __    __     __     __   __     __     __         __  __     ______     ______    
  /\ "-./  \   /\ \   /\ "-.\ \   /\ \   /\ \       /\ \/\ \   /\  __ \   /\  == \   
  \ \ \-./\ \  \ \ \  \ \ \-.  \  \ \ \  \ \ \____  \ \ \_\ \  \ \  __ \  \ \  __<   
   \ \_\ \ \_\  \ \_\  \ \_\\"\_\  \ \_\  \ \_____\  \ \_____\  \ \_\ \_\  \ \_\ \_\ 
    \/_/  \/_/   \/_/   \/_/ \/_/   \/_/   \/_____/   \/_____/   \/_/\/_/   \/_/ /_/ 
```

**A Lua / Luau source minifier, rewritten in Rust**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?logo=rust&logoColor=white)](https://rust-lang.org/)
[![Nix](https://img.shields.io/badge/NIX-%235277C3.svg?logo=NixOS&logoColor=white)](https://nixos.org/)
[![License MIT](https://img.shields.io/badge/LICENSE-MIT-GREEN)](LICENSE)
![Hallucinated Slop](https://img.shields.io/badge/%E2%80%94_Hallucinated%20Slop_%E2%80%94-red)

</div>

MiniLuar is a Rust rewrite of the missing Python [github:aradooo/MiniLua](https://github.com/aradooo/MiniLua) ([reupload](https://git.0x04.cc/alaestor/fork-MiniLua)). The executable is still called `minilua`.

It strips comments and unnecessary whitespace, shortens portable string and number literals, and optionally renames local variables. Source is handled as bytes, without requiring UTF-8.

Targets Lua 5.1~5.4 and Luau. Test minified output with your target runtime. Ordinary minification checks lexical completeness, not the full language grammar; `--rename-locals` rejects unsupported binding syntax rather than guessing.

## Build

With Rust and Cargo installed:

```sh
cargo install --path .
```

Or use Nix flakes from the checkout:

```sh
nix run . -- script.lua --stdout
nix build 
./result/bin/minilua --help
```

## Usage

```sh
minilua script.lua                         # writes script.min.lua
minilua script.lua -o dist/script.lua
minilua src/ -r -d dist/                   # all .lua and .luau files
minilua script.lua --stdout
minilua - --stdout < script.lua
minilua script.lua --rename-locals
```

Existing outputs require `--overwrite`. Use `--in-place` to replace source files. Batch output directories are flat; filename collisions are rejected before writing. Your shell expands globs.

Use `--no-comments`, `--no-whitespace`, `--no-strings`, or `--no-numbers` to disable individual transformations. `--keep-newlines` retains source newline bytes. Diagnostics and file statistics go to stderr. See `minilua --help` for all options.

## Library

```rust
let output = miniluar::minify(b"local value = 0.500 -- comment\n")?;
assert_eq!(output, b"local value=.5\n");
```

Use `minify_with_options` and `Options` to configure transformations.

## Development

Build and run the tests with Lua 5.1~5.4 and Luau:

```sh
nix flake check
```

Or run Cargo tests in the development shell:

```sh
nix develop
cargo test
```

Runtime tests skip unavailable Lua or Luau interpreters. The Nix development shell supplies Lua 5.1~5.4 and Luau for those tests.
