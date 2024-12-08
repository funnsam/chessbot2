use core::str::FromStr;
use std::sync::{atomic::*, Arc};
use chess::*;
use reqwest::*;

#[macro_use]
mod log;

const DISALLOWED_TIME_CONTROLS: &[&str] = &["correspondence", "classical"];
const EXCEPTION_USERS: &[&str] = &["funnsam"];
const ACCEPT_RATED: bool = false;

pub struct LichessClient {
    api_token: String,
    pub active_games: AtomicUsize,
}

impl LichessClient {
    fn new(api_token: String) -> Self {
        Self { api_token, active_games: AtomicUsize::new(0) }
    }

    fn client(&self) -> Client {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(
                &format!("Bearer {}", self.api_token)
            ).unwrap()
        );
        Client::builder()
            .default_headers(headers)
            // .connection_verbose(true)
            .build().unwrap()
    }

    fn http<F: Fn(&Client) -> Request>(&self, f: F) -> impl Future<Output = Result<Response>> {
        let client = self.client();
        client.execute(f(&client))
    }

    pub async fn listen(self: Arc<Self>) {
        let stream = self.http(|c| c
            .get("https://lichess.org/api/stream/event")
            .build().unwrap()
        ).await.unwrap().bytes_stream();
        let mut stream = NdJsonIter::new(stream);

        info!("starting to listen for incoming games");

        while let Some(event) = stream.next_json().await {
            match event["type"].as_str() {
                Some("challenge") => {
                    let challenge = &event["challenge"];
                    let id = challenge["id"].as_str().unwrap();
                    let user = challenge["challenger"]["name"].as_str().unwrap();

                    let variant = challenge["variant"]["key"].as_str().unwrap();
                    let time_ctrl = challenge["speed"].as_str().unwrap();
                    let is_rated = challenge["rated"].as_bool().unwrap();
                    if EXCEPTION_USERS.contains(&user) || (
                        variant == "standard"
                        && !DISALLOWED_TIME_CONTROLS.contains(&time_ctrl)
                        && (ACCEPT_RATED || !is_rated)
                    ) {
                        info!("`{}` challenged bot (id: `{}`)", user, id);

                        if self.http(|c| c
                            .post(format!("https://lichess.org/api/challenge/{id}/accept"))
                            .build().unwrap()
                        ).await.ok().and_then(|a| a.status().is_success().then(|| ())).is_none() {
                            warn!("failed to accept challenge id {}", id);
                        }
                    } else {
                        if self.http(|c| c
                            .post(format!("https://lichess.org/api/challenge/{id}/decline"))
                            .build().unwrap()
                        ).await.ok().and_then(|a| a.status().is_success().then(|| ())).is_none() {
                            warn!("failed to decline challenge id {}", id);
                        }
                    }
                },
                Some("gameStart") => {
                    let game = &event["game"];
                    let id = game["id"].as_str().unwrap().to_string();
                    let user = game["opponent"]["username"].as_str().unwrap();
                    let color = match game["color"].as_str() {
                        Some("black") => Color::Black,
                        Some("white") => Color::White,
                        v => {
                            warn!("unknown color `{:?}`", v);
                            continue;
                        },
                    };
                    let fen = game["fen"].as_str().unwrap();
                    let board = Board::from_str(fen).unwrap();

                    info!("started a game with `{}` (id: `{}`, fen: `{}`)", user, id, fen);

                    let game = chessbot2::Game::new(board);
                    let arc = Arc::clone(&self);
                    tokio::spawn(async move { arc.play_game(id, game, color).await });
                },
                Some("gameFinish") => {
                    self.active_games.fetch_sub(1, Ordering::Relaxed);
                },
                Some("challengeCanceled" | "challengeDeclined") => {},
                Some(typ) => {
                    warn!("got unknown type of event `{}`", typ);
                    dbg!("{:?}", event);
                },
                None => {
                    warn!("got unknown type of event");
                    dbg!("{:?}", event);
                },
            }
        }
    }

    async fn play_game(self: Arc<Self>, game_id: String, game: chessbot2::Game, color: Color) {
        let mut engine = chessbot2::Engine::new(game);

        let color_prefix = if matches!(color, Color::White) { "w" } else { "b" };

        let stream = self.http(|c| c
            .get(format!("https://lichess.org/api/bot/game/stream/{game_id}"))
            .build().unwrap()
        ).await.unwrap().bytes_stream();
        let mut stream = NdJsonIter::new(stream);

        while let Some(event) = stream.next_json().await {
            match event["type"].as_str() {
                Some("gameFull") => {
                    let state = &event["state"];

                    let moves = state["moves"].as_str().unwrap().split_whitespace();
                    // for m in moves {
                    //     engine.game.make_move(move_from_uci(m));
                    // }

                    if engine.game.board().side_to_move() == color {
                        let time = state[color_prefix.to_string() + "time"].as_usize().unwrap();
                        let inc = state[color_prefix.to_string() + "inc"].as_usize().unwrap();

                        engine.time_ctrl = chessbot2::TimeControl {
                            time_left: time,
                            time_incr: inc,
                        };

                        let (next, _, _) = engine.best_move_iter_deep(|engine, (best, eval, depth)| {
                            info!(
                                "depth {} nodes: {} best: {} eval: {}cp",
                                depth,
                                engine.nodes_searched.load(std::sync::atomic::Ordering::Relaxed),
                                best,
                                eval,
                            );
                        });
                        self.send_move(&game_id, next).await;
                    }
                },
                Some("gameState") => {
                    let m = event["moves"].as_str().unwrap().split_whitespace().last().unwrap();
                    engine.game = engine.game.make_move(move_from_uci(m));

                    if engine.game.board().side_to_move() == color {
                        let time = event[color_prefix.to_string() + "time"].as_usize().unwrap();
                        let inc = event[color_prefix.to_string() + "inc"].as_usize().unwrap();

                        engine.time_ctrl = chessbot2::TimeControl {
                            time_left: time,
                            time_incr: inc,
                        };

                        let (next, _, _) = engine.best_move_iter_deep(|engine, (best, eval, depth)| {
                            info!(
                                "depth {} nodes: {} best: {} eval: {}cp",
                                depth,
                                engine.nodes_searched.load(std::sync::atomic::Ordering::Relaxed),
                                best,
                                eval,
                            );
                        });
                        self.send_move(&game_id, next).await;
                    }
                },
                Some(typ) => {
                    warn!("got unknown type of event `{}`", typ);
                    dbg!("{:?}", event);
                },
                None => {
                    warn!("got unknown type of event");
                    dbg!("{:?}", event);
                },
            }
        }

        info!("stream ended (id: `{}`)", game_id);
    }

    async fn send_move(&self, game_id: &str, m: ChessMove) {
        let client = Client::new();
        let resp = client.execute(
            client
            .post(format!("https://lichess.org/api/bot/game/{game_id}/move/{m}"))
            .header("Authorization", format!("Bearer {}", self.api_token))
            .build().unwrap()
            ).await.unwrap();

        if !resp.status().is_success() {
            let reason = json::parse(&resp.text().await.unwrap()).unwrap();
            let reason = reason["error"].as_str().unwrap();
            warn!("move {} invalid ({})", m, reason);
        }
    }
}

