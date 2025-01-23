#![feature(str_split_whitespace_remainder)]

use std::{io::BufRead, str::FromStr};
use chessbot2::*;

mod client;
mod uci;

const DEFAULT_HASH_SIZE_MB: usize = 64;
const DEFAULT_THREADS: usize = 1;
const MB: usize = 1024 * 1024;

fn main() {
    println!("chessbot2 v{VERSION} licensed under GPLv3");

    let mut client = client::State::new();

    let stdin = std::io::stdin().lock().lines();
    for l in std::env::args().skip(1).map(Ok).chain(stdin) {
        if let Ok(l) = l {
            let tokens = l.split_whitespace();
            client.handle_command(uci::parse_command(tokens));
        }
    }
}
