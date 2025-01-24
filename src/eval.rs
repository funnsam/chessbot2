use chess::*;

/// Evaluation score in centipawns. +ve is side to move better and -ve is worse
/// ```text
///    ┌┬┬─ mate in n              ┌┬┬─ mate in !n
/// 10_000…b                    01_111…b
/// -32767                      32767
/// #-0                         #0
/// ←──────|──────|──────|──────→
///      min cp   0   max cp
///      -16383        16383
/// ```
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct Eval(pub i16);

impl Eval {
    pub const MAX: Self = Self(i16::MAX);
    pub const MIN: Self = Self(-Self::MAX.0);
    pub const M0: Self = Self(Self::MAX.0);

    #[inline]
    pub fn incr_mate(self) -> Self {
        match self.0 as u16 >> 14 {
            1 => Self(self.0 - 1),
            2 => Self(self.0 + 1),
            _ => self,
        }
    }

    #[inline]
    pub fn is_mate(self) -> bool {
        matches!(self.0 as u16 >> 14, 1 | 2)
    }

    #[inline]
    pub fn is_positive_mate(self) -> bool {
        self.0 as u16 >> 14 == 1
    }
}

impl core::ops::Add<i16> for Eval {
    type Output = Self;

    #[inline]
    fn add(self, rhs: i16) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl core::ops::Sub<i16> for Eval {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: i16) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl core::ops::Sub<Eval> for i16 {
    type Output = Eval;

    #[inline]
    fn sub(self, rhs: Eval) -> Self::Output {
        Eval(self - rhs.0)
    }
}

impl core::ops::Neg for Eval {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        if self.is_mate() {
            Self(!self.0)
        } else {
            Self(-self.0)
        }
    }
}

impl core::fmt::Display for Eval {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            match self.0 as u16 >> 14 {
                1 => write!(f, "mate {}", !self.0 & 0x3fff),
                2 => write!(f, "mate -{}", self.0 & 0x3fff),
                _ => write!(f, "cp {}", self.0),
            }
        } else {
            match self.0 as u16 >> 14 {
                1 => write!(f, "#{}", ((!self.0 & 0x3fff) + 1) / 2),
                2 => write!(f, "#-{}", ((self.0 & 0x3fff) + 1) / 2),
                _ => write!(f, "{}cp", self.0),
            }
        }
    }
}

#[test]
#[cfg(test)]
fn test_eval() {
    let m0 = Eval::M0;
    let m1 = m0.incr_mate();
    let m_0 = -m0;
    let m_1 = m_0.incr_mate();

    assert_eq!(m0.0, 0x7fff);
    assert_eq!(m0.to_string(), "#0");
    assert_eq!(m1.0, 0x7ffe);
    assert_eq!(m1.to_string(), "#1");
    assert_eq!(m_0.0 as u16, 0x8000);
    assert_eq!(m_0.to_string(), "#-0");
    assert_eq!(m_1.0 as u16, 0x8001);
    assert_eq!(m_1.to_string(), "#-1");

    let m0 = -m_0;
    assert_eq!(m0.0, 0x7fff);

    assert_eq!(--m0, m0);
    assert_eq!(--m1, m1);
    assert_eq!(--m_0, m_0);
    assert_eq!(--m_1, m_1);
    assert_eq!(-m0, m_0);
    assert_eq!(-m1, m_1);
    assert_eq!(-m_0, m0);
    assert_eq!(-m_1, m1);
}

pub type EvalParams = EvalParamList<i16>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvalParamList<T> {
    #[serde(bound = "Pst<T>: serde::Serialize + for<'a> serde::Deserialize<'a>")]
    pub pst_mid: Pst<T>,
    #[serde(bound = "Pst<T>: serde::Serialize + for<'a> serde::Deserialize<'a>")]
    pub pst_end: Pst<T>,
    pub rook_open_file_bonus: T,
    pub king_pawn_penalty: T,
    pub king_open_file_penalty: T,
}

impl Default for EvalParams {
    fn default() -> Self {
        postcard::from_bytes(include_bytes!("eval_params.bin")).unwrap_or_else(|_| {

            Self {
                pst_mid: Pst(core::array::from_fn(|i| {
                    params::PIECE_SQUARE_TABLE_MID[i] + params::PIECE_VALUE_MID[i / 64]
                })),
                pst_end: Pst(core::array::from_fn(|i| {
                    params::PIECE_SQUARE_TABLE_END[i] + params::PIECE_VALUE_END[i / 64]
                })),
                rook_open_file_bonus: 20,
                king_pawn_penalty: 15,
                king_open_file_penalty: 5,
            }
        })
    }
}

