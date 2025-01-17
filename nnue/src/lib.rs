pub type Nnue = NnueImpl;
pub type Accumulator = AccumulatorImpl;

// #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
// mod x86_64_avx2;
// #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
// use x86_64_avx2::*;

mod generic;
use generic::*;
