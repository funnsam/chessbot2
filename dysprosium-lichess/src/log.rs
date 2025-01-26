#![allow(unused_macros)]

#[macro_export]
macro_rules! dbg {
    ($($args: tt)*) => {{
        eprint!("\x1b[90mDebug:\x1b[0m ");
        eprintln!($($args)*);
    }};
}

#[macro_export]
macro_rules! info {
    ($($args: tt)*) => {{
        eprint!("\x1b[1;32mInfo:\x1b[0m ");
        eprintln!($($args)*);
    }};
}

#[macro_export]
macro_rules! warn {
    ($($args: tt)*) => {{
        eprint!("\x1b[1;33mWarn:\x1b[0m ");
        eprintln!($($args)*);
    }};
}

#[macro_export]
macro_rules! error {
    ($($args: tt)*) => {{
        eprint!("\x1b[1;31mError:\x1b[0m ");
        eprintln!($($args)*);
    }};
}
