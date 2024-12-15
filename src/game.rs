use core::str::FromStr;

#[derive(Clone)]
pub struct Game {
    board: chess::Board,
    fifty_move_counter: usize,
    hash_history: Vec<u64>,
}

impl Game {
    pub fn new(board: chess::Board) -> Self {
        Self {
            board,
            fifty_move_counter: 0,
            hash_history: vec![],
        }
    }

    pub fn board(&self) -> &chess::Board { &self.board }

    pub fn make_move(&self, mov: chess::ChessMove) -> Self {
        let mut fifty_move_counter = self.fifty_move_counter + 1;

        let is_pawn = (self.board.pieces(chess::Piece::Pawn) & chess::BitBoard::from_square(mov.get_source())).0 != 0;
        let is_capture = (self.board.combined() & chess::BitBoard::from_square(mov.get_dest())).0 != 0;

        if is_pawn || is_capture {
            fifty_move_counter = 0;
        }

        let board = self.board.make_move_new(mov);
        let mut hash_history = self.hash_history.clone();
        hash_history.push(board.get_hash());

        Self { board, fifty_move_counter, hash_history }
    }

    pub fn make_null_move(&self) -> Option<Self> {
        let board = self.board.null_move()?;
        let fifty_move_counter = self.fifty_move_counter + 1;
        let mut hash_history = self.hash_history.clone();
        hash_history.push(board.get_hash());

        Some(Self { board, fifty_move_counter, hash_history })
    }

    pub fn can_declare_draw(&self) -> bool {
        for h in self.hash_history.iter() {
            if self.hash_history.iter().filter(|i| h == *i).count() >= 3 {
                return true;
            }
        }

        self.fifty_move_counter >= 100
    }

    pub fn history_len(&self) -> usize { self.hash_history.len() }

    pub fn get_fen(&self) -> String {
        let rfen = self.board().to_string();
        format!(
            "{} {} {}",
            &rfen[..rfen.len() - 4], self.fifty_move_counter,
            self.hash_history.len() / 2 + 1,
        )
    }
}

impl core::fmt::Display for Game {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // let atk = self.board().pseudo_attacks(self.board().side_to_move());

        for rank in chess::ALL_RANKS.iter().rev() {
            let get = |file| {
                let sq = chess::Square::make_square(*rank, file);
                let bg = if (file.to_index() + rank.to_index()) & 1 == 0 { 232 } else { 234 };
                // let bg = if (chess::BitBoard::from_square(sq) & atk).0 != 0 { 1 } else { 232 };

                self.board().piece_on(sq).map_or_else(
                    || if f.alternate() { format!("\x1b[48;5;{bg}m \x1b[0m") } else { " ".to_string() },
                    |p| {
                        let c = self.board().color_on(sq).unwrap();

                        if f.alternate() {
                            format!("\x1b[1;38;5;{};48;5;{bg}m{}\x1b[0m", 255 - c.to_index() * 8, p.to_string(c))
                        } else {
                            p.to_string(c)
                        }
                    }
                )
            };

            let pa = get(chess::File::A);
            let pb = get(chess::File::B);
            let pc = get(chess::File::C);
            let pd = get(chess::File::D);
            let pe = get(chess::File::E);
            let pf = get(chess::File::F);
            let pg = get(chess::File::G);
            let ph = get(chess::File::H);

            if *rank == chess::Rank::Eighth {
                writeln!(f, "┌───┬───┬───┬───┬───┬───┬───┬──{}┐", ['₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈'][rank.to_index()])?;
            } else {
                writeln!(f, "├───┼───┼───┼───┼───┼───┼───┼──{}┤", ['₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈'][rank.to_index()])?;
            }
            writeln!(f, "│ {pa} │ {pb} │ {pc} │ {pd} │ {pe} │ {pf} │ {pg} │ {ph} │")?;
        }

        writeln!(f, "└ᵃ──┴ᵇ──┴ᶜ──┴ᵈ──┴ᵉ──┴ᶠ──┴ᵍ──┴ʰ──┘")?;
        writeln!(f)?;

        writeln!(f, "FEN: {}", self.get_fen())?;
        writeln!(f, "Hash: 0x{:016x}", self.board().get_hash())?;
        writeln!(f)?;

        let phase = crate::eval::game_phase(self.board()) as usize;
        writeln!(f, "Phase: {0:█<1$}{0:░<phase$} end", "", 24 - phase)?;

        Ok(())
    }
}

impl FromStr for Game {
    type Err = Box<dyn core::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (rest, moves) = s.rsplit_once(' ').ok_or("")?;
        let (_, fmc) = rest.rsplit_once(' ').ok_or("")?;
        let board = chess::Board::from_str(s).map_err(|_| "")?;

        Ok(Self {
            board,
            fifty_move_counter: fmc.parse()?,
            hash_history: (0..moves.parse::<u64>()? * 2 - (board.side_to_move() == chess::Color::White) as u64).collect(),
        })
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new(chess::Board::default())
    }
}
