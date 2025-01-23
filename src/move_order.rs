use crate::tables::KillerTable;
use crate::{trans_table::TransTableEntry, Game};
use crate::eval::PIECE_VALUE;
use chess::{ChessMove, Piece};

impl<const MAIN: bool> crate::SmpThread<'_, MAIN> {
    pub(crate) fn move_score(
        &self,
        m: ChessMove,
        prev_move: ChessMove,
        prev_piece: Piece,
        game: &Game,
        tte: &Option<TransTableEntry>,
        killer: &KillerTable,
    ) -> i32 {
        if tte.is_some_and(|tte| tte.next == m) {
            i32::MAX
        } else if game.is_capture(m) {
            mvv_lva(game, m) as i32 * 327601
        } else {
            let mut score = 0;
            let piece = game.board().piece_on(m.get_source()).unwrap();

            score += self.cm_history[(prev_piece, prev_move.get_dest())][(piece, m.get_dest())] as i32 * 1000;

            if self.countermove[prev_move] == m {
                score += 1000;
            }

            score += self.hist_table[m] as i32;
            score += killer[m] as i32 * 100;

            score
        }
    }
}

fn mvv_lva(game: &Game, m: ChessMove) -> i16 {
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

    let victim = game.board().piece_on(m.get_dest()).map_or(5, |p| p.to_index());
    let aggressor = game.board().piece_on(m.get_source()).unwrap().to_index();

    MVV_LVA[victim][aggressor]
}
