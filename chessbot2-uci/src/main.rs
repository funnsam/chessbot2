use std::io::BufRead;
use chessbot2::*;

mod uci;

fn main() {
    let mut lines = std::io::stdin().lock().lines();

    let mut engine = Engine::new(Game::new(chess::Board::default()), 64 * 1024 * 1024);
    let mut game_hash = engine.game.board().get_hash();
    let mut debug_mode = false;

    while let Some(Ok(l)) = lines.next() {
        let tokens = l.split_whitespace();
        match uci::parse_command(tokens) {
            Some(uci::UciCommand::Uci) => {
                println!("id name funn's bot");
                println!("id author funnsam");
                println!("uciok");
            },
            Some(uci::UciCommand::Debug(d)) => debug_mode = d,
            Some(uci::UciCommand::IsReady) => println!("readyok"),
            Some(uci::UciCommand::Quit) => std::process::exit(0),
            Some(uci::UciCommand::UciNewGame) => {},
            Some(uci::UciCommand::Position { position, moves }) => {
                engine.game = Game::new(position);
                for m in moves {
                    engine.game = engine.game.make_move(m);
                }
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

                let (mov, ..) = engine.best_move_iter_deep(|engine, (best, eval, depth)| {
                    println!(
                        "info score cp {eval} seldepth {depth} depth {depth} nodes {} pv {}",
                        engine.nodes_searched.load(std::sync::atomic::Ordering::Relaxed),
                        engine.find_pv(best).into_iter()
                            .map(|m| m.to_string())
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                    target_depth.map_or(true, |td| td > depth)
                });
                println!("bestmove {mov}");
            },
            None => println!("info string got unknown command {l}"),
        }
    }
}
