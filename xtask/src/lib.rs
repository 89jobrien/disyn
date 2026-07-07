//! Testable dispatcher for the `cargo xtask` taskit shim.
//!
//! The crate intentionally keeps the local xtask thin: taskit owns the CI and hook
//! behavior, while this shim ensures `cargo xtask <subcommand>` forwards arguments
//! consistently and bootstraps taskit when it is missing.

use std::{
    ffi::{OsStr, OsString},
    io,
    process::{Command, ExitStatus},
};

const TASKIT_BIN: &str = "taskit";
const CARGO_BIN: &str = "cargo";

/// A process invocation requested by the xtask dispatcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    program: OsString,
    args: Vec<OsString>,
}

impl CommandInvocation {
    /// Creates a new process invocation from a program name and argument list.
    pub fn new<P, I, A>(program: P, args: I) -> Self
    where
        P: Into<OsString>,
        I: IntoIterator<Item = A>,
        A: Into<OsString>,
    {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns the program name for this invocation.
    pub fn program(&self) -> &OsStr {
        self.program.as_os_str()
    }

    /// Returns the argument list for this invocation.
    pub fn args(&self) -> impl Iterator<Item = &OsStr> {
        self.args.iter().map(OsString::as_os_str)
    }
}

/// Normalized process status used by the dispatcher and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessOutcome {
    code: Option<i32>,
    success: bool,
}

impl ProcessOutcome {
    /// Creates a successful process outcome.
    pub const fn success() -> Self {
        Self {
            code: Some(0),
            success: true,
        }
    }

    /// Creates a failed process outcome with a concrete exit code.
    pub const fn failure(code: i32) -> Self {
        Self {
            code: Some(code),
            success: false,
        }
    }

    /// Creates a failed process outcome for signal termination or unknown status.
    pub const fn terminated_by_signal() -> Self {
        Self {
            code: None,
            success: false,
        }
    }

    /// Returns whether the process completed successfully.
    pub const fn success_status(self) -> bool {
        self.success
    }

    /// Converts this outcome to an exit code suitable for `std::process::exit`.
    pub const fn exit_code(self) -> i32 {
        match self.code {
            Some(code) => code,
            None => 1,
        }
    }
}

impl From<ExitStatus> for ProcessOutcome {
    fn from(status: ExitStatus) -> Self {
        Self {
            code: status.code(),
            success: status.success(),
        }
    }
}

/// Executes process invocations for the xtask dispatcher.
pub trait ProcessRunner {
    /// Runs an invocation and returns its normalized process outcome.
    fn run(&mut self, invocation: &CommandInvocation) -> io::Result<ProcessOutcome>;
}

/// Process runner backed by `std::process::Command`.
#[derive(Debug, Default)]
pub struct RealRunner;

impl ProcessRunner for RealRunner {
    fn run(&mut self, invocation: &CommandInvocation) -> io::Result<ProcessOutcome> {
        Command::new(invocation.program())
            .args(invocation.args())
            .status()
            .map(ProcessOutcome::from)
    }
}

/// Runs the xtask dispatcher with the real process runner.
pub fn run<I, A>(args: I) -> i32
where
    I: IntoIterator<Item = A>,
    A: Into<OsString>,
{
    let mut runner = RealRunner;
    run_with(args, &mut runner)
}

/// Runs the xtask dispatcher with an injectable process runner.
pub fn run_with<I, A, R>(args: I, runner: &mut R) -> i32
where
    I: IntoIterator<Item = A>,
    A: Into<OsString>,
    R: ProcessRunner,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    run_taskit(args, runner)
}

fn run_taskit<R>(args: Vec<OsString>, runner: &mut R) -> i32
where
    R: ProcessRunner,
{
    let taskit = CommandInvocation::new(TASKIT_BIN, args.clone());

    match runner.run(&taskit) {
        Ok(outcome) => outcome.exit_code(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            install_taskit_and_retry(args, runner)
        }
        Err(error) => {
            eprintln!("failed to run taskit: {error}");
            1
        }
    }
}

fn install_taskit_and_retry<R>(args: Vec<OsString>, runner: &mut R) -> i32
where
    R: ProcessRunner,
{
    eprintln!("taskit not found, installing via cargo install taskit...");

    let install = CommandInvocation::new(CARGO_BIN, ["install", "taskit"]);
    match runner.run(&install) {
        Ok(outcome) if outcome.success_status() => {}
        Ok(outcome) => {
            eprintln!("failed to install taskit");
            return outcome.exit_code();
        }
        Err(error) => {
            eprintln!("failed to run cargo install taskit: {error}");
            return 1;
        }
    }

    let taskit = CommandInvocation::new(TASKIT_BIN, args);
    match runner.run(&taskit) {
        Ok(outcome) => outcome.exit_code(),
        Err(error) => {
            eprintln!("failed to run taskit after install: {error}");
            1
        }
    }
}
