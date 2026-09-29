//! One process-wide lock for tests that read or write process environment.
//!
//! `std::env::set_var` is process-global. Profile tests and format tests share
//! this lock so they do not change each other's network or entity variables.

use std::sync::{Mutex, MutexGuard};

static ENV: Mutex<()> = Mutex::new(());

pub(crate) struct EnvLock {
    _guard: MutexGuard<'static, ()>,
    saved: Vec<(String, Option<String>)>,
}

impl Drop for EnvLock {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            unsafe {
                match value {
                    Some(saved) => std::env::set_var(key, saved),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

pub(crate) fn lock_env(pairs: &[(&str, Option<&str>)]) -> EnvLock {
    let guard = ENV.lock().expect("env lock");
    let mut saved = Vec::new();
    for (key, value) in pairs {
        saved.push(((*key).to_string(), std::env::var(key).ok()));
        unsafe {
            match value {
                Some(next) => std::env::set_var(key, next),
                None => std::env::remove_var(key),
            }
        }
    }
    EnvLock {
        _guard: guard,
        saved,
    }
}