struct NdJsonIter<S: Send + futures_util::stream::Stream<Item = Result<bytes::Bytes>>> {
    stream: S,
    buffer: Vec<u8>,
    leftover: Vec<u8>,
}

impl<S: Send + futures_util::stream::Stream<Item = Result<bytes::Bytes>> + std::marker::Unpin> NdJsonIter<S> {
    fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: Vec::new(),
            leftover: Vec::new(),
        }
    }

    async fn next_json(&mut self) -> Option<json::JsonValue> {
        self.buffer.clear();

        let mut used = 0;
        let mut done = false;
        for b in self.leftover.iter() {
            used += 1;
            if *b != b'\n' {
                self.buffer.push(*b);
            } else if !self.buffer.is_empty() {
                done = true;
                break;
            }
        }

        self.leftover = self.leftover[used..].to_vec();

        if done {
            return json::parse(std::str::from_utf8(&self.buffer).ok()?).ok();
        }

        use futures_util::stream::StreamExt;
        'a: while let Some(Ok(i)) = self.stream.next().await {
            for (j, b) in i.iter().enumerate() {
                if *b != b'\n' {
                    self.buffer.push(*b);
                } else if !self.buffer.is_empty() {
                    self.leftover.extend(&i[j..]);
                    break 'a;
                } else { std::hint::black_box(()); }
            }

        }
        json::parse(std::str::from_utf8(&self.buffer).ok()?).ok()
    }
}

fn move_from_uci(m: &str) -> ChessMove {
    let src = &m[0..2];
    let src = unsafe {
        Square::new(((src.as_bytes()[1] - b'1') << 3) + (src.as_bytes()[0] - b'a'))
    };

    let dst = &m[2..4];
    let dst = unsafe {
        Square::new(((dst.as_bytes()[1] - b'1') << 3) + (dst.as_bytes()[0] - b'a'))
    };

    let piece = m.as_bytes().get(4).and_then(|p| match p {
        b'n' => Some(Piece::Knight),
        b'b' => Some(Piece::Bishop),
        b'q' => Some(Piece::Queen),
        b'r' => Some(Piece::Rook),
        _ => None,
    });

    ChessMove::new(src, dst, piece)
}

#[tokio::main]
async fn main() {
    let api_key = std::fs::read_to_string("api_key.txt").unwrap().trim().to_string();
    Arc::new(LichessClient::new(api_key)).listen().await;
}
