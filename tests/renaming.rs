#[path = "support/runtime.rs"]
mod runtime;

use miniluar::{Options, minify, minify_with_options};

fn renamed(source: &[u8]) -> Vec<u8> {
    minify_with_options(
        source,
        Options {
            rename_locals: true,
            ..Options::default()
        },
    )
    .unwrap()
}

#[test]
fn renaming_shortens_a_local_and_its_references_but_is_opt_in() {
    let source = b"local longVariableName=3;print(longVariableName)";
    let output = renamed(source);
    assert!(output.len() < source.len());
    assert!(
        !output
            .windows(b"longVariableName".len())
            .any(|w| w == b"longVariableName")
    );
    assert_eq!(
        minify(source).unwrap(),
        b"local longVariableName=3;print(longVariableName)\n"
    );
}

#[test]
fn generated_names_cannot_capture_free_global_variables() {
    for runtime in runtime::all() {
        for (source, expected) in [
            ("a=10; local value=2; print(a+value)", "12\n"),
            ("a=10; local value=2; a=a+value; print(a,value)", "12\t2\n"),
            (
                "a=10; local value=2; local functionValue=function() return a+value end; print(functionValue())",
                "12\n",
            ),
            ("local value=2; print(value+3)", "5\n"),
        ] {
            assert_eq!(runtime.stdout(source.as_bytes()), expected.as_bytes());
            assert_eq!(
                runtime.stdout(&renamed(source.as_bytes())),
                expected.as_bytes()
            );
        }
    }
}

