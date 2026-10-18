use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct NodeDriver;

impl NodeDriver {
    const DEADLINE: Duration = Duration::from_mins(2);
    const POLL: Duration = Duration::from_millis(20);

    pub fn command() -> Command {
        let mut command = Command::new("node");
        command.stdin(Stdio::null());
        command
    }

    pub fn output(command: &mut Command) -> Output {
        let description = format!("{command:?}");
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("cannot start {description}: {error}"));
        let stdout = Self::collect(child.stdout.take());
        let stderr = Self::collect(child.stderr.take());
        let deadline = Instant::now() + Self::DEADLINE;
        let mut timed_out = false;
        while child.try_wait().expect("node status").is_none() {
            if Instant::now() > deadline {
                timed_out = true;
                let _killed = child.kill();
                break;
            }
            std::thread::sleep(Self::POLL);
        }
        let status = child.wait().expect("node exit");
        let stdout = stdout.join().expect("stdout reader");
        let stderr = stderr.join().expect("stderr reader");
        assert!(
            !timed_out,
            "{description} did not finish within {} seconds, so it was killed\nstdout before the deadline:\n{}\nstderr before the deadline:\n{}",
            Self::DEADLINE.as_secs(),
            String::from_utf8_lossy(&stdout),
            String::from_utf8_lossy(&stderr)
        );
        Output {
            status,
            stdout,
            stderr,
        }
    }

    fn collect(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _read = pipe.read_to_end(&mut bytes);
            }
            bytes
        })
    }
}
