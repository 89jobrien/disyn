use std::{collections::VecDeque, ffi::OsStr, io};

use xtask::{CommandInvocation, ProcessOutcome, ProcessRunner};

#[derive(Default)]
struct FakeRunner {
    calls: Vec<(String, Vec<String>)>,
    results: VecDeque<io::Result<ProcessOutcome>>,
}

impl FakeRunner {
    fn with_results(results: impl IntoIterator<Item = io::Result<ProcessOutcome>>) -> Self {
        Self {
            calls: Vec::new(),
            results: results.into_iter().collect(),
        }
    }
}

impl ProcessRunner for FakeRunner {
    fn run(&mut self, invocation: &CommandInvocation) -> io::Result<ProcessOutcome> {
        self.calls.push((
            invocation.program().to_string_lossy().into_owned(),
            invocation
                .args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
        ));

        self.results
            .pop_front()
            .unwrap_or_else(|| panic!("unexpected invocation: {:?}", invocation))
    }
}

#[test]
fn forwards_arguments_to_taskit_and_preserves_exit_code() {
    let mut runner = FakeRunner::with_results([Ok(ProcessOutcome::failure(7))]);

    let code = xtask::run_with(["ci", "--fail-fast"], &mut runner);

    assert_eq!(code, 7);
    assert_eq!(
        runner.calls,
        vec![(
            "taskit".to_owned(),
            vec!["ci".to_owned(), "--fail-fast".to_owned()]
        )]
    );
}

#[test]
fn installs_taskit_and_retries_when_taskit_is_missing() {
    let mut runner = FakeRunner::with_results([
        Err(io::Error::from(io::ErrorKind::NotFound)),
        Ok(ProcessOutcome::success()),
        Ok(ProcessOutcome::failure(3)),
    ]);

    let code = xtask::run_with(["fmt", "--check"], &mut runner);

    assert_eq!(code, 3);
    assert_eq!(
        runner.calls,
        vec![
            (
                "taskit".to_owned(),
                vec!["fmt".to_owned(), "--check".to_owned()]
            ),
            (
                "cargo".to_owned(),
                vec!["install".to_owned(), "taskit".to_owned()]
            ),
            (
                "taskit".to_owned(),
                vec!["fmt".to_owned(), "--check".to_owned()]
            ),
        ]
    );
}

#[test]
fn returns_install_failure_code_without_retrying_taskit() {
    let mut runner = FakeRunner::with_results([
        Err(io::Error::from(io::ErrorKind::NotFound)),
        Ok(ProcessOutcome::failure(42)),
    ]);

    let code = xtask::run_with(["test"], &mut runner);

    assert_eq!(code, 42);
    assert_eq!(
        runner.calls,
        vec![
            ("taskit".to_owned(), vec!["test".to_owned()]),
            (
                "cargo".to_owned(),
                vec!["install".to_owned(), "taskit".to_owned()]
            ),
        ]
    );
}

#[test]
fn unrelated_taskit_spawn_errors_do_not_attempt_install() {
    let mut runner =
        FakeRunner::with_results([Err(io::Error::from(io::ErrorKind::PermissionDenied))]);

    let code = xtask::run_with(["pre-push"], &mut runner);

    assert_eq!(code, 1);
    assert_eq!(
        runner.calls,
        vec![("taskit".to_owned(), vec!["pre-push".to_owned()])]
    );
}

#[test]
fn signal_termination_maps_to_failure_exit_code() {
    let mut runner = FakeRunner::with_results([Ok(ProcessOutcome::terminated_by_signal())]);

    let code = xtask::run_with([OsStr::new("ci")], &mut runner);

    assert_eq!(code, 1);
}
