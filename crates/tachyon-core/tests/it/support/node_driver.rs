use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct NodeDriver;

struct Attempt {
    output: Output,
    timed_out: bool,
}

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
        let mut retry = Self::copy(command);
        let first = Self::attempt(command, &description);
        let (output, retried) = if first.timed_out && Self::printed_report(&first.output.stdout) {
            eprintln!(
                "{description} printed its report but did not exit before its deadline; running it once more"
            );
            (Self::attempt(&mut retry, &description), true)
        } else {
            (first, false)
        };
        let which_run = if retried {
            " on its one retry after an exit hang"
        } else {
            ""
        };
        assert!(
            !output.timed_out,
            "{description} did not finish within {} seconds, so it was killed{}\nstdout before the deadline:\n{}\nstderr before the deadline:\n{}",
            Self::DEADLINE.as_secs(),
            which_run,
            String::from_utf8_lossy(&output.output.stdout),
            String::from_utf8_lossy(&output.output.stderr)
        );
        output.output
    }

    fn attempt(command: &mut Command, description: &str) -> Attempt {
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
        Attempt {
            output: Output {
                status,
                stdout: stdout.join().expect("stdout reader"),
                stderr: stderr.join().expect("stderr reader"),
            },
            timed_out,
        }
    }

    fn printed_report(stdout: &[u8]) -> bool {
        let stdout = String::from_utf8_lossy(stdout);
        stdout.ends_with('\n')
            && stdout.lines().last().is_some_and(|line| {
                serde_json::from_str::<serde_json::Value>(line)
                    .is_ok_and(|report| report.is_object() || report.is_array())
            })
    }

    fn copy(command: &Command) -> Command {
        let mut copy = Command::new(command.get_program());
        copy.args(command.get_args());
        if let Some(folder) = command.get_current_dir() {
            copy.current_dir(folder);
        }
        for (name, value) in command.get_envs() {
            match value {
                Some(value) => copy.env(name, value),
                None => copy.env_remove(name),
            };
        }
        copy
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