impl Default for EvalParamList<f64> {
    fn default() -> Self {
        EvalParams::default().to_f64()
    }
}

impl EvalParams {
    pub fn to_f64(&self) -> EvalParamList<f64> {
        EvalParamList {
            pst_mid: Pst(core::array::from_fn(|i| self.pst_mid.0[i] as _)),
            pst_end: Pst(core::array::from_fn(|i| self.pst_end.0[i] as _)),
            rook_open_file_bonus: self.rook_open_file_bonus as _,
            king_open_file_penalty: self.king_open_file_penalty as _,
            king_pawn_penalty: self.king_pawn_penalty as _,
        }
    }
}

impl EvalParamList<f64> {
    pub fn zeroed() -> Self {
        Self {
            pst_mid: Pst([0.0; 6 * 64]),
            pst_end: Pst([0.0; 6 * 64]),
            rook_open_file_bonus: 0.0,
            king_open_file_penalty: 0.0,
            king_pawn_penalty: 0.0,
        }
    }

    pub fn round_into_i16(&self) -> EvalParams {
        EvalParams {
            pst_mid: Pst(core::array::from_fn(|i| self.pst_mid.0[i].round() as _)),
            pst_end: Pst(core::array::from_fn(|i| self.pst_end.0[i].round() as _)),
            rook_open_file_bonus: self.rook_open_file_bonus.round() as _,
            king_open_file_penalty: self.king_open_file_penalty.round() as _,
            king_pawn_penalty: self.king_pawn_penalty.round() as _,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Pst<T>(
    #[serde(with = "serde_big_array::BigArray")]
    #[serde(bound = "T: serde::Serialize + for<'a> serde::Deserialize<'a>")]
    pub [T; 6 * 64]
);

impl<T: serde::Serialize + for<'a> serde::Deserialize<'a>> Pst<T> {
    fn index_of((color, piece, square): (Color, Piece, Square)) -> usize {
        let xor = if color == Color::Black { 0b111_000 } else { 0 };
        (square.to_index() ^ xor) | (piece.to_index() << 6)
    }
}

impl<T> core::ops::Index<(Color, Piece, Square)> for Pst<T> where
    T: serde::Serialize + for<'a> serde::Deserialize<'a>
{
    type Output = T;

    fn index(&self, index: (Color, Piece, Square)) -> &Self::Output {
        &self.0[Self::index_of(index)]
    }
}

impl<T> core::ops::IndexMut<(Color, Piece, Square)> for Pst<T> where
    T: serde::Serialize + for<'a> serde::Deserialize<'a>
{
    fn index_mut(&mut self, index: (Color, Piece, Square)) -> &mut Self::Output {
        &mut self.0[Self::index_of(index)]
    }
}

impl EvalParams {
    /// Mostly PeSTO's evaluation with rook on open file bonus
    pub fn evaluate_static(&self, board: &Board) -> Eval {
        self.evaluate_with(self.get_separated(board))
    }

    pub fn evaluate_with(&self, (mg_eval, eg_eval, mg_phase): (i16, i16, u8)) -> Eval {
        let eg_phase = MAX_PHASE - mg_phase;

        Eval(((mg_eval as i32 * mg_phase as i32 + eg_eval as i32 * eg_phase as i32) / MAX_PHASE as i32) as i16)
    }

    pub fn get_separated(&self, board: &Board) -> (i16, i16, u8) {
        let (mg, eg, phase) = self.get_separated_in_white(board);

        if board.side_to_move() == Color::White {
            (mg, eg, phase)
        } else {
            (-mg, -eg, phase)
        }
    }

    pub fn get_separated_in_white(&self, board: &Board) -> (i16, i16, u8) {
        let mut mid_game = [0, 0];
        let mut end_game = [0, 0];
        let mut phase = 0;

        for square in board.combined().into_iter() {
            // SAFETY: only squares with things on it are checked
            let piece = unsafe { board.piece_on(square).unwrap_unchecked() };
            let color = unsafe { board.color_on(square).unwrap_unchecked() };

            // rook on open file bonus
            let rook_on_open_file = (piece == Piece::Rook
                && (board.pieces(Piece::Pawn) & chess::get_file(square.get_file())).0 == 0
            ) as i16 * self.rook_open_file_bonus;
            let pawn_shield = if piece == Piece::King {
                let mut open_files = 0;
                if let Some(sq) = square.left() {
                    open_files += ((board.pieces(Piece::Pawn) & board.color_combined(color) & chess::get_file(sq.get_file())).0 == 0) as i16;
                }
                if let Some(sq) = square.right() {
                    open_files += ((board.pieces(Piece::Pawn) & board.color_combined(color) & chess::get_file(sq.get_file())).0 == 0) as i16;
                }

                let king_center = square.uforward(color);
                let king_pawns = (board.pieces(Piece::Pawn) & (chess::get_king_moves(king_center) | BitBoard::from_square(king_center))).popcnt();

                -(3_i16.saturating_sub(king_pawns as i16) * self.king_pawn_penalty + open_files * self.king_open_file_penalty)
            } else { 0 };

            let p = (color, piece, square);

            mid_game[color.to_index()] += rook_on_open_file + pawn_shield + self.pst_mid[p];
            end_game[color.to_index()] += rook_on_open_file + self.pst_end[p];
            phase += PIECE_PHASE[piece.to_index()];
        }

        let mg_eval = mid_game[0] - mid_game[1];
        let eg_eval = end_game[0] - end_game[1];

        (mg_eval, eg_eval, phase.min(MAX_PHASE))
    }
}

/// Finds the current phase of the game. 0 is endgame and 24 is midgame.
pub fn game_phase(board: &Board) -> u8 {
    board.combined().into_iter()
        .map(|sq| PIECE_PHASE[unsafe { board.piece_on(sq).unwrap_unchecked() }.to_index()])
        .sum::<u8>()
        .min(MAX_PHASE)
}

/// # Note
/// This table doesn't strictly represent the piece value of the ones that are used in evaluation.
pub const PIECE_VALUE: [i16; 6] = [
    params::PIECE_VALUE_MID[0],
    params::PIECE_VALUE_MID[1],
    params::PIECE_VALUE_MID[2],
    params::PIECE_VALUE_MID[3],
    params::PIECE_VALUE_MID[4],
    20000,
];

pub const MAX_PHASE: u8 = 24;
pub const PIECE_PHASE: [u8; 6] = [0, 1, 1, 2, 4, 0];

mod params {
    pub const PIECE_VALUE_MID: [i16; 6] = [82, 337, 365, 477, 1025,  0];
    pub const PIECE_VALUE_END: [i16; 6] = [94, 281, 297, 512,  936,  0];

    // a1 ----> h1
    // |
    // v
    // a8
    pub static PIECE_SQUARE_TABLE_MID: [i16; 64 * 6] = [
        // Pawn
        0,   0,   0,   0,   0,   0,  0,   0,
        -35,  -1, -20, -23, -15,  24, 38, -22,
        -26,  -4,  -4, -10,   3,   3, 33, -12,
        -27,  -2,  -5,  12,  17,   6, 10, -25,
        -14,  13,   6,  21,  23,  12, 17, -23,
        -6,   7,  26,  31,  65,  56, 25, -20,
        98, 134,  61,  95,  68, 126, 34, -11,
        0,   0,   0,   0,   0,   0,  0,   0,
        // Knight
        -105, -21, -58, -33, -17, -28, -19,  -23,
        -29, -53, -12,  -3,  -1,  18, -14,  -19,
        -23,  -9,  12,  10,  19,  17,  25,  -16,
        -13,   4,  16,  13,  28,  19,  21,   -8,
        -9,  17,  19,  53,  37,  69,  18,   22,
        -47,  60,  37,  65,  84, 129,  73,   44,
        -73, -41,  72,  36,  23,  62,   7,  -17,
        -167, -89, -34, -49,  61, -97, -15, -107,
        // Bishop
        -33,  -3, -14, -21, -13, -12, -39, -21,
        4,  15,  16,   0,   7,  21,  33,   1,
        0,  15,  15,  15,  14,  27,  18,  10,
        -6,  13,  13,  26,  34,  12,  10,   4,
        -4,   5,  19,  50,  37,  37,   7,  -2,
        -16,  37,  43,  40,  35,  50,  37,  -2,
        -26,  16, -18, -13,  30,  59,  18, -47,
        -29,   4, -82, -37, -25, -42,   7,  -8,
        // Rook
        -19, -13,   1,  17, 16,  7, -37, -26,
        -44, -16, -20,  -9, -1, 11,  -6, -71,
        -45, -25, -16, -17,  3,  0,  -5, -33,
        -36, -26, -12,  -1,  9, -7,   6, -23,
        -24, -11,   7,  26, 24, 35,  -8, -20,
        -5,  19,  26,  36, 17, 45,  61,  16,
        27,  32,  58,  62, 80, 67,  26,  44,
        32,  42,  32,  51, 63,  9,  31,  43,
        // Queen
        -1, -18,  -9,  10, -15, -25, -31, -50,
        -35,  -8,  11,   2,   8,  15,  -3,   1,
        -14,   2, -11,  -2,  -5,   2,  14,   5,
        -9, -26,  -9, -10,  -2,  -4,   3,  -3,
        -27, -27, -16, -16,  -1,  17,  -2,   1,
        -13, -17,   7,   8,  29,  56,  47,  57,
        -24, -39,  -5,   1, -16,  57,  28,  54,
        -28,   0,  29,  12,  59,  44,  43,  45,
        // King
        -15,  36,  12, -54,   8, -28,  24,  14,
        1,   7,  -8, -64, -43, -16,   9,   8,
        -14, -14, -22, -46, -44, -30, -15, -27,
        -49,  -1, -27, -39, -46, -44, -33, -51,
        -17, -20, -12, -27, -30, -25, -14, -36,
        -9,  24,   2, -16, -20,   6,  22, -22,
        29,  -1, -20,  -7,  -8,  -4, -38, -29,
        -65,  23,  16, -15, -56, -34,   2,  13,
    ];

    pub static PIECE_SQUARE_TABLE_END: [i16; 64 * 6] = [
        // Pawn
        0,   0,   0,   0,   0,   0,   0,   0,
        13,   8,   8,  10,  13,   0,   2,  -7,
        4,   7,  -6,   1,   0,  -5,  -1,  -8,
        13,   9,  -3,  -7,  -7,  -8,   3,  -1,
        32,  24,  13,   5,  -2,   4,  17,  17,
        94, 100,  85,  67,  56,  53,  82,  84,
        178, 173, 158, 134, 147, 132, 165, 187,
        0,   0,   0,   0,   0,   0,   0,   0,
        // Knight
        -29, -51, -23, -15, -22, -18, -50, -64,
        -42, -20, -10,  -5,  -2, -20, -23, -44,
        -23,  -3,  -1,  15,  10,  -3, -20, -22,
        -18,  -6,  16,  25,  16,  17,   4, -18,
        -17,   3,  22,  22,  22,  11,   8, -18,
        -24, -20,  10,   9,  -1,  -9, -19, -41,
        -25,  -8, -25,  -2,  -9, -25, -24, -52,
        -58, -38, -13, -28, -31, -27, -63, -99,
        // Bishop
        -23,  -9, -23,  -5, -9, -16,  -5, -17,
        -14, -18,  -7,  -1,  4,  -9, -15, -27,
        -12,  -3,   8,  10, 13,   3,  -7, -15,
        -6,   3,  13,  19,  7,  10,  -3,  -9,
        -3,   9,  12,   9, 14,  10,   3,   2,
        2,  -8,   0,  -1, -2,   6,   0,   4,
        -8,  -4,   7, -12, -3, -13,  -4, -14,
        -14, -21, -11,  -8, -7,  -9, -17, -24,
        // Rook
        -9,  2,  3, -1, -5, -13,   4, -20,
        -6, -6,  0,  2, -9,  -9, -11,  -3,
        -4,  0, -5, -1, -7, -12,  -8, -16,
        3,  5,  8,  4, -5,  -6,  -8, -11,
        4,  3, 13,  1,  2,   1,  -1,   2,
        7,  7,  7,  5,  4,  -3,  -5,  -3,
        11, 13, 13, 11, -3,   3,   8,   3,
        13, 10, 18, 15, 12,  12,   8,   5,
        // Queen
        -33, -28, -22, -43,  -5, -32, -20, -41,
        -22, -23, -30, -16, -16, -23, -36, -32,
        -16, -27,  15,   6,   9,  17,  10,   5,
        -18,  28,  19,  47,  31,  34,  39,  23,
        3,  22,  24,  45,  57,  40,  57,  36,
        -20,   6,   9,  49,  47,  35,  19,   9,
        -17,  20,  32,  41,  58,  25,  30,   0,
        -9,  22,  22,  27,  27,  19,  10,  20,
        // King
        -53, -34, -21, -11, -28, -14, -24, -43,
        -27, -11,   4,  13,  14,   4,  -5, -17,
        -19,  -3,  11,  21,  23,  16,   7,  -9,
        -18,  -4,  21,  24,  27,  23,   9, -11,
        -8,  22,  24,  27,  26,  33,  26,   3,
        10,  17,  23,  15,  20,  45,  44,  13,
        -12,  17,  14,  17,  17,  38,  23,  11,
        -74, -35, -18, -18, -11,  15,   4, -17,
    ];
}
