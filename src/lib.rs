pub use eval::{Eval, evaluate_static};
pub use game::Game;
pub use see::see;

use std::time::*;
use std::sync::{atomic::*, RwLock};

mod eval;
pub mod game;
mod move_order;
mod search;
mod see;
mod shared_table;
mod trans_table;

pub struct Engine {
    pub game: RwLock<Game>,
    trans_table: trans_table::TransTable,
    hist_table: move_order::ButterflyTable,

    time_ref: RwLock<Instant>,
    time_usable: RwLock<Duration>,
    can_time_out: AtomicBool,

    nodes_searched: AtomicUsize,

    smp_start: AtomicUsize,
    smp_abort: AtomicUsize,
    smp_exit:  AtomicBool,
    smp_alive: AtomicUsize,
    smp_count: usize,
}

pub(crate) struct SmpThread<'a> {
    // TODO: is this a bottleneck?
    game: &'a RwLock<Game>,
    trans_table: &'a trans_table::TransTable,
    hist_table: &'a move_order::ButterflyTable,

    // TODO: is this a bottleneck?
    nodes_searched: &'a AtomicUsize,

    index: usize,

    /// Non-zero value signals start of depth of specified value
    start: &'a AtomicUsize,
    abort: &'a AtomicUsize,
    exit:  &'a AtomicBool,
    /// Decrement when thread killed
    alive: &'a AtomicUsize,

    thread_abort: usize,

    rng: fastrand::Rng,
}

impl Engine {
    pub fn new(game: Game, hash_size_bytes: usize) -> Self {
        Self {
            game: RwLock::new(game),
            trans_table: trans_table::TransTable::new(hash_size_bytes / trans_table::TransTable::entry_size()),
            hist_table: move_order::ButterflyTable::new(),

            time_ref: Instant::now().into(),
            time_usable: Duration::default().into(),
            can_time_out: AtomicBool::new(true),

            nodes_searched: AtomicUsize::new(0),

            smp_start: AtomicUsize::new(0),
            smp_abort: AtomicUsize::new(0),
            smp_exit:  AtomicBool::new(false),
            smp_alive: AtomicUsize::new(0),
            smp_count: 0,
        }
    }

    pub fn start_smp(&mut self, smp_count: usize) {
        assert_eq!(self.smp_count, 0);

        self.smp_count = smp_count;
        self.smp_alive.store(smp_count, Ordering::Relaxed);

        for index in 0..smp_count {
            // SAFETY: `Engine` checks that no threads are alive when exiting
            let s = unsafe { core::mem::transmute::<_, &'static Self>(&*self) };

            std::thread::spawn(move || {
                SmpThread {
                    game: &s.game,
                    trans_table: &s.trans_table,
                    hist_table: &s.hist_table,

                    nodes_searched: &s.nodes_searched,

                    index,

                    start: &s.smp_start,
                    abort: &s.smp_abort,
                    exit: &s.smp_exit,
                    alive: &s.smp_alive,

                    thread_abort: 1,

                    rng: fastrand::Rng::with_seed(index as _)
                }.start();
            });
        }
    }

    pub fn time_control(&self, time_ctrl: TimeControl) {
        // https://github.com/SebLague/Chess-Coding-Adventure/blob/Chess-V2-UCI/Chess-Coding-Adventure/src/Bot.cs#L64

        let left = time_ctrl.time_left as u64;
        let incr = time_ctrl.time_incr as u64;

        let mut think_time = left / 40;

        if left > incr << 2 {
            think_time += incr * 4 / 5;
        }

        let min_think = (left / 4).min(50);
        *self.time_usable.write().unwrap() = Duration::from_millis(min_think.max(think_time));
    }

    pub fn allow_for(&self, time: Duration) {
        *self.time_usable.write().unwrap() = time;
    }

    pub fn times_up(&self) -> bool {
        self.can_time_out.load(Ordering::Relaxed) && self.elapsed() > *self.time_usable.read().unwrap()
    }

    pub fn find_pv(&self, best: chess::ChessMove, max: usize) -> Vec<chess::ChessMove> {
        use chess::*;

        let mut pv = Vec::with_capacity(max);
        pv.push(best);

        let mut game = self.game.read().unwrap().make_move(best);
        while let Some(tte) = self.trans_table.get(game.board().get_hash()) {
            if tte.next == ChessMove::default() { break }

            pv.push(tte.next);
            game = game.make_move(tte.next);

            if pv.len() >= max { break }
        }

        pv
    }

    pub fn nodes(&self) -> usize {
        self.nodes_searched.load(Ordering::Relaxed)
    }

    pub fn elapsed(&self) -> Duration {
        self.time_ref.read().unwrap().elapsed()
    }

    pub fn tt_size(&self) -> usize { self.trans_table.size() }

    pub fn tt_used(&self) -> usize {
        self.trans_table.filter_count(|e| e.node_type != trans_table::NodeType::None)
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
        self.smp_exit.store(true, Ordering::Relaxed);
        self.smp_abort.store(usize::MAX, Ordering::Relaxed);
        while self.smp_alive.load(Ordering::Relaxed) != 0 {}
    }
}

impl Drop for SmpThread<'_> {
    fn drop(&mut self) {
        self.alive.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Default)]
pub struct TimeControl {
    pub time_left: usize,
    pub time_incr: usize,
}

pub(crate) fn hash<T: core::hash::Hash + ?Sized>(v: &T) -> u64 {
    let mut state = rustc_hash::FxHasher::default();
    v.hash(&mut state);
    core::hash::Hasher::finish(&state)
}
