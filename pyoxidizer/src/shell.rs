//! Cargo-style console output.
//!
//! Messages are written to stderr through [anstream], which strips styling
//! when the stream is not a terminal and honors `NO_COLOR`, `CLICOLOR` and
//! `CLICOLOR_FORCE`.

use std::{
    fmt,
    io::Write,
    ops::DerefMut,
    sync::{Arc, LazyLock, Mutex},
};

use anstream::ColorChoice;
use anstyle::{AnsiColor, Effects, Style};

pub type ShellResult<T> = anyhow::Result<T>;

pub const HEADER: Style = AnsiColor::Green.on_default().effects(Effects::BOLD);
pub const ERROR: Style = AnsiColor::Red.on_default().effects(Effects::BOLD);
pub const WARN: Style = AnsiColor::Yellow.on_default().effects(Effects::BOLD);
pub const NOTE: Style = AnsiColor::Cyan.on_default().effects(Effects::BOLD);
pub const GOOD: Style = AnsiColor::Green.on_default().effects(Effects::BOLD);
pub const SKIPPED: Style = AnsiColor::BrightBlack.on_default();

/// How much output the shell produces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verbosity {
    Verbose,
    Normal,
    Quiet,
}

/// Writes Cargo-style status messages to stderr.
#[derive(Debug)]
pub struct Shell {
    verbosity: Verbosity,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    pub fn new() -> Self {
        Self {
            verbosity: Verbosity::Normal,
        }
    }

    /// Updates the verbosity of the shell.
    pub fn set_verbosity(&mut self, verbosity: Verbosity) {
        self.verbosity = verbosity;
    }

    /// Prints a right-aligned green status message.
    pub fn status<T, U>(&mut self, status: T, message: U) -> ShellResult<()>
    where
        T: fmt::Display,
        U: fmt::Display,
    {
        self.status_with_color(status, message, &HEADER)
    }

    /// Prints a right-aligned status message in the given style.
    pub fn status_with_color<T, U>(
        &mut self,
        status: T,
        message: U,
        style: &Style,
    ) -> ShellResult<()>
    where
        T: fmt::Display,
        U: fmt::Display,
    {
        match self.verbosity {
            Verbosity::Quiet => Ok(()),
            _ => print_stderr(&status, &message, style, true),
        }
    }

    /// Runs the callback only if we are in verbose mode.
    pub fn verbose<F>(&mut self, mut callback: F) -> ShellResult<()>
    where
        F: FnMut(&mut Shell) -> ShellResult<()>,
    {
        match self.verbosity {
            Verbosity::Verbose => callback(self),
            _ => Ok(()),
        }
    }

    /// Runs the callback if we are not in verbose mode.
    pub fn concise<F>(&mut self, mut callback: F) -> ShellResult<()>
    where
        F: FnMut(&mut Shell) -> ShellResult<()>,
    {
        match self.verbosity {
            Verbosity::Verbose => Ok(()),
            _ => callback(self),
        }
    }

    /// Prints a red `error:` message. Errors are printed even in quiet mode.
    pub fn error<T: fmt::Display>(&mut self, message: T) -> ShellResult<()> {
        print_stderr(&"error", &message, &ERROR, false)
    }

    /// Prints a yellow `warning:` message.
    pub fn warn<T: fmt::Display>(&mut self, message: T) -> ShellResult<()> {
        match self.verbosity {
            Verbosity::Quiet => Ok(()),
            _ => print_stderr(&"warning", &message, &WARN, false),
        }
    }

    /// Prints a cyan `note:` message.
    pub fn note<T: fmt::Display>(&mut self, message: T) -> ShellResult<()> {
        match self.verbosity {
            Verbosity::Quiet => Ok(()),
            _ => print_stderr(&"note", &message, &NOTE, false),
        }
    }

    /// Whether colored output should be written to stdout.
    pub fn out_supports_color(&self) -> bool {
        anstream::AutoStream::choice(&std::io::stdout()) != ColorChoice::Never
    }
}

/// Prints a message with a styled status to stderr.
///
/// Justified statuses are right-aligned to 12 characters. Otherwise the
/// status is followed by a colon.
fn print_stderr(
    status: &dyn fmt::Display,
    message: &dyn fmt::Display,
    style: &Style,
    justified: bool,
) -> ShellResult<()> {
    let bold = Style::new().effects(Effects::BOLD);

    let mut buffer = Vec::new();
    if justified {
        write!(buffer, "{style}{status:>12}{style:#}")?;
    } else {
        write!(buffer, "{style}{status}{style:#}{bold}:{bold:#}")?;
    }
    writeln!(buffer, " {message}")?;

    anstream::stderr().lock().write_all(&buffer)?;

    Ok(())
}

static SHELL: LazyLock<Arc<Mutex<Shell>>> = LazyLock::new(|| Arc::new(Mutex::new(Shell::new())));

pub fn with_shell<F, T>(f: F) -> T
where
    F: FnOnce(&mut Shell) -> T,
{
    let mut guard = SHELL.lock().unwrap();
    f(guard.deref_mut())
}

pub fn with_concise_shell<F>(mut f: F) -> ShellResult<()>
where
    F: FnMut(&mut Shell) -> ShellResult<()>,
{
    let mut guard = SHELL.lock().unwrap();
    guard.concise(|shell| f(shell))
}

pub fn with_verbose_shell<F>(mut f: F) -> ShellResult<()>
where
    F: FnMut(&mut Shell) -> ShellResult<()>,
{
    let mut guard = SHELL.lock().unwrap();
    guard.verbose(|shell| f(shell))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbosity_callbacks() -> ShellResult<()> {
        let mut shell = Shell::new();
        let mut calls = vec![];

        for verbosity in [Verbosity::Quiet, Verbosity::Normal, Verbosity::Verbose] {
            shell.set_verbosity(verbosity);
            assert_eq!(shell.verbosity, verbosity);

            shell.verbose(|_| {
                calls.push((verbosity, "verbose"));
                Ok(())
            })?;
            shell.concise(|_| {
                calls.push((verbosity, "concise"));
                Ok(())
            })?;
        }

        assert_eq!(
            calls,
            vec![
                (Verbosity::Quiet, "concise"),
                (Verbosity::Normal, "concise"),
                (Verbosity::Verbose, "verbose"),
            ]
        );

        Ok(())
    }
}
