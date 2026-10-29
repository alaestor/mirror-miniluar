use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

pub struct Runtime {
    pub name: &'static str,
    executable: OsString,
}

pub fn all() -> Vec<Runtime> {
    ["LUA51", "LUA52", "LUA53", "LUA54", "LUAU"]
        .into_iter()
        .map(|name| Runtime {
            name,
            executable: std::env::var_os(name)
                .unwrap_or_else(|| panic!("{name} must name an interpreter; use nix develop")),
        })
        .collect()
}

struct Program(PathBuf);

impl Drop for Program {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn read_all(mut pipe: impl Read) -> Vec<u8> {
    let mut bytes = Vec::new();
    pipe.read_to_end(&mut bytes).unwrap();
    bytes
}

impl Runtime {
    pub fn execute(&self, source: &[u8]) -> Output {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "miniluar-runtime-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let program = Program(directory);
        let path = program.0.join("program.lua");
        fs::write(&path, source).unwrap();
        let mut child = Command::new(&self.executable)
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("{}: {e}", self.name));
        let deadline = Instant::now() + Duration::from_secs(5);
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        std::thread::scope(|scope| {
            // Drain both pipes while waiting. Waiting first can block a healthy
            // interpreter in write() once either pipe fills.
            let stdout = scope.spawn(move || read_all(stdout));
            let stderr = scope.spawn(move || read_all(stderr));
            let mut timed_out = false;
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if Instant::now() >= deadline {
                    timed_out = true;
                    child.kill().unwrap();
                    break child.wait().unwrap();
                }
                std::thread::sleep(Duration::from_millis(2));
            };
            let output = Output {
                status,
                stdout: stdout.join().unwrap(),
                stderr: stderr.join().unwrap(),
            };
            assert!(!timed_out, "{} timed out: {:?}", self.name, output);
            output
        })
    }

    pub fn stdout(&self, source: &[u8]) -> Vec<u8> {
        let output = self.execute(source);
        assert!(
            output.status.success(),
            "{}: {source:?}: {}",
            self.name,
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }
}
