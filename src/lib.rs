pub use eval::Eval;
pub use game::Game;
pub use see::see;

use std::time::*;
use std::sync::atomic::*;

use sync::*;

use parking_lot::{Condvar, Mutex, RwLock};

mod debug;
mod eval;
pub mod game;
mod move_order;
mod node;
mod search;
mod see;
mod selfplay;
mod shared_table;
mod sync;
mod trans_table;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Engine {
    pub game: RwLock<Game>,
    trans_table: trans_table::TransTable,

    time_ref: Instant,
    soft_time_bound: Duration,
    hard_time_bound: Duration,
    can_time_out: AtomicBool,

    debug: debug::DebugStats,
    pub eval_params: eval::EvalParams,

    smp_count: usize,
    smp_prev: Mutex<Eval>,
    smp_start: Condvar,
    smp_abort: CondBarrier,
    smp_exit: CondBarrier,
}

pub(crate) struct SmpThread<'a, const MAIN: bool = false> {
    engine: &'a Engine,
    index: usize,

    hist_table: move_order::HistoryTable,
    countermove: move_order::CountermoveTable,

    nodes_searched: usize,
}

impl Engine {
    pub fn new(game: Game, hash_size_bytes: usize) -> Self {
        Self {
            game: RwLock::new(game),
            trans_table: trans_table::TransTable::new(hash_size_bytes / trans_table::TransTable::entry_size()),

            time_ref: Instant::now(),
            soft_time_bound: Duration::default(),
            hard_time_bound: Duration::default(),
            can_time_out: AtomicBool::new(true),

            debug: debug::DebugStats::default(),
            eval_params: eval::EvalParams::default(),

            smp_count: 0,
            smp_prev: Mutex::new(Eval(0)),
            smp_start: Condvar::new(),
            smp_abort: CondBarrier::new(1),
            smp_exit: CondBarrier::new(1),
        }
    }

    pub fn kill_smp(&mut self) {
        self.smp_abort.initiate_wait();
        self.smp_exit.initiate();

        let mut sum = 0;

        *self.smp_prev.lock() = Eval(0);
        while sum < self.smp_count {
            sum += self.smp_start.notify_all();
        }
        self.smp_abort.initiate();

        self.smp_exit.always_wait();
        self.smp_exit.uninitiate();
        self.smp_abort.uninitiate();
    }

    pub(crate) fn new_thread<'a, const MAIN: bool>(&'a self, index: usize) -> SmpThread<'a, MAIN> {
        SmpThread {
            engine: self,
            index,

            hist_table: move_order::ButterflyTable::new(),
            countermove: move_order::CountermoveTable::new(),

            nodes_searched: 0,
        }
    }

    pub fn start_smp(&mut self, smp_count: usize) {
        self.smp_abort = CondBarrier::new(smp_count + 1);
        self.smp_exit = CondBarrier::new(smp_count + 1);
        self.smp_count = smp_count;

        for index in 1..=smp_count {
            // SAFETY: `Engine` checks that no threads are alive when exiting
            let s = unsafe { core::mem::transmute::<_, &'static Self>(&*self) };

            std::thread::spawn(move || {
                s.new_thread::<false>(index).start();
            });
        }
    }

    pub fn time_control(&mut self, moves_to_go: Option<usize>, time_ctrl: TimeControl) {
        let left = Duration::from_millis(time_ctrl.time_left as _);
        let incr = Duration::from_millis(time_ctrl.time_incr as _);

        let mtg = moves_to_go.unwrap_or(40) as u32;

        self.soft_time_bound = left / mtg + if left > incr * 4 { incr * 3 / 5 } else { Duration::ZERO };
        self.hard_time_bound = self.soft_time_bound * 3 / 2;
    }

    pub fn allow_for(&mut self, time: Duration) {
        self.soft_time_bound = time;
        self.hard_time_bound = time;
    }

    pub fn soft_times_up(&self) -> bool {
        self.can_time_out.load(Ordering::Relaxed) && self.elapsed() > self.soft_time_bound
    }

    pub fn hard_times_up(&self) -> bool {
        self.can_time_out.load(Ordering::Relaxed) && self.elapsed() > self.hard_time_bound
    }

    pub fn find_pv(&self, best: chess::ChessMove, max: usize) -> Vec<chess::ChessMove> {
        use chess::*;

        let mut pv = Vec::with_capacity(max);
        pv.push(best);
        if best == ChessMove::default() { return pv };

        let mut game = self.game.read().make_move(best);
        while let Some(tte) = self.trans_table.get(game.board().get_hash()) {
            if tte.next == ChessMove::default() { break };

            pv.push(tte.next);
            game = game.make_move(tte.next);

            if pv.len() >= max { break };
        }

        pv
    }

    pub fn nodes(&self) -> usize {
        self.debug.nodes.get()
    }

    pub fn elapsed(&self) -> Duration {
        self.time_ref.elapsed()
    }

    pub fn tt_size(&self) -> usize { self.trans_table.size() }

    pub fn tt_used(&self) -> usize {
        self.trans_table.filter_count(|e| e.node_type() != node::NodeType::None)
    }

    pub fn resize_hash(&mut self, hash_size_bytes: usize) {
        self.trans_table = trans_table::TransTable::new(hash_size_bytes / trans_table::TransTable::entry_size());
    }

    pub fn clear_hash(&mut self) {
        self.trans_table.clear();
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.kill_smp();
    }
}

impl<const MAIN: bool> Drop for SmpThread<'_, MAIN> {
    fn drop(&mut self) {
        if !MAIN {
            self.smp_exit.always_wait();
        }
    }
}

impl<const MAIN: bool> core::ops::Deref for SmpThread<'_, MAIN> {
    type Target = Engine;

    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}

#[derive(Debug, Default)]
pub struct TimeControl {
    pub time_left: usize,
    pub time_incr: usize,
}
