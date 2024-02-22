use rand::Rng;
use crate::frame::{util::*, board::Board};

pub struct Zobrist {
    pub hash_boards:       [[u64; 64]; K2 + 1],
    pub hash_en_passant:   [u64; 64],
    pub hash_castlings:    [u64; 16],
    pub hash_turn:         u64
}

// Let us hope we are lucky!
impl Default for Zobrist {
    fn default() -> Zobrist {
        let mut rng = rand::thread_rng();

        let mut hash_boards = [[0; 64]; K2 + 1];
        let mut hash_en_passant = [0; 64];
        let mut hash_castlings = [0; 16];
        let hash_turn = rng.gen::<u64>();

        for hash_board in hash_boards.iter_mut() {
            for hash in hash_board.iter_mut() {
                *hash = rng.gen::<u64>();
            }
        }
        for hash in hash_en_passant.iter_mut() {
            *hash = rng.gen::<u64>();
        }
        for hash in hash_castlings.iter_mut() {
            *hash = rng.gen::<u64>();
        }

        Self {
            hash_boards,
            hash_en_passant,
            hash_castlings,
            hash_turn
        }
    }
}

impl Zobrist {
    pub fn cache_new(&self, board: &Board) -> u64 {
        let mut hash = 0;
        for (i, bb) in board.bbs.into_iter().enumerate().skip(P) {
            let mut mask = bb;
            while mask != 0 {
                let csq = pop_bit(&mut mask);
                hash ^= self.hash_boards[i][csq];
            }
        }
        hash ^= self.hash_castlings[board.castlings as usize];  // TODO: I don't like this cast, maybe change castlings to usize?
        hash ^= self.hash_en_passant[board.en_passant];
        if board.turn {                                         // We don't really need this if we use cache_iter() later, but...
            hash ^= self.hash_turn;
        }                            
        hash
    }

    pub fn cache_iter(&self, board: &Board, last_move: u32, prev_hash: u64) -> u64 {
        let mut hash = prev_hash;
        let from  = move_get_from(last_move);
        let to    = move_get_to(last_move);
        let piece = move_get_piece(last_move);
        let capt  = move_get_capture(last_move);
        let promo = move_get_promotion(last_move);
        if promo != E {
            hash ^= self.hash_boards[promo][to];
        } else {
            hash ^= self.hash_boards[piece][to];
        }
        if capt != E {
            if last_move & MSE_EN_PASSANT != 0 {
                hash ^= self.hash_boards[capt][to + !board.turn as usize * 16 - 8];
            } else {
                hash ^= self.hash_boards[capt][to];
            }
        } else if last_move & MSE_CASTLE_SHORT != 0 {
            hash ^= self.hash_boards[R + !board.turn as usize][to   | 1];
            hash ^= self.hash_boards[R + !board.turn as usize][from | 1];
        } else if last_move & MSE_CASTLE_LONG != 0 {
            hash ^= self.hash_boards[R + !board.turn as usize][to   - 2];
            hash ^= self.hash_boards[R + !board.turn as usize][to   | 1];
        }
        hash ^= self.hash_boards[piece][from];
        hash ^= self.hash_castlings[board.castlings as usize];
        hash ^= self.hash_castlings[*board.cst_history.last().unwrap() as usize];
        hash ^= self.hash_en_passant[board.en_passant];
        hash ^= self.hash_en_passant[*board.enp_history.last().unwrap()];
        hash ^= self.hash_turn;
        hash
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zobrist_cache_iter() {
        let zob = Zobrist::default();
        let positions = [
            "r3k2r/pbpn1pbp/3pqnp1/4p3/P1pPP3/1PN2N2/1BP1QPPP/R3K2R b Kkq d3 0 12",
            "r3k2r/pb1n1pbp/4qnp1/1Pppp3/P3P3/2Np1N2/1BP1QPPP/R3K2R w Kkq c6 0 15",
            "rnbqkbnr/pppp2pp/8/4pp1Q/4PP2/8/PPPP2PP/RNB1KBNR b KQkq - 1 3",
            "rnbqkbnr/pppp2pp/8/4pp2/4PP2/8/PPPP2PP/RNBQKBNR w KQkq - 0 3"
        ];
        for pos in positions.into_iter() {
            let mut board = Board::import(pos);
            let legals = board.get_legal_moves();
            let prev_hash = zob.cache_new(&board);
            for legal in legals.into_iter() {
                board.make_move(legal);
                let hash1 = zob.cache_iter(&board, legal, prev_hash);
                let hash2 = zob.cache_new(&board);
                assert_eq!(hash1, hash2);
                assert_ne!(hash1, prev_hash);
                board.revert_move();
            }
        }
    }
}