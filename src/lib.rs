pub use eval::Eval;
pub use game::Game;
pub(crate) use see::see;
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
    pub game: Game,
    trans_table: trans_table::TransTable,

    time_ref: RwLock<Instant>,
    time_usable: RwLock<Duration>,
    can_time_out: AtomicBool,

    nodes_searched: core::sync::atomic::AtomicUsize,
    // search_done: AtomicBool,
}

impl Engine {
    pub fn new(game: Game, hash_size_bytes: usize) -> Self {
        Self {
            game,
            trans_table: trans_table::TransTable::new(hash_size_bytes / trans_table::TransTable::entry_size()),

            time_ref: Instant::now().into(),
            time_usable: Duration::default().into(),
            can_time_out: AtomicBool::new(true),

            nodes_searched: AtomicUsize::new(0),
            // search_done: AtomicBool::new(false),
        }
    }

    // pub fn ponder(self: Arc<Self>) {
    //     self.allow_for(Duration::ZERO);
    //     self.can_time_out.store(false, Ordering::Relaxed);

    //     {
    //         let engine = Arc::clone(&self);
    //         std::thread::spawn(move || {
    //             engine.best_move(|_, _| true);
    //         });
    //     }
    // }

    // pub fn stop_ponder(&self) {
    //     self.can_time_out.store(true, Ordering::Relaxed);
    //     while !self.search_done.load(Ordering::Relaxed) {}
    // }

    pub fn resize_hash(&mut self, hash_size_bytes: usize) {
        self.trans_table = trans_table::TransTable::new(hash_size_bytes / trans_table::TransTable::entry_size());
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

        let mut game = self.game.make_move(best);
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
}

#[derive(Debug, Default)]
pub struct TimeControl {
    pub time_left: usize,
    pub time_incr: usize,
}
