use miniluar::{Options, minify, minify_with_options};

#[test]
fn removes_trivia_but_keeps_required_word_boundaries() {
    assert_eq!(
        minify(b"-- heading\r\nlocal value = 1 -- trailing\nreturn value\n").unwrap(),
        b"local value=1 return value\n"
    );
}

#[test]
fn unambiguous_punctuation_does_not_keep_unnecessary_spaces() {
    assert_eq!(minify(b" print ( 1 , 2 ) ").unwrap(), b"print(1,2)\n");
}

#[test]
fn removal_cannot_create_comments_compound_operators_or_long_brackets() {
    let options = Options {
        minify_strings: false,
        minify_numbers: false,
        ..Options::default()
    };
    for (source, expected) in [
        ("x - -- comment\n - y", "x- -y\n"),
        ("local x <const> = 1", "local x<const> =1\n"),
        ("t[ [[key]] ]", "t[ [[key]]]\n"),
        ("a[ = [b", "a[ =[b\n"),
        ("x / / y", "x/ /y\n"),
        ("x .. .5", "x.. .5\n"),
        ("return 1. and true", "return 1. and true\n"),
    ] {
        assert_eq!(
            minify_with_options(source.as_bytes(), options).unwrap(),
            expected.as_bytes(),
            "{source}"
        );
    }
}

#[test]
fn disabled_literal_transforms_keep_bytes_and_spelling() {
    let options = Options {
        minify_strings: false,
        minify_numbers: false,
        ..Options::default()
    };
    let source = b"local s = '\xff\\255' ; local n = 0x1.fp+3 ; return [==[a\r\n--b ]=] c]==]";
    assert_eq!(
        minify_with_options(source, options).unwrap(),
        b"local s='\xff\\255';local n=0x1.fp+3;return[==[a\r\n--b ]=] c]==]\n"
    );
}

#[test]
fn quoted_comment_markers_are_not_removed() {
    assert_eq!(
        minify(b"return '--keep', \"--[[keep]]\" --remove").unwrap(),
        b"return'--keep',\"--[[keep]]\"\n"
    );
}

#[test]
fn long_comments_require_a_matching_delimiter() {
    assert_eq!(minify(b"a --[==[ ]=] ignored ]==] + b").unwrap(), b"a+b\n");
    assert!(minify(b"a --[==[ ]=] unfinished").is_err());
}

#[test]
fn unfinished_literals_fail_instead_of_emitting_partial_output() {
    for source in [
        b"'unfinished".as_slice(),
        b"'bare\nnewline'",
        b"[[unfinished",
        b"`a{1",
        b"`a",
    ] {
        assert!(minify(source).is_err(), "{source:?}");
    }
}

#[test]
fn escaped_newlines_and_whitespace_skip_escapes_are_preserved() {
    for source in [b"return 'a\\\r\nb'".as_slice(), b"return 'a\\z \t\r\n b'"] {
        let output = minify(source).unwrap();
        assert_eq!(&output[6..output.len() - 1], &source[7..]);
    }
}

#[test]
fn interpolation_handles_nested_strings_braces_and_comments() {
    let literal = b"`value { {x=`nested {\"}\"}`} -- } `\n } tail`";
    let mut source = b"return ".to_vec();
    source.extend(literal);
    let mut expected = b"return".to_vec();
    expected.extend(literal);
    expected.push(b'\n');
    assert_eq!(minify(&source).unwrap(), expected);
}

#[test]
fn only_an_initial_shebang_is_a_comment_and_it_survives() {
    assert_eq!(
        minify(b"#!/usr/bin/lua\r\n print(42) -- hi").unwrap(),
        b"#!/usr/bin/lua\nprint(42)\n"
    );
    assert_eq!(minify(b" #!x").unwrap(), b"#!x\n");
}

#[test]
fn empty_and_comment_only_inputs_have_one_final_newline() {
    for source in [b"".as_slice(), b"  \r\n", b"-- comment"] {
        assert_eq!(minify(source).unwrap(), b"\n");
    }
}
