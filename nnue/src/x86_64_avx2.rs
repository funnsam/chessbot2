use core::arch::x86_64::*;

pub struct NnueImpl {
    l2_w: [__m256i; 8], // i16x16
    l2_b: __m256i,
    l3_w: __m256i,
    l3_b: i16,
}

pub struct AccumulatorImpl(__m256i); // i16x16

impl NnueImpl {
    pub fn new() -> Self {
    }

    pub fn forward(&self, acc: AccumulatorImpl) -> i16 {
        let mut l2 = self.l2_b;

        for w in self.l2_w.iter() {
            let i = unsafe { _mm256_madd_epi16(acc.0, *w) }; // AABB CCDD
            // l2 = unsafe { _mm256_add_epi32(l2, i) };
        }
        todo!();

        // let l2 = unsafe { _mm256_castsi256_si128(l2) };

        let l3 = unsafe { _mm256_madd_epi16(l2, self.l3_w) }; // 0← SSSS 0000 SSSS 0000
        let l3 = unsafe { _mm256_castsi256_si128(l3) };
        let z = unsafe { _mm_set1_epi16(0) };
        let l3 = unsafe { _mm_hadds_epi16(l3, z) }; // SSSS 0000
        let l3 = unsafe { _mm_hadds_epi16(l3, z) }; // SS00 0000
        let l3 = unsafe { _mm_hadds_epi16(l3, z) }; // S000 0000

        unsafe { _mm_cvtsi128_si32(l3) as i16 + self.l3_b }
    }
}
