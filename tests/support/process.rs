use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard, OnceLock};

static PROCESS_STATE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn lock_process_state() -> MutexGuard<'static, ()> {
    PROCESS_STATE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("process state lock")
}

pub struct EnvVarGuard {
    key: String,
    previous: Option<OsString>,
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => {
                // SAFETY: Test helper restores the original process env value before teardown.
                unsafe { std::env::set_var(&self.key, value) }
            }
            None => {
                // SAFETY: Test helper removes only the key it created during the test.
                unsafe { std::env::remove_var(&self.key) }
            }
        }
    }
}

pub fn set_env_var(key: impl Into<String>, value: impl AsRef<std::ffi::OsStr>) -> EnvVarGuard {
    let key = key.into();
    let previous = std::env::var_os(&key);
    // SAFETY: Tests use this helper in a scoped manner and restore the original value on drop.
    unsafe { std::env::set_var(&key, value) };
    EnvVarGuard { key, previous }
}
