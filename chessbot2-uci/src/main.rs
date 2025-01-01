#![feature(str_split_whitespace_remainder)]

use std::io::BufRead;
use chessbot2::*;

mod uci;

const DEFAULT_HASH_SIZE_MB: usize = 64;
const MB: usize = 1024 * 1024;

fn main() {
//     println!("{:?} {}", stacker::remaining_stack(), core::mem::size_of::<Engine>());
//     stacker::grow(32 * MB, _main);
// }

// fn _main() {
    let mut lines = std::io::stdin().lock().lines();

    let mut engine = Engine::new(Game::new(chess::Board::default()), DEFAULT_HASH_SIZE_MB * MB);
    let mut debug_mode = false;

    while let Some(Ok(l)) = lines.next() {
        let tokens = l.split_whitespace();
        match uci::parse_command(tokens) {
            Some(uci::UciCommand::Uci) => {
                println!("id name funn's bot");
                println!("id author funnsam");
                println!("option name Hash type spin default {DEFAULT_HASH_SIZE_MB} min 0 max 16384");
                println!("uciok");
            },
            Some(uci::UciCommand::SetOption(name, value)) => match name.to_ascii_lowercase().as_str() {
                "hash" => engine.resize_hash(value.unwrap().parse::<usize>().unwrap() * MB),
                _ => println!("info string got invalid setoption option"),
            },
            Some(uci::UciCommand::Debug(d)) => debug_mode = d,
            Some(uci::UciCommand::IsReady) => println!("readyok"),
            Some(uci::UciCommand::Quit) => std::process::exit(0),
            Some(uci::UciCommand::UciNewGame) => {},
            Some(uci::UciCommand::Position { mut position, moves }) => {
                for m in moves {
                    position = position.make_move(m);
                }

                engine.game = position;
            },
            Some(uci::UciCommand::Go { depth: target_depth, movetime, wtime, btime }) => {
                let tc = if matches!(engine.game.board().side_to_move(), chess::Color::White) {
                    wtime
                } else {
                    btime
                };
                if let Some(mt) = movetime {
                    engine.allow_for(mt);
                } else if let Some(tc) = tc {
                    engine.time_control(tc);
                } else {
                    engine.allow_for(std::time::Duration::MAX);
                }

                let mov = best_move(debug_mode, &engine, target_depth);
                if debug_mode {
                    // NOTE: getting the amount of tt used can be expensive, so it is only counted
                    // if in debug mode
                    println!("info hashfull {}", 1000 * engine.tt_used() / engine.tt_size());
                }
                println!("bestmove {mov}");
            },
            Some(uci::UciCommand::D) => print!("{:#}", engine.game),
            Some(uci::UciCommand::Eval) => println!(
                "{:#}Eval: {}",
                engine.game,
                evaluate_static(engine.game.board()),
            ),
            Some(uci::UciCommand::Move(m)) => engine.game = engine.game.make_move(m),
            Some(uci::UciCommand::Bench) => {
                const ITERS: usize = 32;

                let mut _time = 0;

                engine.allow_for(std::time::Duration::MAX);
                for _ in 0..ITERS {
                    engine.resize_hash(DEFAULT_HASH_SIZE_MB * MB);

                    engine.best_move(|engine, (best, eval, depth)| {
                        let time = engine.elapsed();
                        let nodes = engine.nodes();

                        if depth == 8 {
                            _time += time.as_millis();
                        }

                        println!(
                            "info score {eval:#} seldepth {depth} depth {depth} nodes {nodes} time {} nps {} pv {}",
                            time.as_millis(),
                            (nodes as f64 / time.as_secs_f64()) as u64,
                            engine.find_pv(best, if debug_mode { 100 } else { 20 }).into_iter()
                            .map(|m| m.to_string())
                            .collect::<Vec<_>>()
                            .join(" "),
                        );
                        8 > depth
                    });
                }

                println!("depth 8 avg: {:.2}ms", _time as f32 / ITERS as f32);
            },
            None => println!("info string got unknown command {l}"),
        }
    }
}

fn best_move(debug_mode: bool, engine: &Engine, target_depth: Option<usize>) -> chess::ChessMove {
    engine.best_move(|engine, (best, eval, depth)| {
        let time = engine.elapsed();
        let nodes = engine.nodes();

        println!(
            "info score {eval:#} seldepth {depth} depth {depth} nodes {nodes} time {} nps {} pv {}",
            time.as_millis(),
            (nodes as f64 / time.as_secs_f64()) as u64,
            engine.find_pv(best, if debug_mode { 100 } else { 20 }).into_iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join(" "),
        );
        target_depth.map_or(true, |td| td > depth)
    }).0
}
