use core::cmp::*;
use core::cell::UnsafeCell;

use crate::{trans_table::TransTableEntry, Game};
use crate::eval::PIECE_VALUE;
use dychess::prelude::*;

pub struct ButterflyTable<T>(UnsafeCell<[T; 64 * 64]>);

impl<T: Clone> Clone for ButterflyTable<T> {
    fn clone(&self) -> Self {
        unsafe { Self(UnsafeCell::new((*self.0.get()).clone())) }
    }
}

// SAFETY: we don't really care much about race conditions
unsafe impl<T: Sync> Sync for ButterflyTable<T> {}

impl<T: Default> ButterflyTable<T> {
    pub fn new() -> Self {
        Self(UnsafeCell::new(core::array::from_fn(|_| T::default())))
    }

    pub fn clear(&self) {
        unsafe { (*self.0.get()).fill_with(T::default); }
    }
}

impl<T> ButterflyTable<T> {
    pub fn get_mut(&self, m: Move) -> &mut T {
        unsafe {
            &mut (*self.0.get())[m.from().to_usize() * 64 + m.to().to_usize()]
        }
    }
}

impl<T> core::ops::Index<Move> for ButterflyTable<T> {
    type Output = T;

    fn index(&self, m: Move) -> &Self::Output {
        unsafe {
            &(*self.0.get())[m.from().to_usize() * 64 + m.to().to_usize()]
        }
    }
}

impl<T> core::ops::IndexMut<Move> for ButterflyTable<T> {
    fn index_mut(&mut self, m: Move) -> &mut Self::Output {
        unsafe {
            &mut (*self.0.get())[m.from().to_usize() * 64 + m.to().to_usize()]
        }
    }
}

impl ButterflyTable<isize> {
    pub fn update(&self, m: Move, bonus: isize) {
        const MAX: isize = 32760;
        let bonus = bonus.min(MAX).max(-MAX);
        *self.get_mut(m) += bonus - self[m] * bonus.abs() / MAX;
    }
}

pub type HistoryTable = ButterflyTable<isize>;
pub type KillerTable = ButterflyTable<isize>;
pub type CountermoveTable = ButterflyTable<Move>;

impl<const MAIN: bool> crate::SmpThread<'_, MAIN> {
    pub(crate) fn move_score(
        &self,
        m: Move,
        prev_move: Move,
        game: &Game,
        tte: &Option<TransTableEntry>,
        killer: &KillerTable,
    ) -> i32 {
        if tte.is_some_and(|tte| { let next = tte.next; next == m }) {
            i32::MAX
        } else if game.is_capture(m) {
            mvv_lva(game, m) as i32 * 327601
        } else {
            if self.countermove[prev_move] == m {
                return 327600;
            }

            self.hist_table[m] as i32 + killer[m] as i32 * 100
        }
    }
}

fn mvv_lva(game: &Game, m: Move) -> i16 {
    const P: i16 = PIECE_VALUE[0];
    const N: i16 = PIECE_VALUE[1];
    const B: i16 = PIECE_VALUE[2];
    const R: i16 = PIECE_VALUE[3];
    const Q: i16 = PIECE_VALUE[4];
    const K: i16 = PIECE_VALUE[5];
    const MVV_LVA: [[i16; 6]; 6] = [
    // victim                                      aggressor
    //   P      N      B      R      Q      x
        [P - P, N - P, B - P, R - P, Q - P, 0], // P
        [P - N, N - N, B - N, R - N, Q - N, 0], // N
        [P - B, N - B, B - B, R - B, Q - B, 0], // B
        [P - R, N - R, B - R, R - R, Q - R, 0], // R
        [P - Q, N - Q, B - Q, R - Q, Q - Q, 0], // Q
        [P - K, N - K, B - K, R - K, Q - K, 0], // K
    ];

    let victim = game.board().piece_on(m.to()).map_or(5, |p| p as usize);
    let aggressor = game.board().piece_on(m.from()).unwrap() as usize;

    MVV_LVA[victim][aggressor]
}
