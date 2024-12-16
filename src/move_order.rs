use core::cmp::*;
use crate::{see, Game};
use crate::eval::PIECE_VALUE;
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
    pub(crate) fn order_moves(&self, moves: &mut [ChessMove], game: &Game, killer: &KillerTable) {
        // we order moves with the following order:
        // 1. good hash moves
        // 2. bad hash moves
        // 3. good MVV-LVA moves
        // 4. by killer heuristic
        // 5. bad MVV-LVA moves

        moves.sort_unstable_by(|a, b| {
            self.cmp_hash(game, *a, *b)
                .then_with(|| mvv_lva(game, *a, *b))
                .then_with(|| see_order(game, *a, *b))
                .then_with(|| self.killer_heuristic(killer, *a, *b))
        });
    }

    fn cmp_hash(&self, game: &Game, a: ChessMove, b: ChessMove) -> Ordering {
        self.trans_table.get(game.board().get_hash())
            .map_or(Ordering::Equal, |e| (b == e.next).cmp(&(a == e.next)))
    }

    fn killer_heuristic(&self, killer: &KillerTable, a: ChessMove, b: ChessMove) -> Ordering {
        let value = |m: ChessMove| {
            killer.0[m.get_source().to_index() * 64 + m.get_dest().to_index()]
        };

        value(b).cmp(&value(a))
    }
}

fn see_order(game: &Game, a: ChessMove, b: ChessMove) -> Ordering {
    see(game, a).cmp(&see(game, b))
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
