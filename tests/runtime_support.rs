#[path = "support/runtime.rs"]
mod runtime;

#[test]
fn runtime_helper_drains_large_stdout_on_every_dialect() {
    let mut expected = vec![b'x'; 256 * 1024];
    expected.push(b'\n');
    for runtime in runtime::all() {
        assert_eq!(
            runtime.stdout(b"print(string.rep('x', 256 * 1024))"),
            expected,
            "{}",
            runtime.name
        );
    }
}

#[test]
fn runtime_helper_drains_both_streams_without_losing_bytes() {
    // Luau does not expose the Lua io library.
    for runtime in runtime::all().into_iter().filter(|r| r.name != "LUAU") {
        let output = runtime.execute(
            b"io.stdout:write(string.rep('o', 256 * 1024)); io.stderr:write(string.rep('e', 256 * 1024))",
        );
        assert!(output.status.success(), "{}", runtime.name);
        assert_eq!(output.stdout, vec![b'o'; 256 * 1024]);
        assert_eq!(output.stderr, vec![b'e'; 256 * 1024]);
    }
}

#[test]
#[should_panic(expected = "LUA54 timed out")]
fn runtime_helper_still_kills_programs_that_exceed_the_deadline() {
    runtime::all()
        .into_iter()
        .find(|r| r.name == "LUA54")
        .unwrap()
        .execute(b"while true do end");
}
