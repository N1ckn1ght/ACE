use crate::{util::*, maps::Maps};

/* Bitboard index structure (Little-Endian):
         H  G  F  E  D  C  B  A		     A  B  C  D  E  F  G  H
    8 | 63 62 61 60 59 58 57 56	    8 | 56 57 58 59 60 61 62 63
    7 | 55 54 53 52 51 50 49 48     7 | 48 49 50 51 52 53 54 55
    6 | 47 46 45 44 43 42 41 40     6 | 40 41 42 43 44 45 46 47
    5 | 39 38 37 36 35 34 33 32     5 | 32 33 34 35 36 37 38 39
    4 | 31 30 29 28 27 26 25 24     4 | 24 25 26 27 28 29 30 31
    3 | 23 22 21 20 19 18 17 16     3 | 16 17 18 19 20 21 22 23
    2 | 15 14 13 12 11 10  9  8     2 |  8  9 10 11 12 13 14 15
    1 |  7  6  5  4  3  2  1  0     1 |  0  1  2  3  4  5  6  7
*/

pub struct Board {
    pub bbs:        [u64; 12], // bitboards
    pub turn:       bool,      // is black to move
    pub castlings:  [bool; 4], // castle rights (util.rs has const indices)
    pub en_passant: usize,     // en passant target square
    pub hmc:        u32,       // halfmove clock (which drops for every capture or pawn movement)
    pub no:         u32,       // fullmove number (which increases after every black move)

    /* accessible constants */

    pub maps:       Maps
}

