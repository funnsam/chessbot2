use chess::{BoardStatus, Color};

use crate::{Engine, Game};

impl Engine {
    pub fn self_play_gen_fen_csv(&mut self) {
        let mut pos = std::collections::HashMap::new();
        self.allow_for(std::time::Duration::MAX);

        'outer: for _ in 0..3_000 {
            self.game = Game::default();

            for _ in 0..16 {
                self.game = self.game.make_move(fastrand::choice(chess::MoveGen::new_legal(self.game.board())).unwrap());

                if self.game.board().status() != BoardStatus::Ongoing || self.game.can_declare_draw() {
                    continue 'outer;
                }
            }

            let mut this_pos = vec![];

            loop {
                let (m, e, _) = self.best_move(|s, _| s.nodes() < 15_000);
                assert_ne!(m, chess::ChessMove::default(), "{}", self.game.get_fen());
                self.game = self.game.make_move(m);

                if self.game.board().status() != BoardStatus::Ongoing || self.game.can_declare_draw() || e.is_mate() {
                    break;
                }

                this_pos.push((self.game.get_fen(), e));
            }

            let wdl = if self.game.board().status() == BoardStatus::Checkmate { (self.game.board().side_to_move() == Color::Black) as u8 as f32 } else { 0.5 };
            for (fen, e) in this_pos {
                pos.insert(fen, (e, wdl));
            }
            println!("game completed");
        }

        let mut f = std::fs::File::create("self_play_fens.csv").unwrap();
        for (fen, (eval, wdl)) in pos {
            use std::io::Write;

            writeln!(f, "{fen},{},{wdl}", eval.0).unwrap();
        }
        println!("csv written");
    }
}
