mod support;

use std::fs;
use support::{Sandbox, success};
#[test]
fn stdin_is_byte_preserving_and_stdout_contains_only_source() {
    let dir = Sandbox::new();
    let output = dir.run(&[], b"local s = '\xff' -- remove\nprint(s)");
    success(&output);
    assert_eq!(output.stdout, b"local s='\xff'print(s)\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn lexical_failure_produces_no_partial_stdout_or_output_file() {
    let dir = Sandbox::new();
    let output = dir.run(&["-", "-o", "out.lua"], b"print(1); 'unfinished");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unfinished string"));
    assert!(!dir.path("out.lua").exists());
}

#[test]
fn file_mode_writes_a_sibling_without_modifying_the_input() {
    let dir = Sandbox::new();
    fs::write(dir.path("input.lua"), b"print ( 1 )").unwrap();
    let result = dir.run(&["input.lua", "-q"], b"");
    success(&result);
    assert!(result.stdout.is_empty());
    assert_eq!(fs::read(dir.path("input.lua")).unwrap(), b"print ( 1 )");
    assert_eq!(fs::read(dir.path("input.min.lua")).unwrap(), b"print(1)\n");
}

#[test]
fn existing_outputs_require_explicit_overwrite() {
    let dir = Sandbox::new();
    fs::write(dir.path("output.lua"), b"precious").unwrap();
    let result = dir.run(&["-o", "output.lua"], b"print ( 1 )");
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert_eq!(fs::read(dir.path("output.lua")).unwrap(), b"precious");
    let result = dir.run(&["-o", "output.lua", "-f"], b"print ( 1 )");
    success(&result);
    assert_eq!(fs::read(dir.path("output.lua")).unwrap(), b"print(1)\n");
    assert_eq!(
        fs::read_dir(&dir.0).unwrap().count(),
        1,
        "temporary files leaked"
    );
}

#[test]
fn stdout_mode_does_not_create_a_default_output() {
    let dir = Sandbox::new();
    fs::write(dir.path("input.lua"), b"print ( 1 )").unwrap();
    let result = dir.run(&["input.lua", "--stdout"], b"");
    success(&result);
    assert_eq!(result.stdout, b"print(1)\n");
    assert!(!dir.path("input.min.lua").exists());
}

#[test]
fn invalid_arguments_fail_without_consuming_or_writing_source() {
    let dir = Sandbox::new();
    for args in [
        vec!["--unknown"],
        vec!["-o"],
        vec!["--stdout", "-o", "out.lua"],
        vec!["one.lua", "two.lua"],
    ] {
        let result = dir.run(&args, b"");
        assert!(!result.status.success(), "{args:?}");
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 0);
}

#[test]
fn explicit_output_cannot_replace_the_input_even_with_force() {
    let dir = Sandbox::new();
    let original = b"print ( 1 )";
    fs::write(dir.path("input.lua"), original).unwrap();
    let result = dir.run(&["input.lua", "-o", "./input.lua", "-f"], b"");
    assert!(!result.status.success());
    assert_eq!(fs::read(dir.path("input.lua")).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn symlink_and_hardlink_aliases_cannot_replace_the_input() {
    let dir = Sandbox::new();
    fs::write(dir.path("input.lua"), b"print ( 1 )").unwrap();
    std::os::unix::fs::symlink("input.lua", dir.path("sym.lua")).unwrap();
    fs::hard_link(dir.path("input.lua"), dir.path("hard.lua")).unwrap();
    for alias in ["sym.lua", "hard.lua"] {
        let result = dir.run(&["input.lua", "-o", alias, "-f"], b"");
        assert!(!result.status.success());
        assert_eq!(fs::read(dir.path(alias)).unwrap(), b"print ( 1 )");
    }
}

#[test]
fn double_dash_allows_option_shaped_filenames() {
    let dir = Sandbox::new();
    fs::write(dir.path("-input.lua"), b"return 1").unwrap();
    let result = dir.run(&["--stdout", "--", "-input.lua"], b"");
    success(&result);
    assert_eq!(result.stdout, b"return 1\n");
}

#[test]
fn help_and_version_do_not_require_source() {
    let dir = Sandbox::new();
    let help = dir.run(&["--help"], b"");
    success(&help);
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage:"));
    let version = dir.run(&["--version"], b"");
    success(&version);
    assert_eq!(version.stdout, b"minilua 0.1.0\n");
}

#[test]
fn preservation_flags_are_applied_in_both_stdin_and_file_modes() {
    let dir = Sandbox::new();
    let source = b"-- heading\r\nprint ( 1 )\r\n\r\n";
    fs::write(dir.path("input.lua"), source).unwrap();
    for input in ["-", "input.lua"] {
        let result = dir.run(
            &[input, "--stdout", "--no-comments", "--no-whitespace"],
            source,
        );
        success(&result);
        assert_eq!(result.stdout, source);
        let result = dir.run(&[input, "--stdout", "--keep-newlines"], source);
        success(&result);
        assert_eq!(result.stdout, b"\r\nprint(1)\r\n\r\n");
    }
}
