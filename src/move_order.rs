use crate::Game;
use chess::ChessMove;

impl crate::Engine {
    pub fn order_moves(&self, moves: &mut [(ChessMove, Game)]) {
        moves.sort_unstable_by_key(|(_, game)| {
            self.trans_table.get(game.board().get_hash()).map_or_else(
                || crate::eval::evaluate_static(game.board()),
                |t| t.eval,
            )
        });
    }
}
