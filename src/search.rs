use crate::{*, eval::*, trans_table::*};

impl Engine {
    pub fn best_move_iter_deep(&mut self) -> (chess::ChessMove, Eval, usize) {
        self.time_ref = Instant::now();
        self.reserve_time();

        let prev = self.best_move(1);
        let mut prev = (prev.0, prev.1, 1);

        for depth in 2.. {
            let this = self.best_move(depth);
            if self.times_up() { break; }

            prev = (this.0, this.1, depth);
        }

        prev
    }

    fn best_move(&self, depth: usize) -> (chess::ChessMove, Eval) {
        let mut alpha = EVAL_MIN;
        let mut best = (chess::ChessMove::default(), EVAL_MIN - 1);

        let mut moves: Vec<_> = chess::MoveGen::new_legal(self.game.board())
            .map(|m| (m, self.game.make_move(m)))
            .collect();
        moves.sort_unstable_by_key(|(_, game)| {
            self.trans_table.get(game.board().get_hash()).map_or(EVAL_MIN, |t| t.eval)
        });

        for (m, game) in moves {
            let (neg_eval, _nt) = self.evaluate_search(&game, depth - 1, EVAL_MIN, -alpha);
            let eval = -neg_eval;
            if depth != 1 && self.times_up() { break; }

            if eval > best.1 {
                best = (m, eval);
                alpha = alpha.max(eval);
            }
        }

        best
    }

    /// Perform an alpha-beta (fail-soft) negamax search and return the evaluation
    pub fn evaluate_search(
        &self,
        game: &Game,
        depth: usize,
        mut alpha: Eval,
        beta: Eval,
    ) -> (Eval, NodeType) {
        if let Some(trans) = self.trans_table.get(game.board().get_hash()) {
            if trans.depth as usize >= depth && (trans.node_type == NodeType::Exact
                || (trans.node_type == NodeType::LowerBound && trans.eval >= beta)
                || (trans.node_type == NodeType::UpperBound && trans.eval < alpha)) {
                return (trans.eval, NodeType::None);
            }
        }

        if self.times_up() {
            return (0, NodeType::None);
        }

        if game.can_declare_draw() {
            return (0, NodeType::Exact);
        }

        if depth == 0 {
            return (self.quiescence_search(game, alpha, beta), NodeType::Exact);
        }

        let mut best = EVAL_MIN;

        let mut moves: Vec<_> = chess::MoveGen::new_legal(game.board())
            .map(|m| game.make_move(m))
            .collect();
        moves.sort_unstable_by_key(|game| {
            self.trans_table.get(game.board().get_hash()).map_or(EVAL_MIN, |t| t.eval)
        });

        for game in moves {
            let (neg_eval, nt) = self.evaluate_search(&game, depth - 1, -beta, -alpha);
            if self.times_up() { return (best, NodeType::None); }

            if nt != NodeType::None {
                self.trans_table.insert(game.board().get_hash(), TransTableEntry {
                    depth: (depth - 1) as u8,
                    eval: neg_eval,
                    node_type: nt,
                });
            }

            let eval = -neg_eval;

            if eval > best {
                best = eval;
                alpha = alpha.max(eval);
            }
            if eval >= beta {
                return (best, NodeType::LowerBound);
            }
        }

        (best, if best == alpha { NodeType::UpperBound } else { NodeType::Exact })
    }

    fn quiescence_search(&self, game: &Game, mut alpha: Eval, beta: Eval) -> Eval {
        let standing_pat = evaluate_static(game.board());
        if standing_pat >= beta { return beta; }
        alpha = alpha.max(standing_pat);
        let mut best = standing_pat;

        let mut moves = chess::MoveGen::new_legal(game.board());
        moves.set_iterator_mask(*game.board().combined());
        for m in moves {
            let game = game.make_move(m);
            let eval = -self.quiescence_search(&game, -beta, -alpha);

            if eval > best {
                best = eval;
                alpha = alpha.max(eval);
            }
            if eval >= beta {
                return best;
            }
        }

        best
    }
}
