pub type Nnue = NnueImpl;
pub type Accumulator = AccumulatorImpl;

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
mod x86_64_avx2;
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
use x86_64_avx2::*;

/*
pub struct Nnue {
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    weight_l1: [[core::arch::aarch64::int16x8_t; 2]; 8],
}

impl Nnue {
    #[allow(unreachable_code)]
    pub fn forward_pass(&self, acc: &Accumulator) -> i16 {
        #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
        return self.fw_neon(acc);

        todo!("generic implementation");
    }

    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    fn fw_neon(&self, acc: &Accumulator) -> i16 {
        use core::arch::aarch64::*;

        let input = acc.neon;
        let in_low_0 = unsafe { vget_low_s16(input.0) };
        let in_low_1 = unsafe { vget_low_s16(input.1) };
        let mut acc_0 = unsafe { vmovq_n_s32(0) };
        let mut acc_1 = unsafe { vmovq_n_s32(0) };

        for i in 0..8 {
            let w_0 = self.weight_l1[i][0];
            let w_1 = self.weight_l1[i][1];

            acc_0 = unsafe { vpaddq_s32(acc_0, vmull_s16(in_low_0, vget_low_s16(w_0))) };
            acc_0 = unsafe { vpaddq_s32(acc_0, vmull_high_s16(input.0, w_0)) };
            acc_1 = unsafe { vpaddq_s32(acc_1, vmull_s16(in_low_1, vget_low_s16(w_1))) };
            acc_1 = unsafe { vpaddq_s32(acc_1, vmull_high_s16(input.1, w_1)) };
        }

        todo!();
    }
}

pub struct Accumulator {
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    neon: core::arch::aarch64::int16x8x2_t,

    #[cfg(not(any(
        all(target_arch = "aarch64", target_feature = "neon")
    )))]
    fallback: [i16; 16],
}*/
