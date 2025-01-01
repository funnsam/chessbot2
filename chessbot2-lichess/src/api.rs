use chess::{ChessMove, Color, Piece, Square};
use reqwest::{header, Client, Request, Response, Result as ReqResult};
use serde::Deserialize;
use serde_json::from_str;
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
    GameFull { state: GameState<'a> },
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

    fn http<F: Fn(&Client) -> Request>(&self, f: F) -> impl Future<Output = ReqResult<Response>> {
        let client = self.client();
        client.execute(f(&client))
    }

    pub async fn listen<F: AsyncFnMut(Event<'_>)>(&self, mut on_event: F) {
        let stream = self.http(|c| c
            .get("https://lichess.org/api/stream/event")
            .build().unwrap()
        ).await.unwrap().bytes_stream();
        let mut stream = NdJsonIter::new(stream);

        info!("starting to listen for incoming games");

        while let Some(event) = stream.next_json::<Event<'_>>().await {
            match event {
                Ok(ev) => on_event(ev).await,
                Err(err) => error!("got error in event stream: {err}"),
            }
        }
    }

    pub async fn listen_game<F: AsyncFnMut(GameEvent<'_>)>(&self, id: &str, mut on_event: F) {
        let stream = self.http(|c| c
            .get(format!("https://lichess.org/api/bot/game/stream/{id}"))
            .build().unwrap()
        ).await.unwrap().bytes_stream();
        let mut stream = NdJsonIter::new(stream);

        while let Some(event) = stream.next_json::<GameEvent<'_>>().await {
            match event {
                Ok(ev) => on_event(ev).await,
                Err(err) => error!("got error in game event stream: {err}"),
            }
        }
    }

    pub async fn send_move(&self, game_id: &str, m: ChessMove) {
        loop {
            let client = Client::new();
            if let Ok(resp) = client.execute(
                client
                .post(format!("https://lichess.org/api/bot/game/{game_id}/move/{m}"))
                .header("Authorization", format!("Bearer {}", self.api_token))
                .build().unwrap()
            ).await {
                if !resp.status().is_success() {
                    let reason = resp.text().await.unwrap();
                    let reason = from_str::<Error<'_>>(&reason).unwrap();
                    warn!("move {} invalid ({})", m, reason.error);
                }

                break;
            }
        }
    }

    pub async fn accept_challenge(&self, id: &str) {
        if self.http(|c| c
            .post(format!("https://lichess.org/api/challenge/{id}/accept"))
            .build().unwrap()
        ).await.ok().and_then(|a| a.status().is_success().then(|| ())).is_none() {
            warn!("failed to accept challenge id {id}");
        }
    }

    pub async fn decline_challenge(&self, id: &str, reason: &str) {
        if self.http(|c| c
            .post(format!("https://lichess.org/api/challenge/{id}/decline"))
            .body(format!("reason={reason}"))
            .build().unwrap()
        ).await.ok().and_then(|a| a.status().is_success().then(|| ())).is_none() {
            warn!("failed to decline challenge id {id}");
        }
    }
}

struct NdJsonIter<S: Send + futures_util::stream::Stream<Item = ReqResult<bytes::Bytes>>> {
    stream: S,
    buffer: Vec<u8>,
    leftover: Vec<u8>,
}

impl<S: Send + futures_util::stream::Stream<Item = ReqResult<bytes::Bytes>> + std::marker::Unpin> NdJsonIter<S> {
    fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: Vec::new(),
            leftover: Vec::new(),
        }
    }

    async fn next_json<'a, T: 'a + Deserialize<'a>>(&'a mut self) -> Option<Result<T, serde_json::Error>> {
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
            return Some(from_str(std::str::from_utf8(&self.buffer).ok()?));
        }

        use futures_util::stream::StreamExt;
        'l: loop {
            match self.stream.next().await {
                Some(Ok(i)) => for (j, b) in i.iter().enumerate() {
                    if *b != b'\n' {
                        self.buffer.push(*b);
                    } else if !self.buffer.is_empty() {
                        self.leftover.extend(&i[j..]);
                        break 'l;
                    } else {
                        std::hint::black_box(());
                    }
                },
                Some(Err(i)) => panic!("{i}"),
                None => return None,
            }
        }

        Some(from_str(std::str::from_utf8(&self.buffer).ok()?))
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
