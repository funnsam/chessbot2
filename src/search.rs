use core::sync::atomic::Ordering;
use crate::{*, eval::*, trans_table::*};
use chess::{BoardStatus, ChessMove, MoveGen};
use move_order::KillerTable;

impl Engine {
    pub fn best_move<F: FnMut(&Self, (ChessMove, Eval, usize)) -> bool>(&self, mut cont: F) -> (ChessMove, Eval, usize) {
        *self.time_ref.write().unwrap() = Instant::now();
        self.nodes_searched.store(0, Ordering::Relaxed);
        self.hist_table.clear();

        let can_time_out = self.can_time_out.swap(false, Ordering::Relaxed);
        let prev = self.root_search(1, Eval::MIN, Eval::MAX);
        let mut prev = (prev.0, prev.1, 1);
        self.can_time_out.store(can_time_out, Ordering::Relaxed);
        if !cont(self, prev.clone()) { return prev };

        for depth in 2..=255 {
            let this = self.root_search(depth, Eval::MIN, Eval::MAX);
            if self.times_up() { break };

            prev = (this.0, this.1, depth);
            if !cont(self, prev.clone()) { break };
        }

        println!("{:#?}", self.debug);
        prev
    }

    #[inline]
    fn root_search(
        &self,
        depth: usize,
        alpha: Eval,
        beta: Eval,
    ) -> (ChessMove, Eval) {
        let (next, eval, nt) = self._evaluate_search(ChessMove::default(), &self.game, &KillerTable::new(), depth, 0, alpha, beta, false, true);

        self.store_tt(depth, &self.game, (next, eval, nt));

        (next, eval)
    }

    #[inline]
    fn zw_search(
        &self,
        prev_move: ChessMove,
        game: &Game,
        killer: &KillerTable,
        depth: usize,
        ply: usize,
        beta: Eval,
    ) -> Eval {
        if beta == Eval::MIN { println!("{}", beta - 1) };
        self.evaluate_search(prev_move, game, killer, depth, ply, beta - 1, beta, true, false)
    }

    /// Perform an alpha-beta (fail-soft) negamax search and return the evaluation
    #[inline]
    fn evaluate_search(
        &self,
        prev_move: ChessMove,
        game: &Game,
        killer: &KillerTable,
        depth: usize,
        ply: usize,
        alpha: Eval,
        beta: Eval,
        in_zw: bool,
        is_pv: bool,
    ) -> Eval {
        let (next, eval, nt) = self._evaluate_search(prev_move, game, killer, depth, ply, alpha, beta, in_zw, is_pv);

        self.store_tt(depth, game, (next, eval, nt));

        eval
    }

    fn store_tt(&self, depth: usize, game: &Game, (next, eval, nt): (ChessMove, Eval, NodeType)) {
        if nt != NodeType::None && !self.times_up() {
            self.trans_table.insert(game.board().get_hash(), TransTableEntry {
                depth: depth as u8,
                eval,
                node_type: nt,
                next,
            });
        }
    }

    fn _evaluate_search(
        &self,
        prev_move: ChessMove,
        game: &Game,
        p_killer: &KillerTable,
        depth: usize,
        ply: usize,
        mut alpha: Eval,
        beta: Eval,
        in_zw: bool,
        is_pv: bool,
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

        if self.times_up() {
            return (ChessMove::default(), Eval(0), NodeType::None);
        }

        if depth == 0 {
            return (ChessMove::default(), self.quiescence_search(game, alpha, beta), NodeType::None);
        }

        let killer = KillerTable::new();

        // internal iterative reductions
        // if ply > 0 && depth >= 4 && self.trans_table.get(game.board().get_hash()).is_none() {
        //     let low = self._evaluate_search(prev_move, game, &killer, depth / 4, ply, alpha, beta, false, false);
        //     self.store_tt(depth / 4, game, low);

        //     if low.1 <= alpha {
        //         return (low.0, low.1, NodeType::None);
        //     }
        // }

        let in_check = game.board().checkers().0 != 0;

        // null move pruning
        // if ply != 0 && !in_check && depth > 3 && !in_zw {
        //     let game = game.make_null_move().unwrap();
        //     let r = if depth > 7 && game.board().color_combined(game.board().side_to_move()).popcnt() >= 2 { 5 } else { 4 };
        //     let eval = -self.zw_search(prev_move, &game, &killer, depth - r, ply + 1, 1 - beta);

        //     if eval >= beta {
        //         return (ChessMove::default(), eval.incr_mate(), NodeType::None);
        //     }
        // }

        let mut moves = MoveGen::new_legal(game.board()).collect::<arrayvec::ArrayVec<_, 256>>();
        self.order_moves(prev_move, &mut moves, game, &p_killer);

        self.nodes_searched.fetch_add(moves.len(), Ordering::Relaxed);

        let mut best = (ChessMove::default(), Eval::MIN);
        let mut real_i = 0;
        let _game = &game;
        for (i, m) in moves.iter().copied().enumerate() {
            let game = _game.make_move(m);

            // futility pruning: kill nodes with no potential
            // if !in_check && depth <= 2 {
            //     let eval = -evaluate_static(game.board());
            //     let margin = 100 * depth as i16 * depth as i16;

            //     if eval.0 + margin < alpha.0 {
            //         if best.0 == ChessMove::default() {
            //             best = (m, eval - margin);
            //         }

            //         continue;
            //     }
            // }

            let can_reduce = depth >= 3 && !in_check && real_i != 0;

            let mut eval = Eval(i16::MIN);
            let do_full_research = if can_reduce {
                eval = -self.zw_search(m, &game, &killer, depth / 2, ply + 1, -alpha);
                if alpha < eval && depth / 2 < depth - 1 {
                self.debug.researched.inc();
                }else{
                self.debug.no_research.inc();
                }

                alpha < eval && depth / 2 < depth - 1
            } else {
                !is_pv || real_i != 0
            };

            if do_full_research {
                eval = -self.zw_search(m, &game, &killer, depth - 1, ply + 1, -alpha);
            }

            if is_pv && (real_i == 0 || alpha < eval) {
                eval = -self.evaluate_search(m, &game, &killer, depth - 1, ply + 1, -beta, -alpha, in_zw, true);
                self.debug.full.inc();
            }

            if self.times_up() { return (best.0, best.1.incr_mate(), NodeType::None) };

            if ply == 0 {
                println!(" {m} {eval} {can_reduce} {do_full_research} {:?}", self.find_pv(m, 100).into_iter().map(|i| i.to_string()).collect::<Vec<_>>());
            }

            if eval > best.1 || best.0 == ChessMove::default() {
                best = (m, eval);
                alpha = alpha.max(eval);
            }
            if eval >= beta {
                if _game.board().piece_on(m.get_dest()).is_none() {
                    let bonus = 300 * depth as isize - 250;

                    for m in moves[..i].into_iter() {
                        if _game.board().piece_on(m.get_dest()).is_none() {
                            self.hist_table.update(*m, -bonus);
                            p_killer.update(*m, -bonus);
                        }
                    }

                    self.hist_table.update(m, bonus);
                    p_killer.update(m, bonus);
                    *self.countermove.get_mut(prev_move) = m;
                }

                return (best.0, best.1.incr_mate(), NodeType::LowerBound);
            }

            real_i += 1;
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
