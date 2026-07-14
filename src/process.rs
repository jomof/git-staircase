use crate::error::{Result, StaircaseError};
use std::io::{Read, Write};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub struct ProcessExecutor {
    command: Command,
    stdin: Option<Vec<u8>>,
    timeout: Option<Duration>,
}

impl ProcessExecutor {
    pub fn new(command: Command) -> Self {
        Self {
            command,
            stdin: None,
            timeout: None,
        }
    }

    pub fn stdin(mut self, stdin: impl Into<Vec<u8>>) -> Self {
        self.stdin = Some(stdin.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn run(mut self) -> Result<Output> {
        let mut child = self
            .command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(StaircaseError::Io)?;

        let stdin_data = self.stdin.take();
        let mut stdin = child.stdin.take();
        let mut stdout = child.stdout.take();
        let mut stderr = child.stderr.take();

        let (stdout_tx, stdout_rx) = mpsc::channel();
        let (stderr_tx, stderr_rx) = mpsc::channel();

        if let (Some(mut stdin), Some(data)) = (stdin.take(), stdin_data) {
            thread::spawn(move || {
                let _ = stdin.write_all(&data);
            });
        }

        thread::spawn(move || {
            let mut out = Vec::new();
            if let Some(mut stdout) = stdout.take() {
                let _ = stdout.read_to_end(&mut out);
            }
            let _ = stdout_tx.send(out);
        });

        thread::spawn(move || {
            let mut err = Vec::new();
            if let Some(mut stderr) = stderr.take() {
                let _ = stderr.read_to_end(&mut err);
            }
            let _ = stderr_tx.send(err);
        });

        let status = match self.timeout {
            None => child.wait().map_err(StaircaseError::Io)?,
            Some(timeout) => {
                let start = Instant::now();
                loop {
                    match child.try_wait().map_err(StaircaseError::Io)? {
                        Some(status) => break status,
                        None => {
                            if start.elapsed() >= timeout {
                                let _ = child.kill();
                                return Err(StaircaseError::Other(format!(
                                    "Process timed out after {:?}",
                                    timeout
                                )));
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            }
        };

        Ok(Output {
            status,
            stdout: stdout_rx.recv().unwrap_or_default(),
            stderr: stderr_rx.recv().unwrap_or_default(),
        })
    }
}
