use crate::*;

use chessbot2::Engine;

fn print_uci_info() {
    println!("id name chessbot2 v{VERSION}");
    println!("id author funnsam");
    println!(
        "option name Hash type spin default {DEFAULT_HASH_SIZE_MB} min 1 max {}",
        usize::MAX,
    );
    println!(
        "option name Threads type spin default {DEFAULT_THREADS} min 1 max {}",
        usize::MAX - 1,
    );
}

pub struct State {
    engine: Engine,
    debug_mode: bool,
}

impl State {
    pub fn new() -> Self {
        let mut engine = Engine::new(Game::new(chess::Board::default()), DEFAULT_HASH_SIZE_MB * MB);
        engine.start_smp(DEFAULT_THREADS - 1);

        Self {
            engine,
            debug_mode: false,
        }
    }

    pub fn handle_command<'a>(&mut self, command: Option<uci::UciCommand<'a>>) {
        match command {
            Some(uci::UciCommand::Uci) => {
                print_uci_info();
                println!("uciok");
            },
            Some(uci::UciCommand::SetOption(name, value)) => match name.to_ascii_lowercase().as_str() {
                "hash" => self.engine.resize_hash(value.unwrap().parse::<usize>().unwrap() * MB),
                "threads" => {
                    self.engine.kill_smp();
                    self.engine.start_smp(value.unwrap().parse::<usize>().unwrap() - 1);
                },
                _ => println!("info string got invalid setoption"),
            },
            Some(uci::UciCommand::Debug(d)) => self.debug_mode = d,
            Some(uci::UciCommand::IsReady) => println!("readyok"),
            Some(uci::UciCommand::Quit) => std::process::exit(0),
            Some(uci::UciCommand::UciNewGame) => {},
            Some(uci::UciCommand::Position { mut position, moves }) => {
                for m in moves {
                    position = position.make_move(m);
                }

                *self.engine.game.write() = position;
            },
            Some(uci::UciCommand::Move(m)) => {
                *self.engine.game.write() = self.engine.game.read().make_move(m)
            },
            Some(uci::UciCommand::Go { depth: target_depth, movetime, wtime, btime, movestogo }) => {
                let tc = if matches!(self.engine.game.read().board().side_to_move(), chess::Color::White) {
                    wtime
                } else {
                    btime
                };
                if let Some(mt) = movetime {
                    self.engine.allow_for(mt);
                } else if let Some(tc) = tc {
                    self.engine.time_control(movestogo, tc);
                } else {
                    self.engine.allow_for(std::time::Duration::MAX);
                }

                let mov = self.best_move(target_depth);
                if self.debug_mode {
                    // NOTE: getting the amount of tt used can be expensive, so it is only counted
                    // if in debug mode
                    println!("info hashfull {}", 1000 * self.engine.tt_used() / self.engine.tt_size());
                }
                println!("bestmove {mov}");
            },
            Some(uci::UciCommand::D) => print!("{:#}", self.engine.game.read()),
            Some(uci::UciCommand::Eval) => println!(
                "{:#}Eval: {}",
                self.engine.game.read(),
                self.engine.eval_params.evaluate_static(self.engine.game.read().board()),
            ),
            Some(uci::UciCommand::Bench) => self.benchmark(),
            Some(uci::UciCommand::Selfplay) => self.engine.self_play_gen_fen_csv(),
            None => {},
        }
    }

    fn benchmark(&mut self) {
        let mut results = [0; 8];

        *self.engine.game.write() = Game::from_str("r5k1/5pp1/P1p1P2p/2R5/3r4/6PP/1P2R1K1/8 b - - 0 34").unwrap();

        for (i, rec) in results.iter_mut().enumerate() {
            const ITERS: usize = 4;

            self.engine.kill_smp();
            self.engine.start_smp(i);

            let mut nodes = 0;
            for _ in 0..ITERS {
                self.engine.clear_hash();

                self.engine.allow_for(std::time::Duration::from_secs(1));
                self.engine.best_move(|_, (_, _, depth)| {
                    println!("{}t {depth}", i + 1);
                    true
                });

                nodes += self.engine.nodes();
            }

            *rec = nodes / ITERS;
        }

        println!("{results:?}");

        let m = results[1] as f32 - results[0] as f32;

        for (i, r) in results.into_iter().enumerate() {
            println!("{}t: {r} nps (linear: {}, {} × 1t)", i + 1, m * i as f32 + results[0] as f32, r as f32 / results[0] as f32);
        }
    }

    fn best_move(&mut self, target_depth: Option<usize>) -> chess::ChessMove {
        self.engine.best_move(|engine, (best, eval, depth)| {
            let time = engine.elapsed();
            let nodes = engine.nodes();

            println!(
                "info score {eval:#} depth {depth} nodes {nodes} time {} nps {} pv {}",
                time.as_millis(),
                (nodes as f64 / time.as_secs_f64()) as u64,
                engine.find_pv(best, if self.debug_mode { 100 } else { 20 }).into_iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            );
            target_depth.map_or(true, |td| td > depth)
        }).0
    }
}
