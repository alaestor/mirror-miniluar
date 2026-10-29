#[path = "support/runtime.rs"]
mod runtime;

use miniluar::{Options, minify, minify_with_options};

#[test]
fn portable_strings_keep_known_byte_values_on_every_runtime() {
    for runtime in runtime::all() {
        for (literal, expected) in [
            (br#""\255""#.as_slice(), b"1\n255\n".as_slice()),
            (br#""\0012""#, b"2\n1\n50\n"),
            (br#""\0009""#, b"2\n0\n57\n"),
            ("\"é\"".as_bytes(), b"2\n195\n169\n"),
            (b"[=[\r\na\r\nb]=]", b"3\n97\n10\n98\n"),
        ] {
            let mut source = b"local s=".to_vec();
            source.extend(literal);
            source.extend(b";print(#s);for i=1,#s do print(string.byte(s,i)) end");
            assert_eq!(runtime.stdout(&source), expected, "{}", runtime.name);
            assert_eq!(
                runtime.stdout(&minify(&source).unwrap()),
                expected,
                "{}",
                runtime.name
            );
        }
    }
}

#[test]
fn numbers_preserve_values_types_overflow_and_negative_zero() {
    for runtime in runtime::all() {
        for literal in [
            "1.0",
            "0.0",
            "-0.0",
            "1e309",
            "1e-999",
            "1e+03",
            "0.5000",
            "9007199254740993.0",
            "9223372036854775807",
            "9223372036854775808",
            "0009223372036854775807",
            "0xFFFFFFFFFFFFFFFF",
            "1.000e+003",
        ] {
            let source = format!(
                "local n={literal};print(n,tostring(n),1/n);if math.type then print(math.type(n)) end"
            );
            assert_eq!(
                runtime.stdout(&minify(source.as_bytes()).unwrap()),
                runtime.stdout(source.as_bytes()),
                "{}: {literal}",
                runtime.name
            );
        }
        if !matches!(runtime.name, "LUA51" | "LUAU") {
            for literal in ["0x1.8p+1", "0x1p-1074", "0x1.fp-100"] {
                let source = format!("local n={literal};print(n,1/n)");
                assert_eq!(
                    runtime.stdout(&minify(source.as_bytes()).unwrap()),
                    runtime.stdout(source.as_bytes())
                );
            }
        }
    }
}

#[test]
fn dialect_sensitive_escapes_and_long_newlines_retain_their_runtime_behavior() {
    for runtime in runtime::all() {
        for literal in [
            br#""\x41""#.as_slice(),
            b"[[\ra\rb]]",
            b"[[a\n\rb]]",
            br#""a\n\t\\\"b""#,
            br#""\128\255""#,
            br#""\1274""#,
        ] {
            let mut source = b"local s=".to_vec();
            source.extend(literal);
            source.extend(b";print(#s);for i=1,#s do print(string.byte(s,i)) end");
            assert_eq!(
                runtime.stdout(&minify(&source).unwrap()),
                runtime.stdout(&source),
                "{}: {literal:?}",
                runtime.name
            );
        }
    }
}

#[test]
fn preserved_comments_do_not_change_execution_on_any_runtime() {
    let options = Options {
        remove_comments: false,
        ..Options::default()
    };
    for runtime in runtime::all() {
        for (source, expected) in [
            (b"print(1 - --[[comment]] 2)".as_slice(), b"-1\n".as_slice()),
            (b"print(1 - -- comment\n 2)", b"-1\n"),
            (b"print(1 - --[=[comment]=] -2)", b"3\n"),
        ] {
            assert_eq!(
                runtime.stdout(&minify_with_options(source, options).unwrap()),
                expected
            );
        }
    }
}

#[test]
fn luau_binary_literals_numeric_separators_and_interpolation_execute() {
    let runtime = runtime::all()
        .into_iter()
        .find(|r| r.name == "LUAU")
        .unwrap();
    for (source, expected) in [
        ("print(0b1_010)", "10\n"),
        ("print(1e1_0)", "10000000000\n"),
        ("print(1e-0_1)", "0.1\n"),
        ("print(0xF_F)", "255\n"),
        ("local x=3;print(`answer {x+1}`)", "answer 4\n"),
    ] {
        assert_eq!(runtime.stdout(source.as_bytes()), expected.as_bytes());
        assert_eq!(
            runtime.stdout(&minify(source.as_bytes()).unwrap()),
            expected.as_bytes()
        );
    }
}

#[test]
fn generated_long_strings_cannot_close_inside_the_final_content_bytes() {
    for runtime in runtime::all() {
        for ending in ["]", "]=", "]]", "]=]", "]==", "\\n"] {
            let source = format!(
                r#"local s="a\nb\nc\nd\ne\nf{ending}";print(#s);for i=1,#s do print(string.byte(s,i)) end"#
            );
            assert_eq!(
                runtime.stdout(&minify(source.as_bytes()).unwrap()),
                runtime.stdout(source.as_bytes()),
                "{}: {ending:?}",
                runtime.name
            );
        }
    }
}