#[test]
fn table_keys_and_members_remain_in_a_separate_namespace() {
    for runtime in runtime::all() {
        for (source, expected) in [
            (
                r#"local x=1; local t={x=2}; print(t["x"],t.x,x)"#,
                "2\t2\t1\n",
            ),
            (
                r#"local x=3; local t={x=2,[x]=4}; print(t["x"],t[x])"#,
                "2\t4\n",
            ),
            (
                r#"local x=3; local t={nested={x=2},x=x}; print(t.nested["x"],t["x"])"#,
                "2\t3\n",
            ),
            (r#"local x=3; local t={x=0}; t.x=x; print(t["x"])"#, "3\n"),
            (
                r#"local x=3; local t={x=function(self) return 9 end}; print(t["x"](t),t:x(),x)"#,
                "9\t9\t3\n",
            ),
            (
                r#"local x=3; local t={x=2; y=x}; print(t["x"],t.y)"#,
                "2\t3\n",
            ),
        ] {
            assert_eq!(
                runtime.stdout(&renamed(source.as_bytes())),
                expected.as_bytes(),
                "{}: {source}",
                runtime.name
            );
        }
    }
}

#[test]
fn scopes_keep_shadowing_initializers_closures_and_loop_headers_distinct() {
    for runtime in runtime::all() {
        for (source, expected) in [
            (
                "local x=7; do local x=x+1; print(x) end; print(x)",
                "8\n7\n",
            ),
            ("local x=2; local y=3; local x,y=y,x; print(x,y)", "3\t2\n"),
            (
                "local x=1; repeat local x=x+1; print(x) until x==2; print(x)",
                "2\n1\n",
            ),
            (
                "local i=100; for i=i-99,1 do print(i) end; print(i)",
                "1\n100\n",
            ),
            (
                "local x={8}; for x,y in ipairs(x) do print(x,y) end; print(x[1])",
                "1\t8\n8\n",
            ),
            (
                "local x=7; if false then local x=2 elseif x==7 then print(x) else local x=9 end; print(x)",
                "7\n7\n",
            ),
            (
                "local x=7; if false then local x=2 else print(x) end",
                "7\n",
            ),
            ("local function f(f) return f end; print(f(4))", "4\n"),
            (
                "local x=1; local before=function() return x end; local x=2; local after=function() return x end; x=3; print(before(),after())",
                "1\t3\n",
            ),
            (
                "local x=7; local x=function() return x end; print(x())",
                "7\n",
            ),
            (
                "local function f(n) if n==0 then return 1 end return n*f(n-1) end; print(f(4))",
                "24\n",
            ),
            (
                "local self=99; local t={value=7}; function t:get() return self.value end; print(t:get(),self)",
                "7\t99\n",
            ),
            (
                "local self=99; local t={value=7}; function t:get() return function() return self.value end end; print(t:get()(),self)",
                "7\t99\n",
            ),
            (
                r#"local t={}; local method=5; function t.method(method) return method+1 end; print(t["method"](8),method)"#,
                "9\t5\n",
            ),
            (
                "local x=0; while x<2 do local y=x; x=x+1; print(y) end; do local x=9; print(x) end; print(x)",
                "0\n1\n9\n2\n",
            ),
            ("local x=5; do local x; print(x) end; print(x)", "nil\n5\n"),
            (
                "local x=1; local function before() return x end; local x=x+1; print(before(),x)",
                "1\t2\n",
            ),
            (
                "local i=2; for i=1,i do for i=1,i do print(i) end end; print(i)",
                "1\n1\n2\n2\n",
            ),
            (
                "local k=9; for k,v in function(_,last) if last==nil then return k,4 end end,nil,nil do print(k,v) end; print(k)",
                "9\t4\n9\n",
            ),
            (
                "local n=0; repeat local value=n; n=n+1 until (function() return value==1 end)(); print(n)",
                "2\n",
            ),
            (
                "local self=10; local t={}; function t:f(self) return self end; print(t:f(3),self)",
                "3\t10\n",
            ),
            (
                r#"local x=3; local t={f=function(y,z) return y+z end,x=x}; print(t.f(x,2),t["x"])"#,
                "5\t3\n",
            ),
            (
                "local type=type; local value=3; print(type(value))",
                "number\n",
            ),
            (
                "local x=1; local function f(...) local x=select(1,...); return x end; print(f(4),x)",
                "4\t1\n",
            ),
            (
                "local x=2; print(-x^2==-4,2^-x==0.25,x..x,x==2 and x or 9)",
                "true\ttrue\t22\t2\n",
            ),
        ] {
            assert_eq!(
                runtime.stdout(&renamed(source.as_bytes())),
                expected.as_bytes(),
                "{}: {source}",
                runtime.name
            );
        }
    }
}

#[test]
fn preserved_bindings_still_shadow_outer_bindings() {
    let options = Options {
        rename_locals: true,
        preserve_names: &["value", "a"],
        ..Options::default()
    };
    for runtime in runtime::all() {
        for (source, expected) in [
            (
                "local value=1; local function identity(value) return value end; print(identity(3),value)",
                "3\t1\n",
            ),
            ("local a=10; local value=2; print(a+value)", "12\n"),
        ] {
            let output = minify_with_options(source.as_bytes(), options).unwrap();
            assert!(output.windows(5).any(|w| w == b"value"));
            assert_eq!(runtime.stdout(&output), expected.as_bytes());
        }
    }
}

#[test]
fn local_environment_binding_keeps_its_special_name() {
    let source = b"local _ENV=setmetatable({},{__index=_G});local value=3;print(value)";
    let output = renamed(source);
    assert!(output.windows(4).any(|w| w == b"_ENV"));
    for runtime in runtime::all() {
        assert_eq!(runtime.stdout(&output), b"3\n");
    }
}

#[test]
fn lua_labels_and_attributes_are_not_variable_references() {
    for runtime in runtime::all() {
        if runtime.name == "LUAU" || runtime.name == "LUA51" {
            continue;
        }
        let source = b"local target=7 ::target:: print(target)";
        assert_eq!(runtime.stdout(&renamed(source)), b"7\n");
        if runtime.name == "LUA54" {
            for (source, expected) in [
                (
                    "local target <const> = 7; goto target; ::target:: print(target)",
                    "7\n",
                ),
                (
                    "local value=1; do local resource <close> = setmetatable({},{__close=function() print(value) end}); local value=2; print(value) end; print(value)",
                    "2\n1\n1\n",
                ),
            ] {
                assert_eq!(
                    runtime.stdout(&renamed(source.as_bytes())),
                    expected.as_bytes()
                );
            }
        }
    }
}

#[test]
fn luau_compound_assignment_and_continue_keep_their_bindings() {
    let runtime = runtime::all()
        .into_iter()
        .find(|r| r.name == "LUAU")
        .unwrap();
    let source =
        b"local total=0; for i=1,3 do if i==2 then continue end total+=i end; print(total)";
    assert_eq!(runtime.stdout(&renamed(source)), b"4\n");
}

#[test]
fn unsupported_renaming_syntax_is_an_error_before_any_transformation() {
    let options = Options {
        rename_locals: true,
        ..Options::default()
    };
    for source in [
        "local longVariable=3; local value: number=longVariable; print(value)",
        "local function identity(value: number): number return value end; print(identity(3))",
        "type Value = number; local value: Value = 3; print(value)",
        "local value=3; print(value :: number)",
        "local value=3; print(if value==3 then value else 0)",
        "local function identity<T>(value: T): T return value end; print(identity(3))",
        "local value=3; print(`value {value}`)",
    ] {
        assert!(
            minify_with_options(source.as_bytes(), options).is_err(),
            "{source}"
        );
        assert!(
            minify(source.as_bytes()).is_ok(),
            "ordinary compaction should still accept {source}"
        );
    }
}

#[test]
fn allocator_expands_beyond_single_letters_without_capture_or_lengthening() {
    let mut source = String::from("a=1000;");
    for index in 0..120 {
        source.push_str(&format!("local longVariable{index}={index};"));
    }
    source.push_str("print(a");
    for index in 0..120 {
        source.push_str(&format!("+longVariable{index}"));
    }
    source.push(')');
    let output = renamed(source.as_bytes());
    assert!(!output.windows(12).any(|w| w == b"longVariable"));
    for runtime in runtime::all() {
        assert_eq!(runtime.stdout(&output), b"8140\n");
    }

    let source = "local x=1;".repeat(120) + "print(x)";
    let output = renamed(source.as_bytes());
    assert!(output.len() <= minify(source.as_bytes()).unwrap().len());
    for runtime in runtime::all() {
        assert_eq!(runtime.stdout(&output), b"1\n");
    }
}

#[test]
fn renaming_reports_source_offsets_and_bounds_recursive_parsing() {
    let source = b"-- heading\nlocal longVariable=3; local typed: number=longVariable";
    let error = minify_with_options(
        source,
        Options {
            rename_locals: true,
            ..Options::default()
        },
    )
    .unwrap_err();
    assert_eq!(
        error.offset,
        source.iter().position(|&b| b == b':').unwrap()
    );
    for depth in [16, 256] {
        let source = format!(
            "local value=3; print({}value{})",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = minify_with_options(
            source.as_bytes(),
            Options {
                rename_locals: true,
                ..Options::default()
            },
        );
        if depth == 16 {
            for runtime in runtime::all() {
                assert_eq!(runtime.stdout(result.as_ref().unwrap()), b"3\n");
            }
        } else {
            assert_eq!(
                result.unwrap_err().message,
                "renaming nesting limit exceeded"
            );
            assert!(minify(source.as_bytes()).is_ok());
        }
    }
}

#[test]
fn contextual_keywords_can_still_name_bindings_in_dialects_that_allow_them() {
    for runtime in runtime::all() {
        let source = b"local continue=3; local export=4; local type=5; continue=continue+1; print(continue,export,type)";
        assert_eq!(runtime.stdout(source), b"4\t4\t5\n");
        assert_eq!(runtime.stdout(&renamed(source)), b"4\t4\t5\n");
        let source = b"local continue={value=7}; function continue:get() return self.value end; print(continue:get())";
        assert_eq!(runtime.stdout(source), b"7\n");
        assert_eq!(runtime.stdout(&renamed(source)), b"7\n");
        if matches!(runtime.name, "LUA51" | "LUAU") {
            let source = b"local goto=3; goto=goto+1; print(goto)";
            assert_eq!(runtime.stdout(source), b"4\n");
            assert_eq!(runtime.stdout(&renamed(source)), b"4\n");
        }
    }
}
