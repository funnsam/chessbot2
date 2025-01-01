use core::str::FromStr;
use std::{pin::Pin, sync::{atomic::*, Arc}, task::{Context, Poll}};
use api::{move_from_uci, Challenge, Direction, Event, GameEvent, GameState, LichessApi, Player, Speed, Variant};
use chess::*;

mod api;
mod log;

const DISALLOWED_TIME_CONTROLS: &[Speed] = &[Speed::Correspondence, Speed::Classical];
const EXCEPTION_USERS: &[&str] = &["funnsam"];
const ACCEPT_RATED: bool = false;

pub struct LichessClient {
    api: LichessApi,
    pub active_games: AtomicUsize,
}

impl LichessClient {
    pub fn new(api: LichessApi) -> Self {
        Self { api, active_games: AtomicUsize::new(0) }
    }

    pub async fn listen(self: Arc<Self>) {
        self.api.listen(async |event| match event {
            Event::Challenge { challenge: Challenge { direction, id, challenger: Player { name: Some(challenger), .. }, variant: Variant { key: variant }, speed, rated } } => {
                if direction == Direction::Out { return };
                let is_su = EXCEPTION_USERS.contains(&challenger);

                info!("user `{challenger}` challenged bot (id: `{id}`, variant: {variant:?}, time control: {speed:?}, rated: {rated})");
                if !is_su && variant != "standard" {
                    self.api.decline_challenge(id, "standard").await;
                } else if !is_su && DISALLOWED_TIME_CONTROLS.contains(&speed) {
                    self.api.decline_challenge(id, "timeControl").await;
                } else if !is_su && !ACCEPT_RATED && rated {
                    self.api.decline_challenge(id, "casual").await;
                } else {
                    // self.api.accept_challenge(id).await;
                }
            },
            Event::GameStart { game: api::Game { id, color, fen, opponent, .. } } => {
                let game = chessbot2::Game::from_str(fen).unwrap();

                info!("started a game with `{}` (id: `{id}`, fen: `{fen}`)", opponent.username.unwrap());

                pub struct IamSend<F: Future>(F);
                unsafe impl<F: Future> Send for IamSend<F> {}
                impl<F: Future> Future for IamSend<F> {
                    type Output = F::Output;
                    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                        unsafe { self.map_unchecked_mut(|s| &mut s.0).poll(cx) }
                    }
                }

                let arc = Arc::clone(&self);
                let id = id.to_string();
                tokio::spawn(IamSend(async move { arc.play_game(id, game, color.0).await }));
            },
            Event::GameFinish { .. } => {
                self.active_games.fetch_sub(1, Ordering::Relaxed);
            },
            _ => {},
        }).await;
    }

    async fn play_game(self: Arc<Self>, game_id: String, game: chessbot2::Game, color: Color) {
        let mut engine = chessbot2::Engine::new(game, 64 * 1024 * 1024);

        self.api.listen_game(&game_id, async |event| match event {
            GameEvent::GameFull { state } | GameEvent::GameState { state } => {
                if let Some(m) = state.moves.split_whitespace().last() {
                    engine.game = engine.game.make_move(move_from_uci(m));
                }

                if engine.game.board().side_to_move() == color {
                    self.play(&game_id, color, state, &mut engine).await;
                }
            },
            _ => {},
        }).await;

        info!("stream ended (id: `{}`)", game_id);
    }

    async fn play(&self, game_id: &str, color: Color, state: GameState<'_>, engine: &mut chessbot2::Engine) {
        engine.time_control(match color {
            Color::White => chessbot2::TimeControl {
                time_left: state.wtime,
                time_incr: state.winc,
            },
            Color::Black => chessbot2::TimeControl {
                time_left: state.btime,
                time_incr: state.binc,
            },
        });

        let (next, _, _) = engine.best_move(|engine, (best, eval, depth)| {
            let nodes = engine.nodes();
            let time = engine.elapsed().as_secs_f64();

            info!(
                "searched {nodes} nodes at {depth}-ply deep in {time:.2}s ({:.2} MN/s), PV: {} ({eval})",
                nodes as f64 / time / 1_000_000.0,
                engine.find_pv(best, 20).into_iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            );
            true
        });
        self.api.send_move(game_id, next).await;
    }
}

#[tokio::main]
async fn main() {
    let api_key = std::fs::read_to_string("api_key.txt").unwrap().trim().to_string();
    Arc::new(LichessClient::new(LichessApi::new(api_key))).listen().await;
}
