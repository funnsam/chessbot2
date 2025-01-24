use std::{io::BufRead, str::FromStr};

use chess::{Board, Color, ALL_PIECES, ALL_SQUARES};
use chessbot2::eval::{EvalParamList, MAX_PHASE};

const ALPHA: f64 = 0.1;
const K: f64 = 0.4;
const BATCH: usize = 50000;
const MEAN: f64 = 1.0 / BATCH as f64;

fn main() {
    let mut eval_f64 = EvalParamList::<f64>::default();
    let mut eval_params = eval_f64.round_into_i16();

    let lines = std::io::stdin().lock().lines();
    let pos = lines.filter_map(|l| {
        let l = l.ok()?;
        let (fen, r) = l.split_once(',')?;
        let (_, eval) = r.split_once(',')?;

        Some((Board::from_str(fen).ok()?, eval.parse::<f64>().ok()?))
    }).collect::<Vec<_>>();

    println!("{} positions loaded", pos.len());

    for iteration in 0.. {
        let mut cost = 0.0;
        let mut eval_collector = EvalParamList::zeroed();

        for _ in 0..BATCH {
            let (board, r) = &pos[fastrand::usize(..pos.len())];
            let (mg, eg, w) = eval_params.get_separated_in_white(board);
            let eval = eval_params.evaluate_with((mg, eg, w)).0 as f64 / 100.0;

            let s = sigmoid(eval);
            let err = s - *r;
            cost += err * err * MEAN;

            let d_eval = (2.0 * err) * (K * s * (1.0 - s));
            // println!("{eval} {r} {s} {d_eval}");

            let d_mid = d_eval * (w as f64 / MAX_PHASE as f64);
            let d_end = d_eval * (1.0 - w as f64 / MAX_PHASE as f64);

            for square in board.combined().into_iter() {
                // SAFETY: only squares with things on it are checked
                let piece = unsafe { board.piece_on(square).unwrap_unchecked() };
                let color = unsafe { board.color_on(square).unwrap_unchecked() };

                let p = (color, piece, square);
                eval_collector.pst_mid[p] += 100.0 * d_mid;
                eval_collector.pst_end[p] += 100.0 * d_end;
                // println!("  {piece:?} {square}");
            }
        }

        println!("{iteration} {cost}");
        for piece in ALL_PIECES {
            for square in ALL_SQUARES {
                let p = (Color::White, piece, square);
                eval_f64.pst_mid[p] -= ALPHA * eval_collector.pst_mid[p] * MEAN;
                // eval_f64.pst_end[p] -= ALPHA * eval_collector.pst_end[p] * MEAN;
            }
        }
        // eval_f64.rook_open_file_bonus -= ALPHA * eval_collector.rook_open_file_bonus * MEAN;
        // eval_f64.king_pawn_penalty -= ALPHA * eval_collector.king_pawn_penalty * MEAN;
        // eval_f64.king_open_file_penalty -= ALPHA * eval_collector.king_open_file_penalty * MEAN;

        eval_params = eval_f64.round_into_i16();
    }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-K * x).exp())
}
