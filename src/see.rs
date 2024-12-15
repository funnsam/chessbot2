use crate::Game;
use crate::eval::PIECE_VALUE;
use chess::{BitBoard, ChessMove, Color, ALL_PIECES};

pub fn see(game: &Game, m: ChessMove) -> i16 {
    fn smallest_attacker(attadef: BitBoard, stm: Color, game: &Game) -> BitBoard {
        for pt in ALL_PIECES {
            let subset = attadef & game.board().pieces(pt) & game.board().color_combined(!stm);

            if subset.0 != 0 {
                return subset & BitBoard::new(!subset.0 + 1);
            }
        }

        BitBoard::new(0)
    }

    if let Some(target) = game.board().piece_on(m.get_dest()) {
        let attacker = unsafe { game.board().piece_on(m.get_source()).unwrap_unchecked() };

        let mut from = BitBoard::from_square(m.get_source());
        let mut combined = *game.board().combined();
        let mut stm = game.board().side_to_move();
        let mut attadef = game.board().pseudo_attacks_to(m.get_dest(), combined);
        let mut gain = [0; 32];
        gain[0] = PIECE_VALUE[target.to_index()];

        let mut max_d = 0;
        for d in 1..32 {
            gain[d] = PIECE_VALUE[attacker.to_index()] - gain[d - 1];

            attadef ^= from;
            combined ^= from;
            stm = !stm;
            from = smallest_attacker(attadef, stm, game);

            max_d = d;

            if from.0 == 0 { break };
        }

        for inv_d in 1..max_d {
            let d = max_d - inv_d;
            gain[d - 1] = -((-gain[d - 1]).max(gain[d]));
        }

        return gain[0];
    }

    0
}
