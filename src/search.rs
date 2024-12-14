use core::sync::atomic::Ordering;
use crate::{*, eval::*, trans_table::*};
use chess::{BoardStatus, ChessMove, MoveGen};

impl Engine {
    pub fn best_move<F: Fn(&Self, (ChessMove, Eval, usize)) -> bool>(&self, cont: F) -> (ChessMove, Eval, usize) {
        *self.time_ref.write().unwrap() = Instant::now();
        self.nodes_searched.store(0, Ordering::Relaxed);

        let can_time_out = self.can_time_out.swap(false, Ordering::Relaxed);
        let prev = self._evaluate_search(&self.game, 1, 0, Eval::MIN, Eval::MAX, false);
        let mut prev = (prev.0, prev.1, 1);
        self.can_time_out.store(can_time_out, Ordering::Relaxed);
        if !cont(self, prev.clone()) || prev.1.is_positive_mate() { return prev };

        for depth in 2..=255 {
            let this = self._evaluate_search(&self.game, depth, 0, Eval::MIN, Eval::MAX, false);
            if self.times_up() { break };

            prev = (this.0, this.1, depth);
            if !cont(self, prev.clone()) || prev.1.is_positive_mate() { break };
        }

        prev
    }

    #[inline]
    fn zw_search(
        &self,
        game: &Game,
        depth: usize,
        ply: usize,
        beta: Eval,
    ) -> (Eval, NodeType) {
        self.evaluate_search(game, depth, ply, Eval(beta.0 - 1), beta, true)
    }

    /// Perform an alpha-beta (fail-soft) negamax search and return the evaluation
    #[inline]
    fn evaluate_search(
        &self,
        game: &Game,
        depth: usize,
        ply: usize,
        alpha: Eval,
        beta: Eval,
        in_zw: bool,
    ) -> (Eval, NodeType) {
        let (next, eval, nt) = self._evaluate_search(game, depth, ply, alpha, beta, in_zw);

        if nt != NodeType::None && !eval.is_mate() && !self.times_up() {
            self.trans_table.insert(game.board().get_hash(), TransTableEntry {
                depth: depth as u8,
                eval,
                node_type: nt,
                next,
            });
        }

        (eval, nt)
    }

    fn _evaluate_search(
        &self,
        game: &Game,
        depth: usize,
        ply: usize,
        mut alpha: Eval,
        beta: Eval,
        in_zw: bool,
    ) -> (ChessMove, Eval, NodeType) {
        if game.can_declare_draw() {
            return (ChessMove::default(), Eval(0), NodeType::None);
        }

        if let Some(trans) = self.trans_table.get(game.board().get_hash()) {
            let eval = trans.eval;

            if trans.depth as usize >= depth && (trans.node_type == NodeType::Exact
                || (trans.node_type == NodeType::LowerBound && eval >= beta)
                || (trans.node_type == NodeType::UpperBound && eval < alpha)) {
                return (trans.next, eval.incr_mate(), NodeType::None);
            }
        }

        match game.board().status() {
            BoardStatus::Ongoing => {},
            BoardStatus::Checkmate => return (ChessMove::default(), -Eval::M0, NodeType::None),
            BoardStatus::Stalemate => return (ChessMove::default(), Eval(0), NodeType::None),
        }

        if self.times_up() {
            return (ChessMove::default(), Eval(0), NodeType::None);
        }

        if depth == 0 {
            return (ChessMove::default(), self.quiescence_search(game, alpha, beta), NodeType::Exact);
        }

        let in_check = game.board().checkers().0 != 0;
        if ply != 0 && !in_check && depth > 3 && !in_zw {
            let game = game.make_null_move().unwrap();
            let (neg_eval, _) = self.zw_search(&game, depth - if depth > 7 && game.board().color_combined(game.board().side_to_move()).popcnt() >= 2 { 5 } else { 4 }, ply + 1, Eval(1 - beta.0));

            if -neg_eval >= beta {
                return (ChessMove::default(), (-neg_eval).incr_mate(), NodeType::None);
            }
        }

        let mut moves: Vec<_> = MoveGen::new_legal(game.board())
            .map(|m| (m, game.make_move(m)))
            .collect();
        self.order_moves(&mut moves);
        self.nodes_searched.fetch_add(moves.len(), Ordering::Relaxed);

        let mut best = (ChessMove::default(), Eval::MIN);
        for (i, (m, game)) in moves.into_iter().enumerate() {
            let this_depth = if depth < 3 || in_check || i < 5 || game.board().checkers().0 != 0 { depth - 1 } else { depth / 2 };

            // futility pruning: kill nodes with no potential
            if !in_check && depth <= 2 {
                let eval = -evaluate_static(game.board());
                let margin = 100 * depth as i16 * depth as i16  ;

                if eval.0 + margin < alpha.0 {
                    if best.0 == ChessMove::default() {
                        best = (m, Eval(eval.0 - margin));
                    }

                    continue;
                }
            }

            let (mut neg_eval, mut nt) = self.evaluate_search(&game, this_depth, ply + 1, -beta, -alpha, in_zw);
            if self.times_up() { return (best.0, best.1.incr_mate(), NodeType::None); }

            if this_depth < depth - 1 && best.1 < -neg_eval {
                let new = self.evaluate_search(&game, depth - 1, ply + 1, -beta, -alpha, in_zw);

                if !self.times_up() {
                    (neg_eval, nt) = new;
                }
            }

            let eval = -neg_eval;

            if eval > best.1 || best.0 == ChessMove::default() {
                best = (m, eval);
                alpha = alpha.max(eval);
            }
            if eval >= beta {
                return (best.0, best.1.incr_mate(), NodeType::LowerBound);
            }
        }

        (best.0, best.1.incr_mate(), if best.1 == alpha { NodeType::UpperBound } else { NodeType::Exact })
    }

    fn quiescence_search(&self, game: &Game, mut alpha: Eval, beta: Eval) -> Eval {
        let standing_pat = evaluate_static(game.board());
        if standing_pat >= beta { return beta; }
        alpha = alpha.max(standing_pat);
        let mut best = standing_pat;

        let mut moves = MoveGen::new_legal(game.board());
        moves.set_iterator_mask(*game.board().combined());
        self.nodes_searched.fetch_add(moves.len(), Ordering::Relaxed);

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
