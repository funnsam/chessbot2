pub struct NnueImpl {
    l1_w: [[i16; 16]; 768],
    l1_b: [i16; 16],

    l2_w: [[i16; 8]; 16],
    l2_b: [i16; 8],

    l3_w: [i16; 8],
    l3_b: i16,
}

pub struct AccumulatorImpl([i16; 16]);

impl NnueImpl {
    pub fn new_acc(&self) -> AccumulatorImpl {
        AccumulatorImpl(self.l1_b.clone())
    }

    pub fn update_acc(&self, acc: &mut AccumulatorImpl, color: usize, piece: usize, from: usize, to: usize, capture: Option<usize>) {
        let add = (color * 6 + piece) * 64 + to;
        let sub = (color * 6 + piece) * 64 + from;

        if let Some(cap_p) = capture {
            let cap = ((1 - color) * 6 + cap_p) * 64 + to;

            for (i, (add, (sub, cap))) in acc.0.iter_mut().zip(self.l1_w[add].iter().zip(self.l1_w[sub].iter().zip(self.l1_w[cap].iter()))) {
                *i += *add - *sub - *cap;
            }
        } else {
            for (i, (add, sub)) in acc.0.iter_mut().zip(self.l1_w[add].iter().zip(self.l1_w[sub].iter())) {
                *i += *add - *sub;
            }
        }
    }

    pub fn forward(&self, acc: AccumulatorImpl) -> i16 {
        let mut l2 = self.l2_b.clone();

        for (i, j) in acc.0.iter().zip(self.l2_w.iter()) {
            for (j, k) in l2.iter_mut().zip(j.iter()) {
                *j = j.saturating_add(crelu(*i) * *k);
            }
        }

        self.l3_b + self.l3_w.iter().zip(l2).fold(0_i16, |a, (i, j)| a.saturating_add(*i * crelu(j)))
    }
}

fn crelu(x: i16) -> i16 {
    x.max(0).min(127)
}
