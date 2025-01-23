use chess::{ChessMove, Piece, Square};

pub type HistoryTable = ButterflyTable<isize>;
pub type KillerTable = ButterflyTable<isize>;
pub type CountermoveTable = ButterflyTable<ChessMove>;
pub type CmHistoryTable = PieceToTable<PieceToTable<isize>>;

#[derive(Clone)]
pub struct ButterflyTable<T>([T; 64 * 64]);

impl<T: Default> Default for ButterflyTable<T> {
    fn default() -> Self {
        Self(core::array::from_fn(|_| T::default()))
    }
}

impl<T: Default> ButterflyTable<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.0.fill_with(T::default);
    }
}

impl<T> core::ops::Index<ChessMove> for ButterflyTable<T> {
    type Output = T;

    fn index(&self, m: ChessMove) -> &Self::Output {
        &self.0[m.get_source().to_index() * 64 + m.get_dest().to_index()]
    }
}

impl<T> core::ops::IndexMut<ChessMove> for ButterflyTable<T> {
    fn index_mut(&mut self, m: ChessMove) -> &mut Self::Output {
        &mut self.0[m.get_source().to_index() * 64 + m.get_dest().to_index()]
    }
}

impl ButterflyTable<isize> {
    pub fn update(&mut self, m: ChessMove, bonus: isize) {
        const MAX: isize = 32760;
        let bonus = bonus.min(MAX).max(-MAX);
        self[m] += bonus - self[m] * bonus.abs() / MAX;
    }
}

#[derive(Clone)]
pub struct PieceToTable<T>([T; 6 * 64]);

impl<T: Default> Default for PieceToTable<T> {
    fn default() -> Self {
        Self(core::array::from_fn(|_| T::default()))
    }
}

impl<T: Default> PieceToTable<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> core::ops::Index<(Piece, Square)> for PieceToTable<T> {
    type Output = T;

    fn index(&self, (p, t): (Piece, Square)) -> &Self::Output {
        &self.0[p.to_index() * 64 + t.to_index()]
    }
}

impl<T> core::ops::IndexMut<(Piece, Square)> for PieceToTable<T> {
    fn index_mut(&mut self, (p, t): (Piece, Square)) -> &mut Self::Output {
        &mut self.0[p.to_index() * 64 + t.to_index()]
    }
}

impl PieceToTable<isize> {
    pub fn update(&mut self, p: Piece, t: Square, bonus: isize) {
        const MAX: isize = 32760;
        let bonus = bonus.min(MAX).max(-MAX);
        self[(p, t)] += bonus - self[(p, t)] * bonus.abs() / MAX;
    }
}
