mod support;

use std::fs;
use support::{Sandbox, success};

fn source(dir: &Sandbox, name: &str, value: u8) {
    let path = dir.path(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("print ( {value} )")).unwrap();
}

#[test]
fn directory_discovery_is_sorted_and_filters_extensions() {
    let dir = Sandbox::new();
    source(&dir, "src/b.luau", 2);
    source(&dir, "src/a.lua", 1);
    source(&dir, "src/notes.txt", 9);
    source(&dir, "src/nested/c.lua", 3);
    for (args, expected) in [
        (
            vec!["src", "--stdout", "-q"],
            b"print(1)\nprint(2)\n".as_slice(),
        ),
        (
            vec!["src", "--stdout", "-q", "-r"],
            b"print(1)\nprint(2)\nprint(3)\n",
        ),
    ] {
        let result = dir.run(&args, b"");
        success(&result);
        assert_eq!(result.stdout, expected);
    }
}

#[test]
fn explicit_files_keep_nonstandard_extensions_and_input_order() {
    let dir = Sandbox::new();
    source(&dir, "a.txt", 1);
    source(&dir, "b.lua", 2);
    let result = dir.run(&["b.lua", "a.txt", "--stdout"], b"");
    success(&result);
    assert_eq!(result.stdout, b"print(2)\nprint(1)\n");
}

#[test]
fn overlapping_file_and_directory_inputs_are_processed_once() {
    let dir = Sandbox::new();
    source(&dir, "src/only.luau", 42);
    let result = dir.run(
        &["src/only.luau", "src", "src/./only.luau", "--stdout"],
        b"",
    );
    success(&result);
    assert_eq!(result.stdout, b"print(42)\n");
}

#[test]
fn empty_directories_fail_without_creating_an_output_directory() {
    let dir = Sandbox::new();
    fs::create_dir(dir.path("empty")).unwrap();
    let result = dir.run(&["empty", "-d", "dist"], b"");
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("No Lua/Luau files"));
    assert!(!dir.path("dist").exists());
}

#[test]
fn distinct_batch_outputs_use_the_requested_directory_and_suffix() {
    let dir = Sandbox::new();
    source(&dir, "a.lua", 1);
    source(&dir, "b.luau", 2);
    let result = dir.run(
        &["a.lua", "b.luau", "-d", "dist", "--suffix", ".small.lua"],
        b"",
    );
    success(&result);
    assert!(result.stdout.is_empty());
    assert_eq!(
        fs::read(dir.path("dist/a.small.lua")).unwrap(),
        b"print(1)\n"
    );
    assert_eq!(
        fs::read(dir.path("dist/b.small.lua")).unwrap(),
        b"print(2)\n"
    );
}

#[test]
fn duplicate_destinations_are_rejected_before_any_write_even_with_force() {
    let dir = Sandbox::new();
    source(&dir, "a/same.lua", 1);
    source(&dir, "b/same.luau", 2);
    for overwrite in [false, true] {
        let mut args = vec!["a/same.lua", "b/same.luau", "-d", "dist"];
        if overwrite {
            args.push("-f");
        }
        let result = dir.run(&args, b"");
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("collision"));
        assert!(!dir.path("dist").exists());
    }
}

#[test]
fn a_destination_cannot_replace_another_input_even_with_in_place() {
    let dir = Sandbox::new();
    source(&dir, "same.lua", 1);
    source(&dir, "same.min.lua", 2);
    let result = dir.run(&["same.lua", "same.min.lua", "-f"], b"");
    assert!(!result.status.success());
    assert_eq!(fs::read(dir.path("same.lua")).unwrap(), b"print ( 1 )");
    assert_eq!(fs::read(dir.path("same.min.lua")).unwrap(), b"print ( 2 )");
    let result = dir.run(&["same.lua", "same.min.lua", "-i"], b"");
    success(&result);
    assert_eq!(fs::read(dir.path("same.lua")).unwrap(), b"print(1)\n");
    assert_eq!(fs::read(dir.path("same.min.lua")).unwrap(), b"print(2)\n");
}

#[test]
fn stdout_does_not_require_distinct_destinations() {
    let dir = Sandbox::new();
    source(&dir, "a/same.lua", 1);
    source(&dir, "b/same.lua", 2);
    let result = dir.run(&["a/same.lua", "b/same.lua", "--stdout"], b"");
    success(&result);
    assert_eq!(result.stdout, b"print(1)\nprint(2)\n");
}

#[test]
fn single_output_option_requires_one_discovered_source() {
    let dir = Sandbox::new();
    source(&dir, "a.lua", 1);
    source(&dir, "b.lua", 2);
    let result = dir.run(&["a.lua", "b.lua", "-o", "out.lua"], b"");
    assert!(!result.status.success());
    assert!(!dir.path("out.lua").exists());
    assert!(!dir.path("a.min.lua").exists());
}

