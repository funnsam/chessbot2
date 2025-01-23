pub use eval::{Eval, evaluate_static};
pub use game::Game;
pub use see::see;

use std::time::*;
use std::sync::atomic::*;

use sync::*;
use tables::*;

use parking_lot::{Condvar, Mutex, RwLock};

mod debug;
mod eval;
pub mod game;
mod move_order;
mod node;
mod search;
mod see;
mod shared_table;
mod sync;
mod tables;
mod trans_table;

pub struct Engine {
    pub game: RwLock<Game>,
    trans_table: trans_table::TransTable,

    time_ref: Instant,
    time_usable: Duration,
    can_time_out: AtomicBool,

    debug: debug::DebugStats,

    smp_prev: Mutex<Eval>,
    smp_start: Condvar,
    smp_abort: CondBarrier,
    smp_exit: CondBarrier,
    total_nodes_searched: AtomicUsize,

    smp_count: usize,
}

pub(crate) struct SmpThread<'a, const MAIN: bool = false> {
    engine: &'a Engine,
    index: usize,

    hist_table: HistoryTable,
    countermove: CountermoveTable,
    cm_history: Box<CmHistoryTable>,

    nodes_searched: usize,
}

impl Engine {
    pub fn new(game: Game, hash_size_bytes: usize) -> Self {
        Self {
            game: RwLock::new(game),
            trans_table: trans_table::TransTable::new(hash_size_bytes / trans_table::TransTable::entry_size()),

            time_ref: Instant::now(),
            time_usable: Duration::default(),
            can_time_out: AtomicBool::new(true),

            debug: debug::DebugStats::default(),

            smp_prev: Mutex::new(Eval(0)),
            smp_start: Condvar::new(),
            smp_abort: CondBarrier::new(1),
            smp_exit: CondBarrier::new(1),
            total_nodes_searched: AtomicUsize::new(0),

            smp_count: 0,
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

            hist_table: ButterflyTable::new(),
            countermove: CountermoveTable::new(),
            cm_history: CmHistoryTable::new().into(),

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
        let left = time_ctrl.time_left as u64;
        let incr = time_ctrl.time_incr as u64;

        self.time_usable = Duration::from_millis(if let Some(mtg) = moves_to_go {
            left / mtg as u64 + incr
        } else {
            let mut think_time = left / 40;

            if left > incr << 2 {
                think_time += incr * 4 / 5;
            }

            let min_think = (left / 4).min(50);
            min_think.max(think_time)
        });
    }

    pub fn allow_for(&mut self, time: Duration) {
        self.time_usable = time;
    }

    pub fn times_up(&self) -> bool {
        self.can_time_out.load(Ordering::Relaxed) && self.elapsed() > self.time_usable
    }

    pub fn find_pv(&self, best: chess::ChessMove, max: usize) -> Vec<chess::ChessMove> {
        use chess::*;

        let mut pv = Vec::with_capacity(max);
        pv.push(best);

        let mut game = self.game.read().make_move(best);
        while let Some(tte) = self.trans_table.get(game.board().get_hash()) {
            if tte.next == ChessMove::default() { break }

            pv.push(tte.next);
            game = game.make_move(tte.next);

            if pv.len() >= max { break }
        }

        pv
    }

    pub fn nodes(&self) -> usize {
        self.total_nodes_searched.load(Ordering::Relaxed)
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
