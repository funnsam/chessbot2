pub use game::Game;
use std::time::*;

mod eval;
pub mod game;
mod search;
mod shared_table;
mod trans_table;

pub struct Engine {
    pub game: Game,
    pub trans_table: trans_table::TransTable,

    pub time_ctrl: TimeControl,
    pub time_ref: Instant,
    pub time_usable: Duration,
}

impl Engine {
    pub fn new(game: Game) -> Self {
        Self {
            game,

            trans_table: trans_table::TransTable::new(32 * 1024 * 1024 / trans_table::TransTable::entry_size()),
            time_ctrl: TimeControl::default(),
            time_ref: Instant::now(),
            time_usable: Duration::default(),
        }
    }

    fn reserve_time(&mut self) {
        // https://github.com/SebLague/Chess-Coding-Adventure/blob/Chess-V2-UCI/Chess-Coding-Adventure/src/Bot.cs#L64

        let left = self.time_ctrl.time_left as u64;
        let incr = self.time_ctrl.time_incr as u64;

        let mut think_time = left / 40;

        if left > incr << 2 {
            think_time += incr * 4 / 5;
        }

        let min_think = (left / 4).min(50);
        self.time_usable = Duration::from_millis(min_think.max(think_time));
    }

    pub fn times_up(&self) -> bool {
        self.time_ref.elapsed() > self.time_usable
    }
}

#[derive(Debug, Default)]
pub struct TimeControl {
    pub time_left: usize,
    pub time_incr: usize,
}
