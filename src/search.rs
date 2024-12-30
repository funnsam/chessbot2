use core::sync::atomic::Ordering;
use crate::{*, eval::*, trans_table::*};
use chess::{BoardStatus, ChessMove, MoveGen};
use move_order::ButterflyTable;

impl Engine {
    pub fn best_move<F: FnMut(&Self, (ChessMove, Eval, usize)) -> bool>(&self, mut cont: F) -> (ChessMove, Eval, usize) {
        *self.time_ref.write().unwrap() = Instant::now();
        self.nodes_searched.store(0, Ordering::Relaxed);
        self.hist_table.clear();

        let mut main_thread = SmpThread {
            game: &self.game,
            trans_table: &self.trans_table,
            hist_table: &self.hist_table,

            nodes_searched: &self.nodes_searched,

            index: 0,

            start: &AtomicUsize::new(1),
            abort: &AtomicUsize::new(0),
            exit:  &AtomicBool::new(false),
            alive: &AtomicUsize::new(1),

            thread_abort: 0,

            rng: fastrand::Rng::with_seed(0xdeadbeef),
        };
        let prev = main_thread._evaluate_search(&self.game.read().unwrap().clone(), &ButterflyTable::new(), 1, 0, Eval::MIN, Eval::MAX, false);
        let mut prev = (prev.0, prev.1, 1);
        if !cont(self, prev) { return prev };

        for depth in 2..=255 {
            self.smp_start.store(depth, Ordering::Relaxed);
            self.smp_abort.fetch_add(1, Ordering::Relaxed);
            let this = main_thread._evaluate_search(&self.game.read().unwrap().clone(), &mut ButterflyTable::new(), depth, 0, Eval::MIN, Eval::MAX, false);
            prev = (this.0, this.1, depth);
            if !cont(self, prev) { break };
        }

        prev
    }
}

impl SmpThread<'_> {
    pub fn start(mut self) {
        while !self.exit.load(Ordering::Relaxed) {
            let start = self.start.load(Ordering::Relaxed);

            if start != 0 && !self.abort() {
                let depth = start + self.index / 5;

                // println!("thread {} depth {depth}", self.index);
                self.evaluate_search(&self.game.read().unwrap().clone(), &mut ButterflyTable::new(), depth, 0, Eval::MIN, Eval::MAX, false);
                // println!("thread {} depth {depth} end", self.index);

                self.thread_abort += 1;
            }
        }
    }

    fn abort(&self) -> bool {
        self.abort.load(Ordering::Relaxed) != self.thread_abort
    }

    #[inline]
    fn zw_search(
        &mut self,
        game: &Game,
        killer: &ButterflyTable,
        depth: usize,
        ply: usize,
        beta: Eval,
    ) -> Eval {
        self.evaluate_search(game, killer, depth, ply, Eval(beta.0 - 1), beta, true)
    }

    /// Perform an alpha-beta (fail-soft) negamax search and return the evaluation
    #[inline]
    fn evaluate_search(
        &mut self,
        game: &Game,
        killer: &ButterflyTable,
        depth: usize,
        ply: usize,
        alpha: Eval,
        beta: Eval,
        in_zw: bool,
    ) -> Eval {
        let (next, eval, nt) = self._evaluate_search(game, killer, depth, ply, alpha, beta, in_zw);

        if nt != NodeType::None && !self.abort() {
            self.trans_table.insert(game.board().get_hash(), TransTableEntry {
                depth: depth as u8,
                eval,
                node_type: nt,
                next,
            });
        }

        eval
    }

    fn _evaluate_search(
        &mut self,
        game: &Game,
        p_killer: &ButterflyTable,
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
                return (trans.next, eval, NodeType::None);
            }
        }

        match game.board().status() {
            BoardStatus::Ongoing => {},
            BoardStatus::Checkmate => return (ChessMove::default(), -Eval::M0, NodeType::None),
            BoardStatus::Stalemate => return (ChessMove::default(), Eval(0), NodeType::None),
        }

        if self.abort() {
            return (ChessMove::default(), Eval(0), NodeType::None);
        }

        if depth == 0 {
            return (ChessMove::default(), self.quiescence_search(game, alpha, beta), NodeType::Exact);
        }

        let killer = ButterflyTable::new();
        let in_check = game.board().checkers().0 != 0;

        if ply != 0 && !in_check && depth > 3 && !in_zw {
            let game = game.make_null_move().unwrap();
            let r = if depth > 7 && game.board().color_combined(game.board().side_to_move()).popcnt() >= 2 { 5 } else { 4 };
            let eval = -self.zw_search(&game, &killer, depth - r, ply + 1, Eval(1 - beta.0));

            if eval >= beta {
                return (ChessMove::default(), eval.incr_mate(), NodeType::None);
            }
        }

        let mut moves = MoveGen::new_legal(game.board()).collect::<arrayvec::ArrayVec<_, 256>>();
        self.order_moves(&mut moves, game, &p_killer);
        self.nodes_searched.fetch_add(moves.len(), Ordering::Relaxed);

        let mut best = (ChessMove::default(), Eval::MIN);
        let _game = &game;
        for (i, m) in moves.into_iter().enumerate() {
            let game = _game.make_move(m);

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

            let mut eval = -self.evaluate_search(&game, &killer, this_depth, ply + 1, -beta, -alpha, in_zw);
            if self.abort() { return (best.0, best.1.incr_mate(), NodeType::None); }

            if this_depth < depth - 1 && best.1 < eval {
                let new = -self.evaluate_search(&game, &killer, depth - 1, ply + 1, -beta, -alpha, in_zw);

                if !self.abort() {
                    eval = -new
                }
            }

            if eval > best.1 || best.0 == ChessMove::default() {
                best = (m, eval);
                alpha = alpha.max(eval);
            }
            if eval >= beta {
                if _game.board().piece_on(m.get_dest()).is_none() {
                    p_killer.update(m, depth);
                    self.hist_table.update(m, depth);
                }

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
            if see(game, m) < 0 { continue };

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