#[test]
fn per_file_failures_do_not_contaminate_successful_stdout() {
    let dir = Sandbox::new();
    source(&dir, "valid.lua", 42);
    fs::write(dir.path("invalid.lua"), b"'unfinished").unwrap();
    for bad in ["invalid.lua", "missing.lua"] {
        let result = dir.run(&["valid.lua", bad, "--stdout", "-q"], b"");
        assert!(!result.status.success());
        assert_eq!(result.stdout, b"print(42)\n");
        assert!(String::from_utf8_lossy(&result.stderr).contains(bad));
    }
}

#[test]
fn in_place_does_not_authorize_overwriting_an_unrelated_explicit_output() {
    let dir = Sandbox::new();
    source(&dir, "input.lua", 42);
    fs::write(dir.path("existing.lua"), b"precious").unwrap();
    let result = dir.run(&["input.lua", "-i", "-o", "existing.lua"], b"");
    assert!(!result.status.success());
    assert_eq!(fs::read(dir.path("existing.lua")).unwrap(), b"precious");
    let result = dir.run(&["input.lua", "-i", "-o", "existing.lua", "-f"], b"");
    success(&result);
    assert_eq!(fs::read(dir.path("existing.lua")).unwrap(), b"print(42)\n");
    assert_eq!(fs::read(dir.path("input.lua")).unwrap(), b"print ( 42 )");
}

#[cfg(unix)]
#[test]
fn aliases_of_inputs_and_destinations_are_checked_before_writing() {
    use std::os::unix::fs::symlink;
    let dir = Sandbox::new();
    source(&dir, "a.lua", 1);
    source(&dir, "b.lua", 2);
    for hard in [false, true] {
        if hard {
            fs::hard_link(dir.path("b.lua"), dir.path("a.min.lua")).unwrap();
        } else {
            symlink(dir.path("b.lua"), dir.path("a.min.lua")).unwrap();
        }
        let result = dir.run(&["a.lua", "b.lua", "-f"], b"");
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("collision"));
        assert_eq!(fs::read(dir.path("b.lua")).unwrap(), b"print ( 2 )");
        assert!(!dir.path("b.min.lua").exists());
        fs::remove_file(dir.path("a.min.lua")).unwrap();
    }
    source(&dir, "outputs/a.min.lua", 8);
    fs::hard_link(dir.path("outputs/a.min.lua"), dir.path("outputs/b.min.lua")).unwrap();
    let result = dir.run(&["a.lua", "b.lua", "-d", "outputs", "-f"], b"");
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("collision"));
    assert_eq!(
        fs::read(dir.path("outputs/a.min.lua")).unwrap(),
        b"print ( 8 )"
    );
}

#[cfg(unix)]
#[test]
fn recursive_discovery_does_not_follow_directory_symlink_cycles() {
    let dir = Sandbox::new();
    source(&dir, "src/a.lua", 1);
    std::os::unix::fs::symlink(".", dir.path("src/cycle")).unwrap();
    let result = dir.run(&["src", "-r", "--stdout"], b"");
    success(&result);
    assert_eq!(result.stdout, b"print(1)\n");
}

#[cfg(unix)]
#[test]
fn in_place_replacement_retains_executable_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = Sandbox::new();
    source(&dir, "script.lua", 1);
    fs::set_permissions(dir.path("script.lua"), fs::Permissions::from_mode(0o751)).unwrap();
    success(&dir.run(&["script.lua", "-i"], b""));
    assert_eq!(
        fs::metadata(dir.path("script.lua"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o751
    );
}

#[cfg(unix)]
#[test]
fn parent_directory_aliases_cannot_hide_input_collisions() {
    let dir = Sandbox::new();
    fs::create_dir(dir.path("real")).unwrap();
    std::os::unix::fs::symlink("real", dir.path("alias")).unwrap();
    let original = b"print ( 7 )";
    fs::write(dir.path("real/input.lua"), original).unwrap();
    for output in ["alias/input.lua", "real/missing/../input.lua"] {
        let result = dir.run(&["real/input.lua", "-o", output, "-f"], b"");
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("collision"));
        assert_eq!(fs::read(dir.path("real/input.lua")).unwrap(), original);
        assert!(!dir.path("real/missing").exists());
    }
    success(&dir.run(&["real/input.lua", "-o", "alias/new.lua", "-q"], b""));
    assert_eq!(fs::read(dir.path("real/new.lua")).unwrap(), b"print(7)\n");
    assert_eq!(fs::read(dir.path("real/input.lua")).unwrap(), original);
}
