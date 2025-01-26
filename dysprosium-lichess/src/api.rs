use std::io::Read;

use chess::{ChessMove, Color, Piece, Square};
// use reqwest::{header, Client, Request, Response, Result as ReqResure
use serde::Deserialize;
use serde_json::from_str;
use ureq::{Request, Response};
use crate::{error, info, warn};

pub struct LichessApi {
    api_token: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Error<'a> {
    error: &'a str,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(bound = "'de: 'a")]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
#[serde(rename_all_fields = "camelCase")]
pub enum GameEvent<'a> {
    GameFull { initial_fen: &'a str, state: GameState<'a> },
    GameState { #[serde(flatten)] state: GameState<'a> },
    ChatLine {},
    OpponentGone {},
}

#[derive(Deserialize, Debug, Clone)]
pub struct GameState<'a> {
    pub moves: &'a str,
    pub wtime: usize,
    pub winc: usize,
    pub btime: usize,
    pub binc: usize,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(bound = "'de: 'a")]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
#[serde(rename_all_fields = "camelCase")]
pub enum Event<'a> {
    GameStart { game: Game<'a> },
    GameFinish { game: Game<'a> },
    Challenge { challenge: Challenge<'a> },
    ChallengeCanceled { challenge: Challenge<'a> },
    ChallengeDeclined { challenge: Challenge<'a> },
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Game<'a> {
    pub id: &'a str,
    pub color: ColorNt,
    pub fen: &'a str,
    pub opponent: Player<'a>,
    pub rated: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Challenge<'a> {
    pub direction: Option<Direction>,
    pub id: &'a str,
    pub challenger: Player<'a>,
    pub variant: Variant<'a>,
    pub speed: Speed,
    pub rated: bool,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    In, Out
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Player<'a> {
    pub id: Option<&'a str>,
    pub name: Option<&'a str>,
    pub username: Option<&'a str>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Variant<'a> {
    pub key: &'a str,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Speed {
    Ultrabullet,
    Bullet,
    Blitz,
    Rapid,
    Classical,
    Correspondence,
}

// #[derive(Deserialize, Debug, Clone)]
// #[serde(from = "&'de str")]
// pub struct ChessMoveNt(pub ChessMove);

#[derive(Deserialize, Debug, Clone)]
#[serde(from = "&'de str")]
pub struct ColorNt(pub Color);

// impl<'a> From<&'a str> for ChessMoveNt {
//     fn from(value: &'a str) -> Self {
//         Self(move_from_uci(value))
//     }
// }

impl<'a> From<&'a str> for ColorNt {
    fn from(value: &'a str) -> Self {
        Self(if value == "white" { Color::White } else { Color::Black })
    }
}

impl LichessApi {
    pub fn new(api_token: String) -> Self {
        Self { api_token }
    }

    fn request(&self, req: Request) -> Request {
        req.set("Authorization", &format!("Bearer {}", self.api_token))
    }

    fn http(&self, req: Request) -> Result<Response, ureq::Error> {
        self.request(req).call()
    }

    pub fn listen<F: FnMut(Event<'_>)>(&self, mut on_event: F) {
        let stream = self.http(ureq::get("https://lichess.org/api/stream/event")).unwrap().into_reader();
        let mut stream = JsonStreamIter::new(stream);

        info!("starting to listen for incoming games");

        while let Some(event) = stream.next_json::<Event<'_>>() {
            match event {
                Ok(Ok(Ok(ev))) => on_event(ev),
                Ok(Ok(Err(err))) => error!("got error in event stream: {err}"),
                Ok(Err(err)) => error!("got error in event stream: {err}"),
                Err(err) => error!("got error in event stream: {err}"),
            }
        }
    }

    pub fn listen_game<F: FnMut(GameEvent<'_>)>(&self, id: &str, mut on_event: F) {
        let stream = self.http(ureq::get(&format!("https://lichess.org/api/bot/game/stream/{id}"))).unwrap().into_reader();
        let mut stream = JsonStreamIter::new(stream);

        while let Some(event) = stream.next_json::<GameEvent<'_>>() {
            match event {
                Ok(Ok(Ok(ev))) => on_event(ev),
                Ok(Ok(Err(err))) => error!("got error in game event stream: {err}"),
                Ok(Err(err)) => error!("got error in game event stream: {err}"),
                Err(err) => error!("got error in game event stream: {err}"),
            }
        }
    }

    pub fn send_move(&self, game_id: &str, m: ChessMove) {
        loop {
            if let Ok(resp) = self.http(ureq::post(&format!("https://lichess.org/api/bot/game/{game_id}/move/{m}"))) {
                if !success(resp.status()) {
                    let reason = resp.into_string().unwrap();
                    let reason = from_str::<Error<'_>>(&reason).unwrap();
                    warn!("move {} invalid ({})", m, reason.error);
                }

                break;
            }
        }
    }

    pub fn accept_challenge(&self, id: &str) {
        if self.http(ureq::post(&format!("https://lichess.org/api/challenge/{id}/accept")))
            .ok()
            .and_then(|a| success(a.status()).then(|| ()))
            .is_none()
        {
            warn!("failed to accept challenge id {id}");
        }
    }

    pub fn decline_challenge(&self, id: &str, reason: &str) {
        if self.request(ureq::post(&format!("https://lichess.org/api/challenge/{id}/decline")))
            .send_form(&[("reason", reason)])
            .ok()
            .and_then(|a| success(a.status()).then(|| ()))
            .is_none()
        {
            warn!("failed to decline challenge id {id}");
        }
    }
}

struct JsonStreamIter<R: Read> {
    stream: R,
    buffer: Vec<u8>,
}

impl<R: Read> JsonStreamIter<R> {
    fn new(stream: R) -> Self {
        Self {
            stream,
            buffer: Vec::new(),
        }
    }

    fn next_json<'a, T: Deserialize<'a>>(&'a mut self) -> Option<Result<Result<serde_json::Result<T>, std::str::Utf8Error>, std::io::Error>> {
        let mut buf = [0];
        self.buffer.clear();

        loop {
            match self.stream.read(&mut buf) {
                Ok(b) if b == 1 => match buf[0] {
                    b'\n' if self.buffer.is_empty() => continue,
                    b'\n' => break,
                    b => self.buffer.push(b),
                },
                Err(err) => return Some(Err(err)),
                Ok(_) => return None,
            }
        }

        match std::str::from_utf8(&self.buffer) {
            Ok(s) => Some(Ok(Ok(from_str(s)))),
            Err(err) => Some(Ok(Err(err))),
        }
    }
}

pub fn move_from_uci(m: &str) -> ChessMove {
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

fn success(status: u16) -> bool {
    (200..=299).contains(&status)
}
