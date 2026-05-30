use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use super::{
    ActiveRunHandle, RunCompletion, RunEvent, RunEventReceiver, RunOutcome, RunStartError,
    RunToolError, RunToolErrorKind, RunnerState,
};
use crate::runner::RunCommand;

pub(crate) fn start(
    command: RunCommand,
    state: Arc<Mutex<RunnerState>>,
) -> Result<RunEventReceiver, RunStartError> {
    let mut child = Command::new(&command.program)
        .args(&command.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| RunStartError::SpawnFailed {
            program: command.program.clone(),
            message: error.to_string(),
        })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (event_tx, event_rx) = mpsc::channel();
    let (cancel_tx, cancel_rx) = mpsc::channel();

    {
        let mut state = state.lock().expect("runner state lock");
        if state.active.is_some() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RunStartError::ActiveRunInProgress);
        }

        state.latest_report_path = Some(command.report_path.clone());
        state.active = Some(ActiveRunHandle { cancel_tx });
    }

    thread::spawn(move || {
        run_child_process(command, state, child, stdout, stderr, event_tx, cancel_rx)
    });

    Ok(event_rx)
}

fn run_child_process(
    command: RunCommand,
    state: Arc<Mutex<RunnerState>>,
    mut child: std::process::Child,
    stdout: Option<std::process::ChildStdout>,
    stderr: Option<std::process::ChildStderr>,
    event_tx: mpsc::Sender<RunEvent>,
    cancel_rx: mpsc::Receiver<()>,
) {
    let stdout_thread = stdout.map(|stream| spawn_reader(stream, event_tx.clone(), true));
    let stderr_thread = stderr.map(|stream| spawn_reader(stream, event_tx.clone(), false));

    let mut cancelled = false;
    let exit_code = loop {
        if cancel_rx.try_recv().is_ok() {
            cancelled = true;
            let _ = child.kill();
        }

        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                let completion = RunCompletion {
                    exit_code: None,
                    report_path: command.report_path.clone(),
                    outcome: RunOutcome::ToolError(RunToolError {
                        kind: RunToolErrorKind::Terminated,
                        message: format!("failed while waiting for Bruno process: {error}"),
                    }),
                };
                let _ = event_tx.send(RunEvent::Finished(completion));
                clear_active_run(&state);
                return;
            }
        }
    };

    if let Some(thread) = stdout_thread {
        let _ = thread.join();
    }
    if let Some(thread) = stderr_thread {
        let _ = thread.join();
    }

    let completion =
        super::completion::classify_completion(exit_code, &command.report_path, cancelled);
    let _ = event_tx.send(RunEvent::Finished(completion));
    clear_active_run(&state);
}

fn spawn_reader(
    stream: impl std::io::Read + Send + 'static,
    event_tx: mpsc::Sender<RunEvent>,
    stdout: bool,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let event = if stdout {
                        RunEvent::Stdout(line)
                    } else {
                        RunEvent::Stderr(line)
                    };
                    if event_tx.send(event).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = event_tx.send(RunEvent::Stderr(format!(
                        "failed to read Bruno process output: {error}"
                    )));
                    break;
                }
            }
        }
    })
}

fn clear_active_run(state: &Arc<Mutex<RunnerState>>) {
    state.lock().expect("runner state lock").active = None;
}
