use core::str::FromStr;
use std::time::Duration;
use chess::*;
use chessbot2::TimeControl;

pub enum UciCommand<'a> {
    Uci,
    Debug(bool),
    IsReady,
    UciNewGame,
    Position {
        position: chessbot2::Game,
        moves: Vec<ChessMove>,
    },
    Go {
        depth: Option<usize>,
        movetime: Option<Duration>,
        wtime: Option<TimeControl>,
        btime: Option<TimeControl>,
        movestogo: Option<usize>,
    },
    SetOption(&'a str, Option<&'a str>),
    Quit,
    D,
    Eval,
    Move(ChessMove),
    Bench,
    Selfplay,
    Dump(&'a str),
}

fn move_from_uci(m: &str) -> ChessMove {
    let src = &m[0..2];
    let src = Square::new(((src.as_bytes()[1] - b'1') << 3) + (src.as_bytes()[0] - b'a'));

    let dst = &m[2..4];
    let dst = Square::new(((dst.as_bytes()[1] - b'1') << 3) + (dst.as_bytes()[0] - b'a'));

    let piece = m.as_bytes().get(4).and_then(|p| match p {
        b'n' => Some(Piece::Knight),
        b'b' => Some(Piece::Bishop),
        b'q' => Some(Piece::Queen),
        b'r' => Some(Piece::Rook),
        _ => None,
    });

    ChessMove::new(src, dst, piece)
}

pub fn parse_command<'a>(mut token: core::str::SplitWhitespace<'a>) -> Option<UciCommand<'a>> {
    match token.next() {
        Some("uci") => Some(UciCommand::Uci),
        Some("debug") => Some(UciCommand::Debug(token.next()? == "on")),
        Some("isready") => Some(UciCommand::IsReady),
        Some("ucinewgame") => Some(UciCommand::UciNewGame),
        Some("position") => {
            let mut moves = Vec::new();
            let next = token.next();
            let board = if matches!(next, Some("fen")) {
                let mut fen = String::new();

                while let Some(t) = token.next() {
                    if t == "moves" {
                        break;
                    }

                    fen += t;
                    fen += " ";
                }

                chessbot2::Game::from_str(fen.trim()).ok()?
            } else if matches!(next, Some("startpos")) {
                token.next();
                chessbot2::Game::default()
            } else {
                return None;
            };

            while let Some(m) = token.next() {
                moves.push(move_from_uci(m));
            }

            Some(UciCommand::Position {
                position: board,
                moves,
            })
        },
        Some("go") => {
            let mut depth = None;
            let mut movetime = None;
            let mut wtime = None;
            let mut btime = None;
            let mut winc = None;
            let mut binc = None;
            let mut movestogo = None;

            while let Some(t) = token.next() {
                match t {
                    "depth" => depth = token.next().and_then(|t| t.parse().ok()),
                    "movetime" => movetime = token.next().and_then(|t| Some(Duration::from_millis(t.parse().ok()?))),
                    "wtime" => wtime = token.next().and_then(|t| t.parse().ok()),
                    "btime" => btime = token.next().and_then(|t| t.parse().ok()),
                    "winc" => winc = token.next().and_then(|t| t.parse().ok()),
                    "binc" => binc = token.next().and_then(|t| t.parse().ok()),
                    "movestogo" => movestogo = token.next().and_then(|t| t.parse().ok()),
                    _ => {},
                }
            }

            Some(UciCommand::Go {
                depth,
                movetime,
                wtime: wtime.map(|time| TimeControl {
                    time_left: time,
                    time_incr: winc.unwrap_or(0),
                }),
                btime: btime.map(|time| TimeControl {
                    time_left: time,
                    time_incr: binc.unwrap_or(0),
                }),
                movestogo,
            })
        },
        Some("setoption") => {
            token.next();
            let name = token.next()?;
            token.next();
            let value = token.remainder();
            Some(UciCommand::SetOption(name, value))
        },
        Some("quit") => Some(UciCommand::Quit),
        Some("d") => Some(UciCommand::D),
        Some("eval") => Some(UciCommand::Eval),
        Some("move") => Some(UciCommand::Move(move_from_uci(token.next()?))),
        Some("bench") => Some(UciCommand::Bench),
        Some("selfplay") => Some(UciCommand::Selfplay),
        Some("dump") => Some(UciCommand::Dump(token.remainder()?)),
        Some(_) => parse_command(token),
        None => None,
    }
}
