use std::sync::{atomic::*, Barrier};

pub struct CondBarrier {
    initiate: AtomicBool,
    barrier: Barrier,
}

impl CondBarrier {
    pub fn new(n: usize) -> Self {
        Self {
            initiate: AtomicBool::new(false),
            barrier: Barrier::new(n),
        }
    }

    pub fn initiate(&self) {
        self.initiate.store(true, Ordering::Relaxed);
    }

    pub fn uninitiate(&self) {
        self.initiate.store(false, Ordering::Relaxed);
    }

    pub fn initiate_wait(&self) {
        self.initiate();
        self.barrier.wait();
        self.uninitiate();
    }

    pub fn initiated(&self) -> bool {
        self.initiate.load(Ordering::Relaxed)
    }

    pub fn maybe_wait(&self) {
        if self.initiated() {
            self.barrier.wait();
        }
    }

    pub fn always_wait(&self) {
        self.barrier.wait();
    }
}
