use core::cmp::*;
use crate::Game;
use crate::eval::PIECE_VALUE_MID;
use chess::ChessMove;

pub struct KillerTable(pub [usize; 64 * 64]);

impl KillerTable {
    pub fn new() -> Self {
        Self([0; 64 * 64])
    }

    pub fn update(&mut self, m: ChessMove, depth: usize) {
        self.0[m.get_source().to_index() * 64 + m.get_dest().to_index()] += depth * depth;
    }
}

impl crate::Engine {
    pub(crate) fn order_moves(&self, moves: &mut [(ChessMove, Game)], game: &Game, killer: &KillerTable) {
        // we order moves with the following order:
        // 1. good hash moves
        // 2. bad hash moves
        // 3. good MVV-LVA moves
        // 4. by killer heuristic
        // 5. bad MVV-LVA moves

        moves.sort_unstable_by(|(a_move, a), (b_move, b)| {
            self.cmp_hash(a, b)
                .then_with(|| mvv_lva(game, *a_move, *b_move))
                .then_with(|| self.killer_heuristic(killer, *a_move, *b_move))
        });
    }

    fn cmp_hash(&self, a: &Game, b: &Game) -> Ordering {
        self.trans_table.get(a.board().get_hash()).map_or(crate::Eval::MIN, |e| e.eval).cmp(
            &self.trans_table.get(b.board().get_hash()).map_or(crate::Eval::MIN, |e| e.eval)
        )
    }

    fn killer_heuristic(&self, killer: &KillerTable, a: ChessMove, b: ChessMove) -> Ordering {
        let value = |m: ChessMove| {
            killer.0[m.get_source().to_index() * 64 + m.get_dest().to_index()]
        };

        // TODO: not sure if this ordering is correct
        // current 9-3-8 rev
        value(b).cmp(&value(a))
    }
}

fn mvv_lva(game: &Game, a: ChessMove, b: ChessMove) -> Ordering {
    const P: i16 = PIECE_VALUE_MID[0];
    const N: i16 = PIECE_VALUE_MID[1];
    const B: i16 = PIECE_VALUE_MID[2];
    const R: i16 = PIECE_VALUE_MID[3];
    const Q: i16 = PIECE_VALUE_MID[4];
    const K: i16 = 20_000;
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
