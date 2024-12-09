use core::sync::atomic::Ordering;
use crate::{*, eval::*, trans_table::*};

impl Engine {
    pub fn best_move_iter_deep<F: Fn(&Self, (chess::ChessMove, Eval, usize))>(&mut self, iter: F) -> (chess::ChessMove, Eval, usize) {
        self.time_ref = Instant::now();
        self.nodes_searched.store(0, Ordering::Relaxed);
        self.reserve_time();

        let prev = self.best_move(1);
        let mut prev = (prev.0, prev.1, 1);
        iter(self, prev.clone());

        for depth in 2..=255 {
            let this = self.best_move(depth);
            if self.times_up() { break; }

            prev = (this.0, this.1, depth);
            iter(self, prev.clone());

            if this.1 == EVAL_MAX { break; }
        }

        prev
    }

    fn best_move(&self, depth: usize) -> (chess::ChessMove, Eval) {
        let mut alpha = EVAL_MIN;
        let beta = EVAL_MAX;
        let mut best = (chess::ChessMove::default(), EVAL_MIN - 1);

        let mut moves: Vec<_> = chess::MoveGen::new_legal(self.game.board())
            .map(|m| (m, self.game.make_move(m)))
            .collect();
        moves.sort_unstable_by_key(|(_, game)| {
            self.trans_table.get(game.board().get_hash()).map_or_else(
                || evaluate_static(game.board()),
                |t| t.eval,
            )
        });
        self.nodes_searched.fetch_add(moves.len(), Ordering::Relaxed);

        let in_check = self.game.board().checkers().0 != 0;

        for (i, (m, game)) in moves.into_iter().enumerate() {
            let will_check = game.board().checkers().0 != 0;
            let ext = will_check as usize;
            let mut this_depth = if depth < 3 || in_check || i < 5 || will_check { depth - 1 } else { depth / 2 } + ext;

            let (mut neg_eval, mut nt) = self.evaluate_search(&game, this_depth, -beta, -alpha, false);

            if depth != 1 && self.times_up() { break; }

            if this_depth < depth - 1 + ext && best.1 < -neg_eval {
                let new = self.evaluate_search(&game, depth - 1 + ext, -beta, -alpha, false);

                if !self.times_up() {
                    this_depth = depth - 1 + ext;
                    (neg_eval, nt) = new;
                }
            }

            if !self.times_up() && nt != NodeType::None {
                self.trans_table.insert(game.board().get_hash(), TransTableEntry {
                    depth: this_depth as u8,
                    eval: neg_eval,
                    node_type: nt,
                });
            }

            let eval = -neg_eval;

            if eval > best.1 {
                best = (m, eval);
                alpha = alpha.max(eval);
            }
        }

        best
    }

    #[inline]
    pub fn zw_search(
        &self,
        game: &Game,
        depth: usize,
        beta: Eval,
    ) -> (Eval, NodeType) {
        self.evaluate_search(game, depth, beta - 1, beta, true)
    }

    /// Perform an alpha-beta (fail-soft) negamax search and return the evaluation
    pub fn evaluate_search(
        &self,
        game: &Game,
        depth: usize,
        mut alpha: Eval,
        beta: Eval,
        in_zw: bool,
    ) -> (Eval, NodeType) {
        if let Some(trans) = self.trans_table.get(game.board().get_hash()) {
            if trans.depth as usize >= depth && (trans.node_type == NodeType::Exact
                || (trans.node_type == NodeType::LowerBound && trans.eval >= beta)
                || (trans.node_type == NodeType::UpperBound && trans.eval < alpha)) {
                return (trans.eval, NodeType::None);
            }
        }

        match game.board().status() {
            chess::BoardStatus::Ongoing => {},
            chess::BoardStatus::Checkmate => return (EVAL_MIN, NodeType::None),
            chess::BoardStatus::Stalemate => return (0, NodeType::None),
        }

        if game.can_declare_draw() {
            return (0, NodeType::None);
        }

        if self.times_up() {
            return (0, NodeType::None);
        }

        if depth == 0 {
            return (self.quiescence_search(game, alpha, beta), NodeType::Exact);
        }

        let in_check = game.board().checkers().0 != 0;
        if !in_check && depth > 3 && !in_zw {
            let game = game.make_null_move().unwrap();
            let (neg_eval, nt) = self.zw_search(&game, depth - if depth > 7 && game.board().color_combined(game.board().side_to_move()).popcnt() >= 2 { 5 } else { 4 }, 1 - beta);

            if -neg_eval >= beta {
                return (-neg_eval, NodeType::None);
            }
        }

        let mut moves: Vec<_> = chess::MoveGen::new_legal(game.board())
            .map(|m| game.make_move(m))
            .collect();
        moves.sort_unstable_by_key(|game| {
            self.trans_table.get(game.board().get_hash()).map_or_else(
                || evaluate_static(game.board()),
                |t| t.eval,
            )
        });
        self.nodes_searched.fetch_add(moves.len(), Ordering::Relaxed);

        let mut best = EVAL_MIN;
        for (i, game) in moves.into_iter().enumerate() {
            let will_check = game.board().checkers().0 != 0;
            let ext = will_check as usize;
            let mut this_depth = if depth < 3 || in_check || i < 5 || will_check { depth - 1 } else { depth / 2 } + ext;

            // futility pruning: kill nodes with no potential
            if !in_check && this_depth <= 1 {
                if -evaluate_static(game.board()) + (300 * this_depth as Eval + 100) < alpha {
                    continue;
                }
            }

            let (mut neg_eval, mut nt) = self.evaluate_search(&game, this_depth, -beta, -alpha, in_zw);
            if self.times_up() { return (best, NodeType::None); }

            if this_depth < depth - 1 + ext && best < -neg_eval {
                let new = self.evaluate_search(&game, depth - 1 + ext, -beta, -alpha, in_zw);

                if !self.times_up() {
                    this_depth = depth - 1 + ext;
                    (neg_eval, nt) = new;
                }
            }

            if nt != NodeType::None {
                self.trans_table.insert(game.board().get_hash(), TransTableEntry {
                    depth: this_depth as u8,
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
