use core::{fmt, sync::atomic::*};

pub struct RelaxedCounter(AtomicUsize);

impl Default for RelaxedCounter {
    fn default() -> Self {
        Self(AtomicUsize::new(0))
    }
}

impl Clone for RelaxedCounter {
    fn clone(&self) -> Self {
        Self(self.0.load(Ordering::Relaxed).into())
    }
}

impl fmt::Debug for RelaxedCounter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.load(Ordering::Relaxed).fmt(f)
    }
}

impl fmt::Display for RelaxedCounter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.load(Ordering::Relaxed).fmt(f)
    }
}

impl RelaxedCounter {
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
            pub fn clear(&self) {
                $(self.$name.0.store(0, Ordering::Relaxed);)*
            }
        }
    };
}

debugs!(
    no_research,
    research,
    all_full_zw,
    full,
    all_full,
);
