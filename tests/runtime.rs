use std::io::Write;
use std::process::{Command, Output, Stdio};

use miniluar::minify;

fn execute(source: &[u8]) -> Output {
    let mut child = Command::new("lua")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Lua is required for execution tests; use nix develop");
    child.stdin.take().unwrap().write_all(source).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn compacted_programs_preserve_known_results_in_lua() {
    for (source, expected) in [
        (b"local x=3; print(x - -2)".as_slice(), b"5\n".as_slice()),
        (b"print('a' .. .5)", b"a0.5\n"),
        (b"local t={x=7}; print(t[ [[x]] ])", b"7\n"),
        (b"local x <const> = 7; print(x)", b"7\n"),
        (b"local s='\xff'; print(#s, string.byte(s))", b"1\t255\n"),
        (
            b"print(math.type(1.0), math.type(1), 1/(-0.0))",
            b"float\tinteger\t-inf\n",
        ),
        (
            b"local s=[=[\r\na\r\nb]=]; print(#s, s:byte(2))",
            b"3\t10\n",
        ),
    ] {
        for program in [source.to_vec(), minify(source).unwrap()] {
            let output = execute(&program);
            assert!(output.status.success(), "{program:?}: {:?}", output.stderr);
            assert_eq!(output.stdout, expected, "{program:?}");
        }
    }
}

#[test]
fn removing_a_required_separator_changes_execution() {
    let compacted = minify(b"local x=3; print(x - -2)").unwrap();
    let valid = execute(&compacted);
    assert!(valid.status.success());
    assert_eq!(valid.stdout, b"5\n");
    let broken = execute(b"local x=3;print(x--2)\n");
    assert!(!broken.status.success());
    assert!(broken.stdout.is_empty());
}
