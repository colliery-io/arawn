use std::sync::{Arc, Mutex};

use arawn_core::SCRATCH_NAME;

/// Holder for the session-active workstream name. Cheap to clone
/// (`Arc<Mutex<String>>`). T-0250 will retire this in favor of the
/// `Session::workstream_name` field.
#[derive(Clone, Debug)]
pub struct SessionWorkstream {
    inner: Arc<Mutex<String>>,
}

impl SessionWorkstream {
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(initial.into())),
        }
    }

    pub fn scratch() -> Self {
        Self::new(SCRATCH_NAME)
    }

    pub fn current(&self) -> String {
        self.inner.lock().unwrap().clone()
    }

    pub fn set(&self, name: impl Into<String>) {
        *self.inner.lock().unwrap() = name.into();
    }
}

impl Default for SessionWorkstream {
    fn default() -> Self {
        Self::scratch()
    }
}

