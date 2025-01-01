use core::cmp::*;
use core::cell::UnsafeCell;
use crate::Game;
use crate::eval::PIECE_VALUE;
use chess::{ChessMove, Piece, Square};

pub struct ButterflyTable<T>(UnsafeCell<[T; 64 * 64]>);

impl<T: Clone> Clone for ButterflyTable<T> {
    fn clone(&self) -> Self {
        unsafe { Self(UnsafeCell::new((*self.0.get()).clone())) }
    }
}

// SAFETY: we don't really care much about race conditions
unsafe impl<T: Sync> Sync for ButterflyTable<T> {}

impl<T: Default> Default for ButterflyTable<T> {
    fn default() -> Self {
        Self(UnsafeCell::new(core::array::from_fn(|_| T::default())))
    }
}

impl<T: Default> ButterflyTable<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        unsafe { (*self.0.get()).fill_with(T::default); }
    }
}

impl<T> ButterflyTable<T> {
    pub fn get_mut(&self, m: ChessMove) -> &mut T {
        unsafe {
            &mut (*self.0.get())[m.get_source().to_index() * 64 + m.get_dest().to_index()]
        }
    }
}

impl<T> core::ops::Index<ChessMove> for ButterflyTable<T> {
    type Output = T;

    fn index(&self, m: ChessMove) -> &Self::Output {
        unsafe {
            &(*self.0.get())[m.get_source().to_index() * 64 + m.get_dest().to_index()]
        }
    }
}

impl<T> core::ops::IndexMut<ChessMove> for ButterflyTable<T> {
    fn index_mut(&mut self, m: ChessMove) -> &mut Self::Output {
        unsafe {
            &mut (*self.0.get())[m.get_source().to_index() * 64 + m.get_dest().to_index()]
        }
    }
}

impl ButterflyTable<usize> {
    pub fn update(&self, m: ChessMove, depth: usize) {
        *self.get_mut(m) += depth * depth;
    }
}

pub type HistoryTable = ButterflyTable<usize>;
pub type KillerTable = ButterflyTable<usize>;
pub type CountermoveTable = ButterflyTable<ChessMove>;

pub struct CmHistoryTable([HistoryTable; 6 * 64]);

impl CmHistoryTable {
    pub fn new() -> Self {
        Self(core::array::from_fn(|_| HistoryTable::default()))
    }

    pub fn idx(prev_piece: Piece, prev_to: Square) -> usize {
        prev_piece.to_index() * 64 + prev_to.to_index()
    }

    pub fn update(&self, prev_piece: Piece, prev_to: Square, m: ChessMove, depth: usize) {
        self.0[Self::idx(prev_piece, prev_to)].update(m, depth)
    }
}

impl crate::Engine {
    pub(crate) fn order_moves(&self, prev_move: ChessMove, moves: &mut [ChessMove], game: &Game, killer: &KillerTable) {
        // we order moves with the following order:
        // 1. good hash moves
        // 2. bad hash moves
        // 3. good MVV-LVA moves
        // 4. by killer heuristic
        // 5. bad MVV-LVA moves

        let prev_piece = game.board().piece_on(prev_move.get_dest()).unwrap();

        moves.sort_unstable_by(|a, b| {
            self.cmp_hash(game, *a, *b)
                .then_with(|| mvv_lva(game, *a, *b))
                .then_with(|| self.cm_history_heuristic(prev_piece, prev_move, *a, *b))
                .then_with(|| self.countermove_heuristic(prev_move, *a, *b))
                .then_with(|| self.butterfly_heuristic(&self.hist_table, *a, *b))
                .then_with(|| self.butterfly_heuristic(killer, *a, *b))
        });
    }

    fn cmp_hash(&self, game: &Game, a: ChessMove, b: ChessMove) -> Ordering {
        self.trans_table.get(game.board().get_hash())
            .map_or(Ordering::Equal, |e| (b == e.next).cmp(&(a == e.next)))
    }

    fn butterfly_heuristic(&self, bft: &ButterflyTable<usize>, a: ChessMove, b: ChessMove) -> Ordering {
        bft[b].cmp(&bft[a])
    }

    fn cm_history_heuristic(&self, prev_piece: Piece, prev_move: ChessMove, a: ChessMove, b: ChessMove) -> Ordering {
        let value = |m: ChessMove| self.cm_history.0[CmHistoryTable::idx(prev_piece, prev_move.get_dest())][m];

        value(b).cmp(&value(a))
    }

    fn countermove_heuristic(&self, prev_move: ChessMove, a: ChessMove, b: ChessMove) -> Ordering {
        let value = |m: ChessMove| self.countermove[prev_move] == m;

        value(b).cmp(&value(a))
    }
}

fn mvv_lva(game: &Game, a: ChessMove, b: ChessMove) -> Ordering {
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

    let value = |m: ChessMove| {
        let victim = game.board().piece_on(m.get_dest()).map_or(5, |p| p.to_index());
        let aggressor = game.board().piece_on(m.get_source()).unwrap().to_index();

        MVV_LVA[victim][aggressor]
    };

    value(b).cmp(&value(a))
}
