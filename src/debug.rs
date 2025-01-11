use core::{fmt, sync::atomic::*};

pub struct RelaxedCounter(pub AtomicUsize);

impl Default for RelaxedCounter {
    fn default() -> Self {
        Self(AtomicUsize::new(0))
    }
}

impl Clone for RelaxedCounter {
    fn clone(&self) -> Self {
        Self(self.get().into())
    }
}

impl fmt::Debug for RelaxedCounter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.get().fmt(f)
    }
}

impl fmt::Display for RelaxedCounter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.get().fmt(f)
    }
}

impl RelaxedCounter {
    pub fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.0.store(0, Ordering::Relaxed);
    }

    pub fn inc(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

macro_rules! debugs {
    ($($name:ident),* $(,)?) => {
        #[derive(Debug, Clone, Default)]
        pub struct DebugStats {
            $(pub $name: RelaxedCounter,)*
        }

        impl DebugStats {
            pub fn reset(&self) {
                $(self.$name.reset();)*
            }
        }
    };
}

debugs!(
    nodes,
);
