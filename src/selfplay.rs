use chess::{BoardStatus, Color};

use crate::{Engine, Game};

impl Engine {
    pub fn self_play_gen_fen_csv(&mut self) {
        let mut pos = std::collections::HashMap::new();
        self.allow_for(std::time::Duration::MAX);

        'outer: for i in 0..100 {
            self.game = Game::default();

            for _ in 0..16 {
                self.game = self.game.make_move(fastrand::choice(chess::MoveGen::new_legal(self.game.board())).unwrap());

                if self.game.board().status() != BoardStatus::Ongoing || self.game.can_declare_draw() {
                    continue 'outer;
                }
            }

            let mut this_pos = vec![];

            loop {
                let (m, e, _) = self.best_move(|s, (_, e, d)| {
                    println!("{i} {} {e} {d} {}", self.game.board(), self.nodes());
                    s.nodes() < 50_000 && d <= 10
                });
                if m == chess::ChessMove::default() { break };

                if self.game.board().status() != BoardStatus::Ongoing || self.game.can_declare_draw() || e.is_mate() || self.game.board().combined().popcnt() <= 4 {
                    break;
                }

                if !(self.game.is_capture(m) || self.game.is_in_check()) {
                    let e = if self.game.board().side_to_move() == Color::White { e } else { -e };

                    this_pos.push((self.game.get_fen(), e));
                }

                self.game = self.game.make_move(m);
            }

            for (fen, e) in this_pos {
                pos.insert(fen, e);
            }
            println!("game completed");
        }

        let mut f = std::fs::File::create("self_play_fens.csv").unwrap();
        for (fen, eval) in pos {
            use std::io::Write;

            writeln!(f, "{fen},{}", eval.0).unwrap();
        }
        println!("csv written");
    }
}
