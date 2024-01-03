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
    pub bbs:          [u64; 12], /* bitboards (P - K2)
                                    - idea: maybe worth adding extra `E` board to remove some if's?.. */
    pub turn:         bool,      // is black to move
    pub castlings:    u8,        // castle rights (util.rs has const indices)
    pub en_passant:   usize,     /* en passant target square
                                    - warning: it will be 0 in case if there's none, even though 0 is a valid square itself */
    pub hmc:          u32,       // halfmove clock (which drops for every capture or pawn movement)
    pub no:           u32,       // fullmove number (which increases after each black move)
    /* accessible constants */
    pub maps:         Maps,
    /* takeback funcitonal */
    pub move_history: Vec<u64>,
    pub hmc_history:  Vec<u32>,
    pub enp_history:  Vec<usize>,
    pub cst_history:  Vec<u8>
}

impl Board {
    pub fn new() -> Self {
        Self::import("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    /* TODO (optimize): it is possible to generate leval moves using some extra bitboards WITHOUT making and reverting pseudo-legal moves.
       This is proven to be slightly faster (with the exception of en passant, probably), but also depends on the code. */
    pub fn get_legal_moves(&mut self) -> Vec<u64> {
        let mut moves = self.get_pseudo_legal_moves();
        let mut i = 0;
        let mut len = moves.len();
        while i < len {
            self.make_move(moves[i]);
            let csq = gtz(self.bbs[K + !self.turn as usize]);
            let ally = self.get_occupancies(!self.turn);
            if self.is_under_attack(self.turn, csq, ally | self.get_occupancies(self.turn), ally) {
                moves.swap(i, len - 1);
                moves.pop();
                len -= 1;
            } else {
                i += 1;
            }
            self.revert_move();
        }
        moves
    }

    pub fn make_move(&mut self, mov: u64) {
        self.move_history.push(mov);
        self.hmc_history.push(self.hmc);
        self.enp_history.push(self.en_passant);
        self.cst_history.push(self.castlings);
        self.en_passant = 0;
        self.hmc += 1;
        self.no += self.turn as u32;

        let turn  = self.turn as usize;
        let from  = move_get_from(mov);
        let to    = move_get_to(mov);
        let piece = move_get_piece(mov);

        del_bit(&mut self.bbs[piece], from);
        if move_get_promotion(mov) < E {
            set_bit(&mut self.bbs[move_get_promotion(mov)], to);
        } else {
            set_bit(&mut self.bbs[piece], to);
            if mov & MSE_EN_PASSANT > 0 {
                if self.turn {
                    del_bit(&mut self.bbs[P ], to + 8);
                } else {
                    del_bit(&mut self.bbs[P2], to - 8);
                }
                self.hmc = 0;
            } else if move_get_capture(mov) < E {
                del_bit(&mut self.bbs[move_get_capture(mov)], to);
                self.hmc = 0;
            } else if piece == K | turn {
                if mov & MSE_CASTLE_SHORT > 0 {
                    del_bit(&mut self.bbs[R | turn], to   | 1);
                    set_bit(&mut self.bbs[R | turn], from | 1);
                } else if mov & MSE_CASTLE_LONG > 0 {
                    del_bit(&mut self.bbs[R | turn], to   - 2);
                    set_bit(&mut self.bbs[R | turn], to   | 1);
                }
                self.castlings &= !(CSW << turn);
                self.castlings &= !(CLW << turn);
            } else if piece == R | turn {
                match from {
                    0  => self.castlings &= !CLW,
                    7  => self.castlings &= !CSW,
                    56 => self.castlings &= !CLB,
                    63 => self.castlings &= !CSB,
                    _ => ()
                }
            } else if piece == P | turn {
                self.hmc = 0;
                if mov & MSE_DOUBLE_PAWN > 0 {
                    if self.turn {
                        if RANK_5 & (get_bit(self.bbs[P ], to - 1) | get_bit(self.bbs[P ], to + 1)) > 0 {
                            self.en_passant = to + 8;
                        }
                    } else  if RANK_4 & (get_bit(self.bbs[P2], to - 1) | get_bit(self.bbs[P2], to + 1)) > 0 {
                        self.en_passant = from + 8;
                    }
                }
            }
        }
        self.turn = !self.turn;
    }

    pub fn revert_move(&mut self) {
        let mov = self.move_history.pop().unwrap();
        self.en_passant = self.enp_history.pop().unwrap();
        self.hmc        = self.hmc_history.pop().unwrap();
        self.castlings  = self.cst_history.pop().unwrap();
        self.turn = !self.turn;                            // the previous move was for previous colour
        self.no -= self.turn as u32;

        let from  = move_get_from(mov);
        let to    = move_get_to(mov);
        let piece = move_get_piece(mov);

        set_bit(&mut self.bbs[piece], from);
        if move_get_promotion(mov) < E {
            del_bit(&mut self.bbs[move_get_promotion(mov)], to);
        } else {
            del_bit(&mut self.bbs[move_get_piece(mov)], to);
        }
        if move_get_capture(mov) < E {
            if mov & MSE_EN_PASSANT > 0 {
                set_bit(&mut self.bbs[move_get_capture(mov)], to + self.turn as usize * 16 - 8);
            } else {
                set_bit(&mut self.bbs[move_get_capture(mov)], to);
            }
        } else if mov & MSE_CASTLE_SHORT > 0 {
            del_bit(&mut self.bbs[R + self.turn as usize], from | 1);
            set_bit(&mut self.bbs[R + self.turn as usize], to   | 1);
        } else if mov & MSE_CASTLE_LONG > 0 {
            del_bit(&mut self.bbs[R + self.turn as usize], to   | 1);
            set_bit(&mut self.bbs[R + self.turn as usize], to   - 2);
        }
    }

    pub fn get_pseudo_legal_moves(&self) -> Vec<u64> {
        let mut moves: Vec<u64  > = Vec::with_capacity(64);
        let turn = self.turn as usize;
        // occupancy masks
        let ally  = self.get_occupancies( self.turn);
        let enemy = self.get_occupancies(!self.turn);
        let both  = ally | enemy;
        // king
        let sq = gtz(self.bbs[K | turn]);
        let mut mask = self.maps.attacks_king[sq] & !ally;
        while mask > 0 {
            let csq = pop_bit(&mut mask);
            moves.push(move_encode(sq, csq, K | turn, self.get_capture(!self.turn, csq), E, MSE_NOTHING));            
        }
        // king, special
        if self.castlings & (CSW << turn) > 0 {
            let csq = 6 + 56 * turn;
            if get_bit(both, csq) == 0 && get_bit(both, csq - 1) == 0 && !self.is_under_attack(!self.turn, csq - 1, both, ally) && !self.is_under_attack(!self.turn, sq, both, ally) {
                moves.push(move_encode(sq, csq, K | turn, E, E, MSE_CASTLE_SHORT));
            }
        }
        if self.castlings & (CLW << turn) > 0 {
            let csq = 2 + 56 * turn;
            if get_bit(both, csq) == 0 && get_bit(both, csq | 1) == 0 && !self.is_under_attack(!self.turn, csq | 1, both, ally) && !self.is_under_attack(!self.turn, sq, both, ally) {
                moves.push(move_encode(sq, csq, K | turn, E, E, MSE_CASTLE_LONG));
            }
        }
        // knight
        let mut knights = self.bbs[N | turn];
        while knights > 0 {
            let sq = pop_bit(&mut knights);
            let mut mask = self.maps.attacks_knight[sq] & !ally;
            while mask > 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, N | turn, self.get_capture(!self.turn, csq), E, MSE_NOTHING));
            }
        }
        // bishop
        let mut bishops = self.bbs[B | turn];
        while bishops > 0 {
            let sq = pop_bit(&mut bishops);
            let mut mask = self.get_sliding_diagonal_attacks(sq, both, ally);
            while mask > 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, B | turn, self.get_capture(!self.turn, csq), E, MSE_NOTHING));
            }
        }
        // rook
        let mut rooks = self.bbs[R | turn];
        while rooks > 0 {
            let sq = pop_bit(&mut rooks);
            let mut mask = self.get_sliding_straight_attacks(sq, both, ally);
            while mask > 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, R | turn, self.get_capture(!self.turn, csq), E, MSE_NOTHING));
            }
        }
        // queen
        let mut queens = self.bbs[Q | turn];
        while queens > 0 {
            let sq = pop_bit(&mut queens);
            let mut mask = self.get_sliding_diagonal_attacks(sq, both, ally) | self.get_sliding_straight_attacks(sq, both, ally);
            while mask > 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, Q | turn, self.get_capture(!self.turn, csq), E, MSE_NOTHING));
            }
        }
        // pawn (a hardcoded if-else)
        let mut pawns = self.bbs[P | turn];
        if self.turn {
            // black
            while pawns > 0 {
                let sq = pop_bit(&mut pawns);
                let mut mask = self.maps.attacks_pawns[sq + 64] & enemy;
                if get_bit(RANK_2, sq) > 0 {
                    // promotion
                    while mask > 0 {
                        let csq = pop_bit(&mut mask);
                        moves.push(move_encode(sq, csq, P2, self.get_capture(false, csq), Q2, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P2, self.get_capture(false, csq), R2, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P2, self.get_capture(false, csq), B2, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P2, self.get_capture(false, csq), N2, MSE_NOTHING));
                    }
                    let csq = sq - 8;
                    if get_bit(both, csq) == 0 {
                        moves.push(move_encode(sq, csq, P2, E, Q2, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P2, E, R2, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P2, E, B2, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P2, E, N2, MSE_NOTHING));
                    }
                } else {
                    while mask > 0 {
                        let csq = pop_bit(&mut mask);
                        moves.push(move_encode(sq, csq, P2, self.get_capture(false, csq), E, MSE_NOTHING));
                    }
                    let csq = sq - 8;
                    if get_bit(both, csq) == 0 {
                        moves.push(move_encode(sq, csq, P2, E, E, MSE_NOTHING));
                        // double pawn move
                        if get_bit(RANK_7, sq) > 0 && get_bit(both, csq - 8) == 0 {
                            moves.push(move_encode(sq, csq - 8, P2, E, E, MSE_DOUBLE_PAWN));
                        }
                    }
                }
            }
            // en passant
            if self.en_passant > 0 {
                if get_bit(self.bbs[P2], self.en_passant + 7) & RANK_4 > 0 {
                    moves.push(move_encode(self.en_passant + 7, self.en_passant, P2, P, E, MSE_EN_PASSANT));
                }
                if get_bit(self.bbs[P2], self.en_passant + 9) & RANK_4 > 0 {
                    moves.push(move_encode(self.en_passant + 9, self.en_passant, P2, P, E, MSE_EN_PASSANT));
                }
            }
        } else {
            // white
            while pawns > 0 {
                let sq = pop_bit(&mut pawns);
                let mut mask = self.maps.attacks_pawns[sq] & enemy;
                if get_bit(RANK_7, sq) > 0 {
                    // promotion
                    while mask > 0 {
                        let csq = pop_bit(&mut mask);
                        moves.push(move_encode(sq, csq, P, self.get_capture(true, csq), Q, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P, self.get_capture(true, csq), R, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P, self.get_capture(true, csq), B, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P, self.get_capture(true, csq), N, MSE_NOTHING));
                    }
                    let csq = sq + 8;
                    if get_bit(both, csq) == 0 {
                        moves.push(move_encode(sq, csq, P, E, Q, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P, E, R, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P, E, B, MSE_NOTHING));
                        moves.push(move_encode(sq, csq, P, E, N, MSE_NOTHING));
                    }
                } else {
                    while mask > 0 {
                        let csq = pop_bit(&mut mask);
                        moves.push(move_encode(sq, csq, P, self.get_capture(true, csq), E, MSE_NOTHING));
                    }
                    let csq = sq + 8;
                    if get_bit(both, csq) == 0 {
                        moves.push(move_encode(sq, csq, P, E, E, MSE_NOTHING));
                        // double pawn move
                        if get_bit(RANK_2, sq) > 0 && get_bit(both, csq + 8) == 0 {
                            moves.push(move_encode(sq, csq + 8, P, E, E, MSE_DOUBLE_PAWN));
                        }
                    }
                }
            }
        }
        // en passant
        if self.en_passant > 0 {
            if get_bit(self.bbs[P], self.en_passant - 7) & RANK_5 > 0 {
                moves.push(move_encode(self.en_passant - 7, self.en_passant, P, P2, E, MSE_EN_PASSANT));
            }
            if get_bit(self.bbs[P], self.en_passant - 9) & RANK_5 > 0 {
                moves.push(move_encode(self.en_passant - 9, self.en_passant, P, P2, E, MSE_EN_PASSANT));
            }
        }
        moves
    }

    /* atk_turn is a colour of ATTACKING pieces
       occupancies is the bitboard all pieces of every colour combined
       defenders is the bitboard of pieces of the ATTACKED piece colour */
    pub fn is_under_attack(&self, atk_turn: bool, sq: usize, occupancies: u64, defenders: u64) -> bool {
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
        let diagonal_vision    = self.get_sliding_diagonal_attacks(sq, occupancies, defenders);
        if diagonal_attackers & diagonal_vision > 0 {
            return true;
        }
        // rooks | queens
        let straight_attackers = self.bbs[R + atk_turn as usize] | self.bbs[Q + atk_turn as usize];
        let straight_vision    = self.get_sliding_straight_attacks(sq, occupancies, defenders);
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
        // self.en_passant != 0 && self.en_passant == sq
        false
    }

    /* Note: king capture is not included
       turn is a color of a captured piece */
    pub fn get_capture(&self, turn: bool, sq: usize) -> usize {
        let turn = turn as usize;
        if get_bit(self.bbs[P | turn], sq) > 0 {
            return P | turn;
        }
        if get_bit(self.bbs[N | turn], sq) > 0 {
            return N | turn;
        }
        if get_bit(self.bbs[B | turn], sq) > 0 {
            return B | turn;
        }
        if get_bit(self.bbs[R | turn], sq) > 0 {
            return R | turn;
        }
        if get_bit(self.bbs[Q | turn], sq) > 0 {
            return Q | turn;
        }
        E
    }

    pub fn import(fen: &str) -> Self {
        let mut bbs = [0; 12];
        let mut turn = false;
        let mut castlings = 0;
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
        if part.starts_with('b') {
            turn = true;
        }

        // castle rights
        let part = parts.next().unwrap();
        for char in part.chars() {
            match char {
                'K' => castlings |= CSW,
                'Q' => castlings |= CLW,
                'k' => castlings |= CSB,
                'q' => castlings |= CLB,
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
            if char > '9' { 
                en_passant += char as usize - 'a' as usize;
            } else { 
                en_passant += (char as usize - '0' as usize) * 8 - 8;
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
            maps:         Maps::init(),
            move_history: Vec::with_capacity(300),
            hmc_history:  Vec::with_capacity(300),
            enp_history:  Vec::with_capacity(300),
            cst_history:  Vec::with_capacity(300)
        }
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
        if skip > 0 {
            fen.push(char::from_u32(skip + '0' as u32).unwrap());
        }
        fen.push(' ');
        if self.turn { 
            fen.push('b');
        } else {
            fen.push('w');
        }
        fen.push(' ');
        let chars   = ['K', 'Q', 'k', 'q'];
        let castles = [CSW, CLW, CSB, CLB];
        let mut pushed = false;
        for (i, castle) in castles.iter().enumerate() {
            if self.castlings & castle > 0 {
                fen.push(chars[i]);
                pushed = true;
            }
        }
        if !pushed {
            fen.push('-');
        }
        fen.push(' ');
        if self.en_passant > 0 {
            fen.push(char::from_u32(self.en_passant as u32 / 8 + 'b' as u32).unwrap());
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
        self.bbs[P | turn as usize] | self.bbs[N | turn as usize] | self.bbs[B | turn as usize] | self.bbs[R | turn as usize] | self.bbs[Q | turn as usize] | self.bbs[K | turn as usize]
    }

    // ally in this context are pieces of the same colour as attacker
    #[inline]
    pub fn get_sliding_diagonal_attacks(&self, sq: usize, occupancies: u64, ally: u64) -> u64 {
        let mask = occupancies & self.maps.bbs_bishop[sq];
        let magic_index = mask.wrapping_mul(self.maps.magics_bishop[sq]) >> (64 - self.maps.magic_bits_bishop[sq]);
        self.maps.attacks_bishop[magic_index as usize + self.maps.ais_bishop[sq]] & !ally
    }

    #[inline]
    pub fn get_sliding_straight_attacks(&self, sq: usize, occupancies: u64, ally: u64) -> u64 {
        let mask = occupancies & self.maps.bbs_rook[sq];
        let magic_index = mask.wrapping_mul(self.maps.magics_rook[sq]) >> (64 - self.maps.magic_bits_rook[sq]);
        self.maps.attacks_rook[magic_index as usize + self.maps.ais_rook[sq]] & !ally
    }

    /* TODO (optional):
        - fn import_pgn
        - fn export_pgn
    */

    /* Debug */

    pub fn perft(&mut self, depth: usize) -> u64 {
        let moves = self.get_legal_moves();
        if depth == 1 {
            return moves.len() as u64;
        }
        let mut count = 0;
        for mov in moves.iter() {
            self.make_move(*mov);
            count += self.perft(depth - 1);
            self.revert_move();
        }
        count
    }
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
        let dmask = board.get_sliding_diagonal_attacks(37, occupancies, ally);
        let smask = board.get_sliding_straight_attacks(37, occupancies, ally);
        assert_eq!("0000000010101000011100001101111101110000101010000000000000000000", bb_to_str(dmask | smask));
    }
    
    // it also tests make/revert move because of get_legal_move() realization (I AM lazy)
    #[test]
    fn test_board_legal_moves() {
        assert_eq!(Board::new().get_legal_moves().len(), 20);
        assert_eq!(Board::import("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1").get_legal_moves().len(), 26);
        assert_eq!(Board::import("1rbq1r1k/p1ppB1pp/2p5/8/2BPp1n1/2N4N/P1P1Q1PP/R3K2R b KQ d3 0 15").get_legal_moves().len(), 38);
        assert_eq!(Board::import("1rbq1r1k/p1ppB1pp/2p5/8/2B3n1/2Np3N/P1P1Q1PP/R3K2R w KQ - 0 16").get_legal_moves().len(), 50);
        assert_eq!(Board::import("nnn5/1P4P1/8/8/7k/8/8/4K3 w - - 0 1").get_legal_moves().len(), 17);
        assert_eq!(Board::import("k1K5/p7/1p6/2p5/8/7p/4p1p1/3QQQ2 b - - 0 1").get_legal_moves().len(), 21);
        assert_eq!(Board::import("rnbqkbnr/pppppp1p/8/8/6pP/8/PPPPPPP1/RNBQKBNR b KQkq h3 0 3").get_legal_moves().len(), 22);
        assert_eq!(Board::import("rnbqkbnr/pppppp1p/8/8/6pP/8/PPPPPPP1/RNBQKBNR b KQkq h3 0 3").get_legal_moves().len(), 22);
        assert_eq!(Board::import("rnbqkbnr/pppppp1p/8/8/6pP/8/PPPPPPP1/RNBQKBNR b KQkq h3 0 3").get_legal_moves().len(), 22);
        assert_eq!(Board::import("rnbqkbnr/pppp1p1p/8/8/4pPp1/8/PPPPP1PP/RNBQKBNR b KQkq f3 0 5").get_legal_moves().len(), 33);
        assert_eq!(Board::import("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3").get_legal_moves().len(), 0);
        assert_eq!(Board::import("rn1q1b1r/ppp1kBpp/3p1n2/3NN3/4P3/8/PPPP1PPP/R1BbK2R b KQ - 2 7").get_legal_moves().len(), 1);
        assert_eq!(Board::import("rn1q1bnr/ppp1kBpp/3p1p2/3NN3/4P3/8/PPPP1PPP/R1BbK2R b KQ - 3 7").get_legal_moves().len(), 0);
        /* Test positions from https://gist.github.com/peterellisjones/8c46c28141c162d1d8a0f0badbc9cff9 */
        assert_eq!(Board::import("r6r/1b2k1bq/8/8/7B/8/8/R3K2R b KQ - 3 2").get_legal_moves().len(), 8);
        assert_eq!(Board::import("8/8/8/2k5/2pP4/8/B7/4K3 b - d3 0 3").get_legal_moves().len(), 8);
        assert_eq!(Board::import("r1bqkbnr/pppppppp/n7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 2 2").get_legal_moves().len(), 19);
        assert_eq!(Board::import("r3k2r/p1pp1pb1/bn2Qnp1/2qPN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQkq - 3 2").get_legal_moves().len(), 5);
        assert_eq!(Board::import("2kr3r/p1ppqpb1/bn2Qnp1/3PN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQ - 3 2").get_legal_moves().len(), 44);
        assert_eq!(Board::import("rnb2k1r/pp1Pbppp/2p5/q7/2B5/8/PPPQNnPP/RNB1K2R w KQ - 3 9").get_legal_moves().len(), 39);
        assert_eq!(Board::import("2r5/3pk3/8/2P5/8/2K5/8/8 w - - 5 4").get_legal_moves().len(), 9);
    }

    // it will break on move encoding change
    #[test]
    fn test_board_legal_moves_specific() {
        let mut board = Board::import("rnbqkbnr/pppp1p1p/8/8/4p1p1/8/PPPPPPPP/RNBQKBNR w KQkq - 0 5");
        assert_eq!(board.get_legal_moves().len(), 18);
        board.make_move(54762736897);
        assert_eq!(board.get_legal_moves().len(), 33);
        board.make_move(54779917057);
        assert_eq!(board.get_legal_moves().len(), 18);
        board.make_move(54762605313);
        assert_eq!(board.get_legal_moves().len(), 37);
        board.make_move(54847552768);
        assert_eq!(board.get_legal_moves().len(), 22);
        board.make_move(54795765248);
        assert_eq!(board.get_legal_moves().len(), 38);
        board.make_move(54814588416);
        assert_eq!(board.get_legal_moves().len(), 26);
        board.make_move(54829253120);
        assert_eq!(board.get_legal_moves().len(), 34);
        board.make_move(54813931776);
        assert_eq!(board.get_legal_moves().len(), 28);
        assert_eq!(board.export(), "r1bqk2r/ppp1np1p/2nb4/3p4/3PpPp1/4BN2/PPP1P1PP/RN1QKB1R w KQkq - 5 9");
        board.make_move(54795436288);
        assert_eq!(board.get_legal_moves().len(), 34);
        board.make_move(54847158784);
        assert_eq!(board.get_legal_moves().len(), 28);
        board.make_move(54796424448);
        assert_eq!(board.get_legal_moves().len(), 35);
        board.make_move(54915185408);
        assert_eq!(board.get_legal_moves().len(), 25);
        board.make_move(54762278400);
        assert_eq!(board.get_legal_moves().len(), 35);
        board.make_move(54949198850);
        assert_eq!(board.get_legal_moves().len(), 27);
        board.make_move(54828860672);
        assert_eq!(board.get_legal_moves().len(), 30);
        board.make_move(54779785473);
        assert_eq!(board.get_legal_moves().len(), 31);
        board.make_move(54896296704);
        assert_eq!(board.get_legal_moves().len(), 31);
        board.make_move(37598993408);
        assert_eq!(board.get_legal_moves().len(), 33);
        assert_eq!(board.export(), "2kr3r/p1pqnp1p/2nb4/1p1p1b2/3P1PpN/N2pB1P1/PPP1P1BP/R3K2R w KQ - 0 14");
    }

    #[test]
    fn test_board_import_export_advanced() {
        let mut board = Board::new();
        _ = board.get_legal_moves();
        assert_eq!("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", board.export());
        let mut board = Board::import("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
        _ = board.get_legal_moves();
        assert_eq!("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1", board.export());
        let mut board = Board::import("1rbq1r1k/p1ppB1pp/2p5/8/2BPp1n1/2N4N/P1P1Q1PP/R3K2R b KQ d3 0 15");
        _ = board.get_legal_moves();
        assert_eq!("1rbq1r1k/p1ppB1pp/2p5/8/2BPp1n1/2N4N/P1P1Q1PP/R3K2R b KQ d3 0 15", board.export());
        let mut board = Board::import("1rbq1r1k/p1ppB1pp/2p5/8/2B3n1/2Np3N/P1P1Q1PP/R3K2R w KQ - 0 16");
        _ = board.get_legal_moves();
        assert_eq!("1rbq1r1k/p1ppB1pp/2p5/8/2B3n1/2Np3N/P1P1Q1PP/R3K2R w KQ - 0 16", board.export());
        let mut board = Board::import("r3k2r/p1pp1pb1/bn2Qnp1/2qPN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQkq - 3 2");
        _ = board.get_legal_moves();
        assert_eq!("r3k2r/p1pp1pb1/bn2Qnp1/2qPN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQkq - 3 2", board.export());
        let mut board = Board::import("4k3/8/8/8/8/8/Q7/4K3 w - - 0 1");
        _ = board.get_legal_moves();
        assert_eq!("4k3/8/8/8/8/8/Q7/4K3 w - - 0 1", board.export());
    }

    #[test]
    fn test_board_legal_moves_advanced() {
        let mut board = Board::new();
        assert_eq!(board.perft(1), 20);
        assert_eq!(board.perft(2), 400);
        assert_eq!(board.perft(3), 8902);
        assert_eq!(board.perft(4), 197281);
        /* Positions from https://www.chessprogramming.org/Perft_Results */
        let mut board = Board::import("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        assert_eq!(board.perft(1), 48);
        assert_eq!(board.perft(2), 2039);
        assert_eq!(board.perft(3), 97862);
        assert_eq!(board.perft(4), 4085603);
        let mut board = Board::import("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
        assert_eq!(board.perft(1), 14);
        assert_eq!(board.perft(2), 191);
        assert_eq!(board.perft(3), 2812);
        assert_eq!(board.perft(4), 43238);
        let mut board = Board::import("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
        assert_eq!(board.perft(1), 6);
        assert_eq!(board.perft(2), 264);
        assert_eq!(board.perft(3), 9467);
        assert_eq!(board.perft(4), 422333);
        let mut board = Board::import("r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1");
        assert_eq!(board.perft(1), 6);
        assert_eq!(board.perft(2), 264);
        assert_eq!(board.perft(3), 9467);
        assert_eq!(board.perft(4), 422333);
    }
}