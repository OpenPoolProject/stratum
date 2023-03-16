use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Default, Clone)]
pub struct ReadyIndicator(Arc<AtomicBool>);

impl ReadyIndicator {
    #[must_use]
    pub fn new(ready: bool) -> Self {
        ReadyIndicator(Arc::new(AtomicBool::new(ready)))
    }

    #[must_use]
    pub fn create_new(&self) -> Self {
        Self(Arc::clone(&self.0))
    }

    pub fn ready(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn not_ready(&self) {
        self.0.store(false, Ordering::Relaxed);
    }

    #[must_use]
    pub fn status(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
