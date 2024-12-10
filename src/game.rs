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
}
