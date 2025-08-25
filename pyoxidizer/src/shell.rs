use std::{ops::DerefMut, sync::{Arc, LazyLock, Mutex}};

use cargolike::{Shell, ShellResult};

static SHELL: LazyLock<Arc<Mutex<Shell>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(Shell::new()))
});

pub fn with_shell<F, T>(f: F) -> T
    where F: FnOnce(&mut Shell) -> T
{
    let mut guard = SHELL.lock().unwrap();
    f(guard.deref_mut())
}

pub fn with_concise_shell<F>(mut f: F) -> ShellResult<()>
    where F: FnMut(&mut Shell) -> ShellResult<()>
{
    let mut guard = SHELL.lock().unwrap();
    guard.concise(|shell| f(shell))
}

pub fn with_verbose_shell<F>(mut f: F) -> ShellResult<()>
    where F: FnMut(&mut Shell) -> ShellResult<()>
{
    let mut guard = SHELL.lock().unwrap();
    guard.verbose(|shell| f(shell))
}