impl Board {
    pub fn new() -> Self {
        Self::import("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    pub fn import(fen: &str) -> Self {
        let mut bbs = [0; 12];
        let mut turn = false;
        let mut castlings = [false; 4];
        let mut en_passant = 0;
        let mut hmc = 0;
        let mut no = 0;

        let mut parts = fen.split_whitespace();

        // chess board
        let part = parts.next().unwrap();
        let mut bit = 0;
        for char in part.chars() {
            if PIECES.contains_key(&char) {
                set_bit(&mut bbs[PIECES[&char]], 8 * (7 - bit / 8) + bit % 8);
                bit += 1;
            } else if char != '/' {
                bit += char.to_digit(10).unwrap() as usize;
            }
        }

        // turn
        let part = parts.next().unwrap();
        for char in part.chars() {
            if char == 'b' {
                turn = true;
            }
            break;
        }

        // castle rights
        let part = parts.next().unwrap();
        for char in part.chars() {
            match char {
                'K' => castlings[CSW] = true,
                'Q' => castlings[CLW] = true,
                'k' => castlings[CSB] = true,
                'q' => castlings[CLB] = true,
                '-' => (),
                _   => panic!("Failed to import from FEN")
            };
        }

        // en passant
        let part = parts.next().unwrap();
        for char in part.chars() {
            if char == '-' {
                break;
            }
            en_passant *= 8;
            if char > '9' { 
                en_passant += char as usize - 'a' as usize 
            } else { 
                en_passant += char as usize - '0' as usize 
            };
        }
        if en_passant > 63 { 
            panic!("Failed to import from FEN")
        };

        // halfmove clock
        let part = parts.next().unwrap();
        for char in part.chars() {
            hmc *= 10;
            hmc += char as u32 - '0' as u32;
        }

        // fullmove number
        let part = parts.next().unwrap();
        for char in part.chars() {
            no *= 10;
            no += char as u32 - '0' as u32;
        }

        Self { 
            bbs, 
            turn,
            castlings,
            en_passant,
            hmc,
            no,
            maps: Maps::init()
        }
    }

    // TODO (optimize): it is possible to generate leval moves using some additional variables WITHOUT making and reverting pseudo-legal moves.
    // This is proven to be slightly faster (with the exception of en passant, probably), but also depends on the code.
    pub fn get_legal_moves(&mut self) -> Vec<u32> {
        let mut moves = self.get_pseudo_legal_moves();
        // TODO
        moves
    }

    // TODO: trasnform move format (e7e8=Q) into quick readable using interface
    pub fn make_move(&mut self, mov: u32) {
        // TODO
    }

    pub fn revert_move(&mut self) {
        // TODO
    }

    pub fn get_pseudo_legal_moves(&self) -> Vec<u32> {
        let mut moves: Vec<u32> = Vec::with_capacity(64);
        let turn = self.turn as usize;
        // occupancy masks
        let ally  = self.get_occupancies( self.turn);
        let enemy = self.get_occupancies(!self.turn);
        let both  = ally | enemy;
        // king
        let sq = gtz(self.bbs[K + turn]);
        let mut mask = self.maps.attacks_king[sq] & !ally;
        while mask > 0 {
            let csq = pop_bit(&mut mask);
            moves.push(move_encode(sq, csq, K + turn, E, self.get_capture(!self.turn, csq), MSE_NOTHING));            
        }
        // king, special
        if self.castlings[CSW + turn] {
            let csq = 6 + 56 * turn;
            if get_bit(both, csq) == 0 && get_bit(both, csq - 1) == 0 {
                if !self.is_under_attack(!self.turn, csq - 1, both) {
                    moves.push(move_encode(sq, csq, K + turn, E, E, MSE_CASTLE_SHORT));
                }
            }
        }
        if self.castlings[CLW + turn] {
            let csq = 2 + 56 * turn;
            if get_bit(both, csq) == 0 && get_bit(both, csq + 1) == 0 {
                if !self.is_under_attack(!self.turn, csq + 1, both) {
                    moves.push(move_encode(sq, csq, K + turn, E, E, MSE_CASTLE_LONG));
                }
            }
        }
        // knight
        let mut knights = self.bbs[N + turn];
        while knights > 0 {
            let sq = pop_bit(&mut knights);
            let mut mask = self.maps.attacks_knight[sq] & !ally;
            while mask > 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, N + turn, E, self.get_capture(!self.turn, sq), MSE_NOTHING));
            }
        }
        // bishop
        let mut bishops = self.bbs[B + turn];
        while bishops > 0 {
            let sq = pop_bit(&mut bishops);
            let mut mask = self.get_sliding_diagonal_attacks(sq, both);
            while mask > 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, B + turn, E, self.get_capture(!self.turn, sq), MSE_NOTHING));
            }
        }
        // rook
        let mut rooks = self.bbs[R + turn];
        while rooks > 0 {
            let sq = pop_bit(&mut rooks);
            let mut mask = self.get_sliding_straight_attacks(sq, both);
            while mask > 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, R + turn, E, self.get_capture(!self.turn, sq), MSE_NOTHING));
            }
        }
        // queen
        let mut queens = self.bbs[Q + turn];
        while queens > 0 {
            let sq = pop_bit(&mut queens);
            let mut mask = self.get_sliding_diagonal_attacks(sq, both) | self.get_sliding_straight_attacks(sq, both);
            while mask > 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, Q + turn, E, self.get_capture(!self.turn, sq), MSE_NOTHING));
            }
        }
        // pawn
        let mut pawns = self.bbs[P + turn];
        while pawns > 0 {
            let sq = pop_bit(&mut pawns);
            // TODO: the most annoying piece to code it :D
        }
        moves
    }

    // atk_turn is a color of ATTACKING pieces
    // occupancies is the all pieces of every colour combined
    pub fn is_under_attack(&self, atk_turn: bool, sq: usize, occupancies: u64) -> bool {
        // kings
        if self.maps.attacks_king[sq] & self.bbs[K + atk_turn as usize] > 0 {
            return true;
        }
        // knights
        if self.maps.attacks_knight[sq] & self.bbs[N + atk_turn as usize] > 0 {
            return true;
        }
        // bishops | queens
        let diagonal_attackers = self.bbs[B + atk_turn as usize] | self.bbs[Q + atk_turn as usize];
        let diagonal_vision    = self.get_sliding_diagonal_attacks(sq, occupancies);
        if diagonal_attackers & diagonal_vision > 0 {
            return true;
        }
        // rooks | queens
        let straight_attackers = self.bbs[R + atk_turn as usize] | self.bbs[Q + atk_turn as usize];
        let straight_vision    = self.get_sliding_straight_attacks(sq, occupancies);
        if straight_attackers & straight_vision > 0 {
            return true;
        }
        // pawns
        if self.maps.attacks_pawns[sq + 64 * !atk_turn as usize] & self.bbs[P + atk_turn as usize] > 0 {
            return true;
        }
        /* additional en passant check (optional)
           this will be useless since the function is called to determine if a king (or a castle field) is in check
           feel free to uncomment if need it, but even, for example, in engine eval(), the need is to _count_ attacks
        */
        // sq == self.en_passant
        false
    }

    // Note: removing king capture check will optimize this function a bit
    // turn is a color of a captured piece
    pub fn get_capture(&self, turn: bool, sq: usize) -> usize {
        let turn = turn as usize;
        if get_bit(self.bbs[P + turn], sq) > 0 {
            return P + turn;
        }
        if get_bit(self.bbs[N + turn], sq) > 0 {
            return N + turn;
        }
        if get_bit(self.bbs[B + turn], sq) > 0 {
            return B + turn;
        }
        if get_bit(self.bbs[R + turn], sq) > 0 {
            return R + turn;
        }
        if get_bit(self.bbs[Q + turn], sq) > 0 {
            return Q + turn;
        }
        if get_bit(self.bbs[K + turn], sq) > 0 {
            return K + turn;
        }
        E
    }

    pub fn export(&self) -> String {
        let mut fen = String::new();
        let mut pieces: [usize; 64] = [E; 64];
        for (i, bb) in self.bbs.iter().enumerate() {
            let mut mask = *bb;
            while mask > 0 {
                let csq = pop_bit(&mut mask);
                pieces[8 * (7 - csq / 8) + csq % 8] = i;
            }
        }
        let mut skip = 0;
        for (i, piece) in pieces.iter().enumerate() {
            if *piece < E {
                if skip > 0 {
                    fen.push(char::from_u32(skip + '0' as u32).unwrap());
                    skip = 0;
                }
                fen.push(PIECES_REV[&(*piece as u32)]);
            } else {
                skip += 1;
            }
            if i % 8 == 7 && i < 63 {
                if skip > 0 {
                    fen.push(char::from_u32(skip + '0' as u32).unwrap());
                    skip = 0;
                }
                fen.push('/');
            }
        }
        fen.push(' ');
        if self.turn { 
            fen.push('b');
        } else {
            fen.push('w');
        }
        fen.push(' ');
        let chars = ['K', 'k', 'Q', 'q'];
        let mut pushed = false;
        for i in [0, 2, 1, 3] {
            if self.castlings[i] {
                fen.push(chars[i]);
                pushed = true;
            }
        }
        if !pushed {
            fen.push('-');
        }
        fen.push(' ');
        if self.en_passant > 0 {
            fen.push(char::from_u32(self.en_passant as u32 / 8 + 'a' as u32).unwrap());
            fen.push(char::from_u32(self.en_passant as u32 % 8 + '0' as u32).unwrap());
        } else {
            fen.push('-');
        }
        fen.push(' ');
        fen.push_str(&self.hmc.to_string());
        fen.push(' ');
        fen.push_str(&self.no.to_string());
        fen
    }

    #[inline]
    pub fn get_occupancies(&self, turn: bool) -> u64 {
        self.bbs[P + turn as usize] | self.bbs[N + turn as usize] | self.bbs[B + turn as usize] | self.bbs[R + turn as usize] | self.bbs[Q + turn as usize] | self.bbs[K + turn as usize]
    }

    #[inline]
    pub fn get_sliding_diagonal_attacks(&self, sq: usize, occupancies: u64) -> u64 {
        let mask = occupancies & self.maps.bbs_bishop[sq];
        let magic_index = mask.wrapping_mul(self.maps.magics_bishop[sq]) >> (64 - self.maps.magic_bits_bishop[sq]);
        self.maps.attacks_bishop[magic_index as usize + self.maps.ais_bishop[sq]]
    }

    #[inline]
    pub fn get_sliding_straight_attacks(&self, sq: usize, occupancies: u64) -> u64 {
        let mask = occupancies & self.maps.bbs_rook[sq];
        let magic_index = mask.wrapping_mul(self.maps.magics_rook[sq]) >> (64 - self.maps.magic_bits_rook[sq]);
        self.maps.attacks_rook[magic_index as usize + self.maps.ais_rook[sq]]
    }

    /* TODO (optional):
        - fn import_pgn
        - fn export_pgn
    */
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_import_export() {
        let mut board1 = Board::new();
        let board2 = Board::import("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert_eq!("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", board1.export());
        assert_eq!("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", board2.export());
        del_bit(&mut board1.bbs[P],   8);
        del_bit(&mut board1.bbs[P],  12);
        del_bit(&mut board1.bbs[P],  13);
        del_bit(&mut board1.bbs[P2], 55);
        del_bit(&mut board1.bbs[N2], 62);
        assert_eq!("rnbqkb1r/ppppppp1/8/8/8/8/1PPP2PP/RNBQKBNR w KQkq - 0 1", board1.export());
        let board3 = Board::import("rnbqkb1r/ppppppp1/8/8/8/8/1PPP2PP/RNBQKBNR w KQkq - 0 1");
        assert_eq!(board1.export(), board3.export());
    }

    #[test]
    fn test_board_magic() {
        let mut board = Board::new();
        set_bit(&mut board.bbs[Q], 37);
        let ally = board.get_occupancies(false);
        assert_eq!("0000000000000000000000000010000000000000000000001111111111111111", bb_to_str(ally));
        let occupancies = board.get_occupancies(true) | ally;
        assert_eq!("1111111111111111000000000010000000000000000000001111111111111111", bb_to_str(occupancies));
        let dmask = board.get_sliding_diagonal_attacks(37, occupancies);
        assert_eq!("0000000010001000010100000000000001010000100010000000010000000000", bb_to_str(dmask));
        let smask = board.get_sliding_straight_attacks(37, occupancies);
        assert_eq!("0000000000100000001000001101111100100000001000000010000000000000", bb_to_str(smask));
        assert_eq!("0000000010101000011100001101111101110000101010000000000000000000", bb_to_str((dmask | smask) & !ally));
    }
}