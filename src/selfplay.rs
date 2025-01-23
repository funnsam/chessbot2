use chess::{BoardStatus, Color};

use crate::{Engine, Game};

impl Engine {
    pub fn self_play_gen_fen_csv(&mut self) {
        let mut positions = 0;
        let mut file = std::fs::File::create(format!("self_play_fen_{}.csv", fastrand::usize(..))).unwrap();
        self.allow_for(std::time::Duration::MAX);

        'outer: for game_no in 0..5_000 {
            {
                let mut game = self.game.write();
                *game = Game::default();

                for _ in 0..8 {
                    *game = game.make_move(fastrand::choice(chess::MoveGen::new_legal(game.board())).unwrap());

                    if game.board().status() != BoardStatus::Ongoing || game.can_declare_draw() {
                        continue 'outer;
                    }
                }
            }

            let mut this_pos = vec![];
            let mut is_mate = false;
            let mut is_pos_mate = false;
            self.trans_table.clear();

            loop {
                self.allow_for(std::time::Duration::from_millis(250));
                let (m, e, _) = self.best_move(|_, _| true);

                let mut game = self.game.write();
                if m == chess::ChessMove::default() {
                    println!("warn: null move picked {}", game.get_fen());
                    continue 'outer;
                }

                let fen = game.get_fen();
                *game = game.make_move(m);

                is_mate |= e.is_mate();
                is_pos_mate |= e.is_positive_mate();
                if game.board().status() != BoardStatus::Ongoing || game.can_declare_draw() || !game.is_sufficient_material() || e.is_mate() {
                    break;
                }

                this_pos.push((fen, e));
                positions += 1;

                if positions % 50 == 0 {
                    println!("{positions} positions");
                }
            }

            let game = self.game.read();
            let wdl = if is_mate { (is_pos_mate ^ (game.board().side_to_move() == Color::White)) as u8 as f32 } else { 0.5 };
            for (fen, e) in this_pos {
                use std::io::Write;

                writeln!(file, "{fen},{e},{wdl}").unwrap();
            }
            println!("game {game_no} completed");
        }
        println!("completed");
    }
}
