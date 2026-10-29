use miniluar::{Options, minify_with_options};

fn literal(source: &[u8], options: Options) -> Vec<u8> {
    let mut program = b"local x=".to_vec();
    program.extend(source);
    let compacted = minify_with_options(&program, options).unwrap();
    compacted[b"local x=".len()..compacted.len() - 1].to_vec()
}

#[test]
fn numeric_shortening_uses_spelling_not_floating_point_round_trips() {
    for (source, expected) in [
        ("0.5000", ".5"),
        ("1.000e+003", "1e3"),
        ("000123", "123"),
        ("1.0", "1."),
        ("000e+000", "0e0"),
        ("0xF_F", "0xFF"),
        ("0b1_010", "0b1010"),
        ("9007199254740993.000", "9007199254740993."),
    ] {
        assert_eq!(
            literal(source.as_bytes(), Options::default()),
            expected.as_bytes()
        );
    }
}

#[test]
fn disabling_numeric_shortening_keeps_the_original_spelling() {
    let options = Options {
        minify_numbers: false,
        ..Options::default()
    };
    for source in [b"0.5000".as_slice(), b"1.000e+003", b"000123", b"0b1_010"] {
        assert_eq!(literal(source, options), source);
    }
}

#[test]
fn portable_decimal_escapes_shorten_without_utf8_conversion() {
    for (source, expected) in [
        (br#""\065\066\067""#.as_slice(), br#""ABC""#.as_slice()),
        (br#""\255""#, b"\"\xff\""),
        (br#""\\x\065\066\067""#, br#""\\xABC""#),
        (b"[[hello]]", b"\"hello\""),
    ] {
        assert_eq!(literal(source, Options::default()), expected);
    }
}

#[test]
fn unsupported_or_dialect_sensitive_string_escapes_are_untouched() {
    for source in [
        br#""\256""#.as_slice(),
        br#""\xGG""#,
        br#""\u{}""#,
        br#""\u{110000}""#,
        br#""\q""#,
        br#""\u{D800}""#,
        br#""\x41""#,
        br#""a\z  b""#,
        b"[[\ra\rb]]",
        b"[[a\n\rb]]",
    ] {
        assert_eq!(literal(source, Options::default()), source);
    }
}

#[test]
fn disabling_string_shortening_preserves_escapes_and_long_brackets() {
    let options = Options {
        minify_strings: false,
        ..Options::default()
    };
    for source in [br#""\065\066\067""#.as_slice(), b"[[hello]]"] {
        assert_eq!(literal(source, options), source);
    }
}

#[test]
fn long_string_candidates_cannot_create_a_bracket_opener_with_indexing() {
    let source = br#"local t={["a\nb\nc\nd\ne\nf"]=7};print(t[ "a\nb\nc\nd\ne\nf" ])"#;
    let output = minify_with_options(source, Options::default()).unwrap();
    assert!(
        output.windows(4).any(|window| window == b"t[ ["),
        "{output:?}"
    );
}
