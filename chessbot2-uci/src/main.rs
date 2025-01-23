#![feature(str_split_whitespace_remainder)]

use core::usize::MAX as USIZE_MAX;
use std::{io::BufRead, str::FromStr};
use chessbot2::*;

mod uci;

const DEFAULT_HASH_SIZE_MB: usize = 64;
const DEFAULT_THREADS: usize = 1;
const MB: usize = 1024 * 1024;

fn main() {
    let mut lines = std::io::stdin().lock().lines();

    let mut engine = Engine::new(Game::new(chess::Board::default()), DEFAULT_HASH_SIZE_MB * MB);
    engine.start_smp(DEFAULT_THREADS - 1);
    let mut debug_mode = false;

    while let Some(Ok(l)) = lines.next() {
        let tokens = l.split_whitespace();
        match uci::parse_command(tokens) {
            Some(uci::UciCommand::Uci) => {
                println!("id name funn's bot");
                println!("id author funnsam");
                println!("option name Hash type spin default {DEFAULT_HASH_SIZE_MB} min 0 max {USIZE_MAX}");
                println!("option name Threads type spin default {DEFAULT_THREADS} min 1 max {USIZE_MAX}");
                println!("uciok");
            },
            Some(uci::UciCommand::SetOption(name, value)) => match name.to_ascii_lowercase().as_str() {
                "hash" => engine.resize_hash(value.unwrap().parse::<usize>().unwrap() * MB),
                "threads" => {
                    engine.kill_smp();
                    engine.start_smp(value.unwrap().parse::<usize>().unwrap() - 1);
                },
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

                *engine.game.write() = position;
            },
            Some(uci::UciCommand::Go { depth: target_depth, movetime, wtime, btime, movestogo }) => {
                let tc = if matches!(engine.game.read().board().side_to_move(), chess::Color::White) {
                    wtime
                } else {
                    btime
                };
                if let Some(mt) = movetime {
                    engine.allow_for(mt);
                } else if let Some(tc) = tc {
                    engine.time_control(movestogo, tc);
                } else {
                    engine.allow_for(std::time::Duration::MAX);
                }

                let mov = best_move(debug_mode, &mut engine, target_depth);
                if debug_mode {
                    // NOTE: getting the amount of tt used can be expensive, so it is only counted
                    // if in debug mode
                    println!("info hashfull {}", 1000 * engine.tt_used() / engine.tt_size());
                }
                println!("bestmove {mov}");
            },
            Some(uci::UciCommand::D) => print!("{:#}", engine.game.read()),
            Some(uci::UciCommand::Eval) => println!(
                "{:#}Eval: {}",
                engine.game.read(),
                evaluate_static(engine.game.read().board()),
            ),
            Some(uci::UciCommand::Move(m)) => *engine.game.write() = engine.game.read().make_move(m),
            Some(uci::UciCommand::Bench) => {
                let mut results = [0; 8];

                *engine.game.write() = Game::from_str("r5k1/5pp1/P1p1P2p/2R5/3r4/6PP/1P2R1K1/8 b - - 0 34").unwrap();

                for (i, rec) in results.iter_mut().enumerate() {
                    const ITERS: usize = 4;

                    engine.kill_smp();
                    engine.start_smp(i);

                    let mut nodes = 0;
                    for _ in 0..ITERS {
                        engine.clear_hash();

                        engine.allow_for(std::time::Duration::from_secs(1));
                        engine.best_move(|_, (_, _, depth)| {
                            println!("{}t {depth}", i + 1);
                            true
                        });

                        nodes += engine.nodes();
                    }

                    *rec = nodes / ITERS;
                }

                println!("{results:?}");

                let m = results[1] as f32 - results[0] as f32;

                for (i, r) in results.into_iter().enumerate() {
                    println!("{}t: {r} nps (linear: {}, {} × 1t)", i + 1, m * i as f32 + results[0] as f32, r as f32 / results[0] as f32);
                }
            },
            None => println!("info string got unknown command {l}"),
        }
    }
}

fn best_move(debug_mode: bool, engine: &mut Engine, target_depth: Option<usize>) -> chess::ChessMove {
    engine.best_move(|engine, (best, eval, depth)| {
        let time = engine.elapsed();
        let nodes = engine.nodes();

        println!(
            "info score {eval:#} depth {depth} nodes {nodes} time {} nps {} pv {}",
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
