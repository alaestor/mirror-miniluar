use miniluar::{Options, minify, minify_with_options};

#[test]
fn disabled_trivia_transforms_preserve_exact_layout_and_bytes() {
    let options = Options {
        remove_comments: false,
        remove_whitespace: false,
        ..Options::default()
    };
    for source in [
        b"local  x = 1\n\n\tprint( x )\n\n".as_slice(),
        b"-- heading\r\nlocal x = 1\r\n-- tail\r\n",
        b"local --[[inline]] x = 1\nprint(x)\n",
        b"local x = 1\rprint(x)\r",
        b"local x = 1\n\rprint(x)\n\r",
        b"\t \n\n",
        b"#!/usr/bin/lua\r\nprint ( '\xff' )\r\n",
    ] {
        assert_eq!(minify_with_options(source, options).unwrap(), source);
    }
}

#[test]
fn whitespace_preservation_does_not_disable_comment_removal() {
    let options = Options {
        remove_whitespace: false,
        ..Options::default()
    };
    assert_eq!(
        minify_with_options(b"-- heading\nlocal  x = 1 -- inline\n\nprint(x)\n", options).unwrap(),
        b"\nlocal  x = 1 \n\nprint(x)\n"
    );
}

#[test]
fn removing_the_only_separator_still_keeps_token_boundaries() {
    let options = Options {
        remove_whitespace: false,
        ..Options::default()
    };
    assert_eq!(
        minify_with_options(b"local--[[only separator]]x = 1\n", options).unwrap(),
        b"local x = 1\n"
    );
}

#[test]
fn kept_comments_get_only_the_separators_they_need() {
    let options = Options {
        remove_comments: false,
        ..Options::default()
    };
    for (source, expected) in [
        ("print(1 - --[[comment]] 2)", "print(1- --[[comment]]2)\n"),
        ("print(1 - -- comment\n 2)", "print(1- -- comment\n2)\n"),
        (
            "print(1 - --[=[comment]=] -2)",
            "print(1- --[=[comment]=]-2)\n",
        ),
        ("print(1 + --[[comment]] 2)", "print(1+--[[comment]]2)\n"),
        (
            "--[not a long comment\nprint(1)",
            "--[not a long comment\nprint(1)\n",
        ),
        ("-- heading\nprint(1)", "-- heading\nprint(1)\n"),
    ] {
        assert_eq!(
            minify_with_options(source.as_bytes(), options).unwrap(),
            expected.as_bytes()
        );
    }
}

#[test]
fn kept_comments_do_not_duplicate_preserved_whitespace() {
    let options = Options {
        remove_comments: false,
        remove_whitespace: false,
        ..Options::default()
    };
    for source in [
        b"print(1 - --[[comment]] 2)\n".as_slice(),
        b"print(1 - -- comment\n 2)\n",
        b"print(1\t-\t--[=[comment]=]\t2)\n",
    ] {
        assert_eq!(minify_with_options(source, options).unwrap(), source);
    }
}

#[test]
fn keep_newlines_preserves_blank_lines_and_newline_spelling_but_not_spaces() {
    let options = Options {
        keep_newlines: true,
        ..Options::default()
    };
    for ending in ["\n", "\r", "\r\n", "\n\r"] {
        let source = format!("print ( 1 ){ending}{ending} print ( 2 ){ending}{ending}");
        let expected = format!("print(1){ending}{ending}print(2){ending}{ending}");
        assert_eq!(
            minify_with_options(source.as_bytes(), options).unwrap(),
            expected.as_bytes()
        );
    }
    assert_eq!(
        minify_with_options(b" \n\t\n\n", options).unwrap(),
        b"\n\n\n"
    );
}

#[test]
fn comment_preservation_does_not_disable_whitespace_removal() {
    let options = Options {
        remove_comments: false,
        ..Options::default()
    };
    assert_eq!(
        minify_with_options(b"local  x = 1\n\n print( x ) -- keep\n", options).unwrap(),
        b"local x=1 print(x)-- keep\n"
    );
    assert_eq!(
        minify(b"local  x = 1\n\n print( x ) -- remove\n").unwrap(),
        b"local x=1 print(x)\n"
    );
}

#[test]
fn preservation_adds_a_missing_final_newline() {
    let options = Options {
        remove_whitespace: false,
        ..Options::default()
    };
    assert_eq!(
        minify_with_options(b"print( 1 )", options).unwrap(),
        b"print( 1 )\n"
    );
}
