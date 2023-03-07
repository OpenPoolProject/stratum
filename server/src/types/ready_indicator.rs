use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

//@todo testing this
#[derive(Default, Clone)]
pub struct ReadyIndicator(Arc<AtomicBool>);

impl ReadyIndicator {
    #[must_use]
    pub fn new(ready: bool) -> Self {
        ReadyIndicator(Arc::new(AtomicBool::new(ready)))
    }

    #[must_use]
    pub fn create_new(&self) -> Self {
        //@todo review this, no idea why we do this.
        Self(Arc::clone(&self.0))
    }

    //@todo figure out if relaxed is ok
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
