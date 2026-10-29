#[path = "support/runtime.rs"]
mod runtime;
mod support;

use std::fs;
use support::{Sandbox, success};

#[test]
fn file_and_stdout_modes_preserve_literal_bytes_on_every_runtime() {
    let dir = Sandbox::new();
    for (literal, expected) in [
        (b"'\xff'".as_slice(), b"1\t255\n".as_slice()),
        (b"[[\xff]]", b"1\t255\n"),
        (b"'\xc3\xa9'", b"2\t195\t169\n"),
    ] {
        let mut source = b"local value=".to_vec();
        source.extend(literal);
        source.extend(b";print(#value,value:byte(1,#value))");
        fs::write(dir.path("input.lua"), &source).unwrap();
        for mode in ["--stdout", "--in-place", "--output"] {
            let args = if mode == "--output" {
                vec!["input.lua", mode, "out.lua", "-f", "-q"]
            } else {
                vec!["input.lua", mode, "-q"]
            };
            let result = dir.run(&args, b"");
            success(&result);
            let output = match mode {
                "--stdout" => result.stdout,
                "--in-place" => fs::read(dir.path("input.lua")).unwrap(),
                _ => fs::read(dir.path("out.lua")).unwrap(),
            };
            for runtime in runtime::all() {
                assert_eq!(runtime.stdout(&output), expected);
            }
            fs::write(dir.path("input.lua"), &source).unwrap();
        }
    }
}

#[cfg(unix)]
#[test]
fn in_place_minification_keeps_a_script_directly_executable() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    let dir = Sandbox::new();
    let interpreter = std::env::var_os("LUA54").expect("use nix develop");
    let mut source = b"#!".to_vec();
    source.extend(interpreter.as_encoded_bytes());
    source.extend(b"\n print ( 42, '#!/literal' ) -- strip\n");
    let path = dir.path("script.lua");
    fs::write(&path, source).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let original = Command::new(&path).output().unwrap();
    success(&original);
    assert_eq!(original.stdout, b"42\t#!/literal\n");
    success(&dir.run(&["script.lua", "-i", "-q"], b""));
    let output = Command::new(&path).output().unwrap();
    success(&output);
    assert_eq!(output.stdout, original.stdout);
}

#[test]
fn shebang_terminates_before_code_even_when_comments_are_kept() {
    for runtime in runtime::all() {
        for remove_comments in [false, true] {
            let output = miniluar::minify_with_options(
                b"#!/usr/bin/lua\r\n print ( 42 )",
                miniluar::Options {
                    remove_comments,
                    ..miniluar::Options::default()
                },
            )
            .unwrap();
            assert!(output.starts_with(b"#!/usr/bin/lua\n"));
            assert_eq!(runtime.stdout(&output), b"42\n");
        }
    }
}
