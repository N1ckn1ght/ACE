use crate::frame::{util::*, maps::Maps};

/* Bitboard index structure (Little-Endian):
         H  G  F  E  D  C  B  A          A  B  C  D  E  F  G  H
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
    pub bbs:          [u64; 12],    /* bitboards (P - K2)
                                        - idea: maybe worth adding extra `E` board to remove some if's?.. */
    pub turn:         bool,         // is black to move
    pub castlings:    u8,           // castle rights (util.rs has const indices)
    pub en_passant:   usize,        /* en passant target square
                                        - warning: it will be 0 in case if there's none, even though 0 is a valid square itself */
    pub hmc:          u16,          // halfmove clock (which drops for every capture or pawn movement)
    pub no:           i16,          /* halfmove number
                                        it should act as a fullmove number in import/export (which increases after each black move) */
    /* Accessible constants */
    pub maps:         Maps,
    /* Takeback funcitonal */
    pub move_history: Vec<u64>,
    pub hmc_history:  Vec<u16>,
    pub enp_history:  Vec<usize>,
    pub cst_history:  Vec<u8>
}

impl Default for Board {
    fn default() -> Board {
        Board::import("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }
}

impl Board {
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
                set_bit(&mut bbs[PIECES[&char]], flip(bit));
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
            hmc += char as u16 - '0' as u16;
        }

        // fullmove number
        let part = parts.next().unwrap();
        for char in part.chars() {
            no *= 10;
            no += char as i16 - '0' as i16;
        }

        // fullmove to halfmove
        no = (no - 1) * 2 + turn as i16;

        Self { 
            bbs, 
            turn,
            castlings,
            en_passant,
            hmc,
            no,
            maps:         Maps::default(),
            move_history: Vec::with_capacity(300),
            hmc_history:  Vec::with_capacity(300),
            enp_history:  Vec::with_capacity(300),
            cst_history:  Vec::with_capacity(300)
        }
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
        self.no += 1;

        let turn  = self.turn as usize;
        let from  = move_get_from(mov);
        let to    = move_get_to(mov);
        let piece = move_get_piece(mov);
        let capt  = move_get_capture(mov);

        del_bit(&mut self.bbs[piece], from);
        if capt < E {
            del_bit(&mut self.bbs[capt], to);
            self.hmc = 0;
            if capt | 1 == R | R2 {
                match to {
                    0  => self.castlings &= !CLW,
                    7  => self.castlings &= !CSW,
                    56 => self.castlings &= !CLB,
                    63 => self.castlings &= !CSB,
                    _ => ()
                }
            }
        }
        if move_get_promotion(mov) < E {
            set_bit(&mut self.bbs[move_get_promotion(mov)], to);
        } else {
            set_bit(&mut self.bbs[piece], to);
            if mov & MSE_EN_PASSANT != 0 {
                if self.turn {
                    del_bit(&mut self.bbs[P ], to + 8);
                } else {
                    del_bit(&mut self.bbs[P2], to - 8);
                }
                self.hmc = 0;
            } else if piece == K | turn {
                if mov & MSE_CASTLE_SHORT != 0 {
                    del_bit(&mut self.bbs[R | turn], to   | 1);
                    set_bit(&mut self.bbs[R | turn], from | 1);
                } else if mov & MSE_CASTLE_LONG != 0 {
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
                if mov & MSE_DOUBLE_PAWN != 0 {
                    if self.turn {
                        if RANK_5 & (get_bit(self.bbs[P ], to - 1) | get_bit(self.bbs[P ], to + 1)) != 0 {
                            self.en_passant = to + 8;
                        }
                    } else if RANK_4 & (get_bit(self.bbs[P2], to - 1) | get_bit(self.bbs[P2], to + 1)) != 0 {
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
        self.no -= 1;

        let from  = move_get_from(mov);
        let to    = move_get_to(mov);
        let piece = move_get_piece(mov);

        set_bit(&mut self.bbs[piece], from);
        if move_get_promotion(mov) < E {
            del_bit(&mut self.bbs[move_get_promotion(mov)], to);
        } else {
            del_bit(&mut self.bbs[piece], to);
        }
        if move_get_capture(mov) < E {
            if mov & MSE_EN_PASSANT != 0 {
                set_bit(&mut self.bbs[move_get_capture(mov)], to + self.turn as usize * 16 - 8);
            } else {
                set_bit(&mut self.bbs[move_get_capture(mov)], to);
            }
        } else if mov & MSE_CASTLE_SHORT != 0 {
            del_bit(&mut self.bbs[R + self.turn as usize], from | 1);
            set_bit(&mut self.bbs[R + self.turn as usize], to   | 1);
        } else if mov & MSE_CASTLE_LONG != 0 {
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
        while mask != 0 {
            let csq = pop_bit(&mut mask);
            moves.push(move_encode(sq, csq, K | turn, self.get_capture(!self.turn, csq), E, MSE_NOTHING));            
        }
        // king, special
        if self.castlings & (CSW << turn) != 0 {
            let csq = 6 + 56 * turn;
            if CSMASK << (56 * turn) & both == 0 && !self.is_under_attack(!self.turn, csq - 1, both, ally) && !self.is_under_attack(!self.turn, sq, both, ally) {
                moves.push(move_encode(sq, csq, K | turn, E, E, MSE_CASTLE_SHORT));
            }
        }
        if self.castlings & (CLW << turn) != 0 {
            let csq = 2 + 56 * turn;
            if CLMASK << (56 * turn) & both == 0 && !self.is_under_attack(!self.turn, csq | 1, both, ally) && !self.is_under_attack(!self.turn, sq, both, ally) {
                moves.push(move_encode(sq, csq, K | turn, E, E, MSE_CASTLE_LONG));
            }
        }
        // knight
        let mut knights = self.bbs[N | turn];
        while knights != 0 {
            let sq = pop_bit(&mut knights);
            let mut mask = self.maps.attacks_knight[sq] & !ally;
            while mask != 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, N | turn, self.get_capture(!self.turn, csq), E, MSE_NOTHING));
            }
        }
        // bishop
        let mut bishops = self.bbs[B | turn];
        while bishops != 0 {
            let sq = pop_bit(&mut bishops);
            let mut mask = self.get_sliding_diagonal_attacks(sq, both, ally);
            while mask != 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, B | turn, self.get_capture(!self.turn, csq), E, MSE_NOTHING));
            }
        }
        // rook
        let mut rooks = self.bbs[R | turn];
        while rooks != 0 {
            let sq = pop_bit(&mut rooks);
            let mut mask = self.get_sliding_straight_attacks(sq, both, ally);
            while mask != 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, R | turn, self.get_capture(!self.turn, csq), E, MSE_NOTHING));
            }
        }
        // queen
        let mut queens = self.bbs[Q | turn];
        while queens != 0 {
            let sq = pop_bit(&mut queens);
            let mut mask = self.get_sliding_diagonal_attacks(sq, both, ally) | self.get_sliding_straight_attacks(sq, both, ally);
            while mask != 0 {
                let csq = pop_bit(&mut mask);
                moves.push(move_encode(sq, csq, Q | turn, self.get_capture(!self.turn, csq), E, MSE_NOTHING));
            }
        }
        // pawn (a hardcoded if-else)
        let mut pawns = self.bbs[P | turn];
        if self.turn {
            // black
            while pawns != 0 {
                let sq = pop_bit(&mut pawns);
                let mut mask = self.maps.attacks_pawns[1][sq] & enemy;
                if get_bit(RANK_2, sq) != 0 {
                    // promotion
                    while mask != 0 {
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
                    while mask != 0 {
                        let csq = pop_bit(&mut mask);
                        moves.push(move_encode(sq, csq, P2, self.get_capture(false, csq), E, MSE_NOTHING));
                    }
                    let csq = sq - 8;
                    if get_bit(both, csq) == 0 {
                        moves.push(move_encode(sq, csq, P2, E, E, MSE_NOTHING));
                        // double pawn move
                        if get_bit(RANK_7, sq) != 0 && get_bit(both, csq - 8) == 0 {
                            moves.push(move_encode(sq, csq - 8, P2, E, E, MSE_DOUBLE_PAWN));
                        }
                    }
                }
            }
            // en passant
            if self.en_passant != 0 {
                if get_bit(self.bbs[P2], self.en_passant + 7) & RANK_4 != 0 {
                    moves.push(move_encode(self.en_passant + 7, self.en_passant, P2, P, E, MSE_EN_PASSANT));
                }
                if get_bit(self.bbs[P2], self.en_passant + 9) & RANK_4 != 0 {
                    moves.push(move_encode(self.en_passant + 9, self.en_passant, P2, P, E, MSE_EN_PASSANT));
                }
            }
        } else {
            // white
            while pawns != 0 {
                let sq = pop_bit(&mut pawns);
                let mut mask = self.maps.attacks_pawns[0][sq] & enemy;
                if get_bit(RANK_7, sq) != 0 {
                    // promotion
                    while mask != 0 {
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
                    while mask != 0 {
                        let csq = pop_bit(&mut mask);
                        moves.push(move_encode(sq, csq, P, self.get_capture(true, csq), E, MSE_NOTHING));
                    }
                    let csq = sq + 8;
                    if get_bit(both, csq) == 0 {
                        moves.push(move_encode(sq, csq, P, E, E, MSE_NOTHING));
                        // double pawn move
                        if get_bit(RANK_2, sq) != 0 && get_bit(both, csq + 8) == 0 {
                            moves.push(move_encode(sq, csq + 8, P, E, E, MSE_DOUBLE_PAWN));
                        }
                    }
                }
            }
        }
        // en passant
        if self.en_passant != 0 {
            if get_bit(self.bbs[P], self.en_passant - 7) & RANK_5 != 0 {
                moves.push(move_encode(self.en_passant - 7, self.en_passant, P, P2, E, MSE_EN_PASSANT));
            }
            if get_bit(self.bbs[P], self.en_passant - 9) & RANK_5 != 0 {
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
        if self.maps.attacks_king[sq] & self.bbs[K + atk_turn as usize] != 0 {
            return true;
        }
        // knights
        if self.maps.attacks_knight[sq] & self.bbs[N + atk_turn as usize] != 0 {
            return true;
        }
        // bishops | queens
        let diagonal_attackers = self.bbs[B + atk_turn as usize] | self.bbs[Q + atk_turn as usize];
        let diagonal_vision    = self.get_sliding_diagonal_attacks(sq, occupancies, defenders);
        if diagonal_attackers & diagonal_vision != 0 {
            return true;
        }
        // rooks | queens
        let straight_attackers = self.bbs[R + atk_turn as usize] | self.bbs[Q + atk_turn as usize];
        let straight_vision    = self.get_sliding_straight_attacks(sq, occupancies, defenders);
        if straight_attackers & straight_vision != 0 {
            return true;
        }
        // pawns
        if self.maps.attacks_pawns[!atk_turn as usize][sq] & self.bbs[P + atk_turn as usize] != 0 {
            return true;
        }
        // This is useless since the function is called to determine whether a king (or a castle field) is in check
        // self.en_passant != 0 && self.en_passant == sq
        false
    }

    /* Note: king capture is not included
       turn is a color of a captured piece */
    pub fn get_capture(&self, turn: bool, sq: usize) -> usize {
        let turn = turn as usize;
        if get_bit(self.bbs[P | turn], sq) != 0 {
            return P | turn;
        }
        if get_bit(self.bbs[N | turn], sq) != 0 {
            return N | turn;
        }
        if get_bit(self.bbs[B | turn], sq) != 0 {
            return B | turn;
        }
        if get_bit(self.bbs[R | turn], sq) != 0 {
            return R | turn;
        }
        if get_bit(self.bbs[Q | turn], sq) != 0 {
            return Q | turn;
        }
        E
    }

    pub fn export(&self) -> String {
        let mut fen = String::new();
        let mut pieces: [usize; 64] = [E; 64];
        for (i, bb) in self.bbs.iter().enumerate() {
            let mut mask = *bb;
            while mask != 0 {
                let csq = pop_bit(&mut mask);
                pieces[flip(csq)] = i;
            }
        }
        let mut skip = 0;
        for (i, piece) in pieces.iter().enumerate() {
            if *piece < E {
                if skip != 0 {
                    fen.push(char::from_u32(skip + '0' as u32).unwrap());
                    skip = 0;
                }
                fen.push(PIECES_REV[&(*piece as u32)]);
            } else {
                skip += 1;
            }
            if i & 7 == 7 && i < 63 {
                if skip != 0 {
                    fen.push(char::from_u32(skip + '0' as u32).unwrap());
                    skip = 0;
                }
                fen.push('/');
            }
        }
        if skip != 0 {
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
            if self.castlings & castle != 0 {
                fen.push(chars[i]);
                pushed = true;
            }
        }
        if !pushed {
            fen.push('-');
        }
        fen.push(' ');
        if self.en_passant != 0 {
            fen.push(char::from_u32( self.en_passant as u32 / 8  + 'b' as u32).unwrap());
            fen.push(char::from_u32((self.en_passant as u32 & 7) + '0' as u32).unwrap());
        } else {
            fen.push('-');
        }
        fen.push(' ');
        fen.push_str(&self.hmc.to_string());
        fen.push(' ');
        // halfmove to fullmove
        let no = (self.no + 1 + (!self.turn) as i16) / 2;
        fen.push_str(&no.to_string());
        fen
    }

    #[inline]
    pub fn get_occupancies(&self, turn: bool) -> u64 {
        let turn = turn as usize;
        self.bbs[P | turn] | self.bbs[N | turn] | self.bbs[B | turn] | self.bbs[R | turn] | self.bbs[Q | turn] | self.bbs[K | turn]
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

    /* More functions for evaluation and stuff */

	pub fn get_sliding_straight_path(&self, sq1: usize, sq2: usize) -> u64 {
        if sq1 & 7 == sq2 & 7 || sq1 & 56 == sq2 & 56 {
            return self.get_sliding_straight_attacks(sq1, 1 << sq2, 0) & self.get_sliding_straight_attacks(sq2, 1 << sq1, 0);
        }
        0
	}

	pub fn get_sliding_diagonal_path(&self, sq1: usize, sq2: usize) -> u64 {
        let attack1 = self.get_sliding_diagonal_attacks(sq1, 1 << sq2, 0);
        if attack1 & (1 << sq2) != 0 {
            return attack1 & self.get_sliding_diagonal_attacks(sq2, 1 << sq1, 0);
        }
        0
	}

    #[inline]
    pub fn get_sliding_straight_path_unsafe(&self, sq1: usize, sq2: usize) -> u64 {
        self.get_sliding_straight_attacks(sq1, 1 << sq2, 0) & self.get_sliding_straight_attacks(sq2, 1 << sq1, 0)
	}

    #[inline]
    pub fn get_sliding_diagonal_path_unsafe(&self, sq1: usize, sq2: usize) -> u64 {
        self.get_sliding_diagonal_attacks(sq1, 1 << sq2, 0) & self.get_sliding_diagonal_attacks(sq2, 1 << sq1, 0)
	}

    pub fn is_in_check(&self) -> bool {
        let ally = self.get_occupancies(self.turn);
        let enemy = self.get_occupancies(!self.turn);
        self.is_under_attack(!self.turn, gtz(self.bbs[K | self.turn as usize]), ally | enemy, ally)
    }
    
    /* pub fn is_game_ended(&mut self) -> bool {
        self.get_legal_moves().len() == 0
    } */

    #[inline]
    pub fn is_passing(&self, sq: usize, enemy_pawns: u64, ally_colour: usize) -> bool {
        self.maps.piece_passing[ally_colour][sq] & enemy_pawns == 0
    }

    #[inline]
    pub fn is_protected(&self, sq: usize, ally_pawns: u64, enemy_colour: usize) -> bool {
        self.maps.attacks_pawns[enemy_colour][sq] & ally_pawns != 0
    }

    #[inline]
    pub fn is_outpost(&self, sq: usize, ally_pawns: u64, enemy_pawns: u64, colour: bool) -> bool {
        self.maps.piece_pb[colour as usize][sq] & enemy_pawns == 0 && self.is_protected(sq, ally_pawns, !colour as usize)
    }

    pub fn is_easily_protected(&self, sq: usize, ally_pawns: u64, occupancy: u64, ally_colour: usize, enemy_colour: usize) -> bool {
        // if there are existing pawns on necessary lanes at all
        if self.maps.piece_pb[enemy_colour][sq] & ally_pawns == 0 {
            return false;
        }
        // if the piece is already protected enough
        let mut mask = self.maps.attacks_pawns[enemy_colour][sq];
        if mask & ally_pawns != 0 {
            return true;
        }
        // if there's nothing standing in the way of any pawn to protect the piece
        while mask != 0 {
            let csq = pop_bit(&mut mask);
            let lane = self.maps.files[csq] & self.maps.piece_pb[enemy_colour][sq];
            let pbit = lane & ally_pawns;
            if pbit == 0 {
                continue;
            }
            // if we are white - we are interested in leading bit (it's the closest one)
            // otherwise we need trailing bit
            let path;
            if ally_colour == 0 {
                path = self.get_sliding_straight_path_unsafe(csq, glz(pbit));
            } else {
                path = self.get_sliding_straight_path_unsafe(csq, gtz(pbit));
            }
            if path & occupancy == 0 {
                return true;
            }
        }
        false
    }

    /* Debug and benchmarking */

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

    // uses standard output instead
    pub fn perft_divided(&mut self, depth: usize) {
        let moves = self.get_legal_moves();
        for mov in moves.iter() {
            self.make_move(*mov);
            println!("{}\t{}\t{}\t{}", mov, move_transform(*mov), self.perft(depth - 1), self.export());
            self.revert_move();
        }
    }

    // [moves, captures, en passants, castles, promotions]
    pub fn perft_verbosed(&mut self, depth: usize) -> [u64; 5] {
        let moves = self.get_legal_moves();
        if depth == 1 {
            let mut arr = [0, 0, 0, 0, 0];
            for mov in moves.iter() { 
                arr[0] += 1;
                if move_get_capture(*mov) < E {
                    arr[1] += 1;
                }
                if mov & MSE_EN_PASSANT != 0 {
                    arr[2] += 1;
                }
                if mov & (MSE_CASTLE_SHORT | MSE_CASTLE_LONG) != 0 {
                    arr[3] += 1;
                }
                if move_get_promotion(*mov) < E {
                    arr[4] += 1;
                }
            }
            return arr;
        }
        let moves = self.get_legal_moves();
        let mut count = [0, 0, 0, 0, 0];
        for mov in moves.iter() {
            self.make_move(*mov);
            let temp = self.perft_verbosed(depth - 1);
            for (i, elem) in count.iter_mut().enumerate() {
                *elem += temp[i];
            }
            self.revert_move();
        }
        count
    }
}


#[cfg(test)]
mod tests {
    use test::Bencher;
    use super::*;

    #[test]
    fn test_board_import_export() {
        let mut board1 = Board::default();
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
        let mut board = Board::default();
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
    fn test_board_legal_moves_1() {
        assert_eq!(Board::default().get_legal_moves().len(), 20);
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
        assert_eq!(Board::import("r3k2r/8/8/4B3/4b3/8/8/R3K2R w KQkq - 0 1").get_legal_moves().len(), 38);
        assert_eq!(Board::import("r3k2r/8/8/4B3/4b3/8/8/R3K2R b KQkq - 0 1").get_legal_moves().len(), 38);
        assert_eq!(Board::import("r3k2r/8/8/3B4/3b4/8/8/R3K2R w KQkq - 0 1").get_legal_moves().len(), 36);
        assert_eq!(Board::import("r3k2r/8/8/3B4/3b4/8/8/R3K2R b KQkq - 0 1").get_legal_moves().len(), 36);
        assert_eq!(Board::import("r3k2r/8/8/3B4/3b4/8/8/R3K2R b KQkq - 0 1").get_legal_moves().len(), 36);
        assert_eq!(Board::import("r3k2r/4r3/8/8/7Q/8/8/R3K2R w KQkq - 0 1").get_legal_moves().len(), 6);
        assert_eq!(Board::import("r3k2r/8/8/4R2q/8/8/8/R3K2R b KQkq - 0 1").get_legal_moves().len(), 5);    
        assert_eq!(Board::import("r3k2r/8/8/8/8/8/8/RB2K2R w KQkq - 0 1").get_legal_moves().len(), 29);
        assert_eq!(Board::import("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/Rn2K1nR w KQkq - 0 1").get_legal_moves().len(), 20);
        assert_eq!(Board::import("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/R1n1Kn1R w KQkq - 0 1").get_legal_moves().len(), 22);
        assert_eq!(Board::import("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/R2nK2R w KQkq - 0 1").get_legal_moves().len(), 24);
        assert_eq!(Board::import("r2Nk1Nr/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").get_legal_moves().len(), 22);
        assert_eq!(Board::import("r3k2r/pppppppp/4N3/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").get_legal_moves().len(), 21);
        assert_eq!(Board::import("r3k2r/pppppppp/2N5/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").get_legal_moves().len(), 23);
        assert_eq!(Board::import("r3k2r/pppppppp/N7/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").get_legal_moves().len(), 24);
        assert_eq!(Board::import("r3k2r/pp1p1p1p/4p3/8/4N3/B6B/PP1PP1PP/4K3 b kq - 0 1").get_legal_moves().len(), 18);
        assert_eq!(Board::import("r3k2r/pp1pQp1p/4p3/8/8/B7/PP1PP1PP/4K3 b kq - 0 1").get_legal_moves().len(), 0);
        assert_eq!(Board::import("r3k2r/7P/4P1P1/3PKP2/2P5/1P6/P7/8 b kq - 0 1").get_legal_moves().len(), 16);
    }

    #[test]
    fn test_board_legal_moves_2() {
        /* Test positions from https://gist.github.com/peterellisjones/8c46c28141c162d1d8a0f0badbc9cff9 */
        assert_eq!(Board::import("r6r/1b2k1bq/8/8/7B/8/8/R3K2R b KQ - 3 2").get_legal_moves().len(), 8);
        assert_eq!(Board::import("8/8/8/2k5/2pP4/8/B7/4K3 b - d3 0 3").get_legal_moves().len(), 8);
        assert_eq!(Board::import("r1bqkbnr/pppppppp/n7/8/8/P7/1PPPPPPP/RNBQKBNR w KQkq - 2 2").get_legal_moves().len(), 19);
        assert_eq!(Board::import("r3k2r/p1pp1pb1/bn2Qnp1/2qPN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQkq - 3 2").get_legal_moves().len(), 5);
        assert_eq!(Board::import("2kr3r/p1ppqpb1/bn2Qnp1/3PN3/1p2P3/2N5/PPPBBPPP/R3K2R b KQ - 3 2").get_legal_moves().len(), 44);
        assert_eq!(Board::import("rnb2k1r/pp1Pbppp/2p5/q7/2B5/8/PPPQNnPP/RNB1K2R w KQ - 3 9").get_legal_moves().len(), 39);
        assert_eq!(Board::import("2r5/3pk3/8/2P5/8/2K5/8/8 w - - 5 4").get_legal_moves().len(), 9);
        /* Tricky positions, some from perft suits */
        assert_eq!(Board::import("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").get_legal_moves().len(), 14);
        assert_eq!(Board::import("rnbqkbnr/ppp1pppp/8/8/8/1QNBBN2/PPPpPPPP/R3K2R w KQkq - 0 1").get_legal_moves().len(), 5);
        assert_eq!(Board::import("rnbqkbnr/ppp1pppp/8/8/8/1QNBBN2/PPP1PPPp/R3K2R w KQkq - 0 1").get_legal_moves().len(), 53);
        assert_eq!(Board::import("r3k2r/Pnp1pppp/1b3bn1/8/2Pp3q/1Q1BBN2/PP1NPPPp/R3K2R b KQkq c3 0 1").get_legal_moves().len(), 40);
        assert_eq!(Board::import("r6r/p1ppqkb1/bn2pnp1/3P4/1p2P3/2NQ3p/PPPBBPPP/R3K2R b KQ - 1 2").get_legal_moves().len(), 51);
        assert_eq!(Board::import("7b/2k5/8/8/2p5/8/6K1/8 w - - 2 4").get_legal_moves().len(), 8);
    }

    // it will break on move encoding change
    #[test]
    fn test_board_legal_moves_specific_1() {
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
    fn test_board_legal_moves_specific_2() {
        let mut board = Board::import("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1R1K b kq - 1 1");
        assert_eq!(board.get_legal_moves().len(), 46);
        board.make_move(27128760576);
        assert_eq!(board.get_legal_moves().len(), 37);
        assert_eq!(board.export(), "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/P2P2PP/b2Q1R1K w kq - 0 2");
    }

    #[test]
    fn test_board_legal_moves_specific_3() {
        let mut board = Board::import("r3k2r/p1p1qNb1/bn1ppnp1/3P4/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 2");
        board.make_move(33323693312);
        assert_eq!(board.get_legal_moves().len(), 40);
        assert_eq!(board.export(), "r3k2N/p1p1q1b1/bn1ppnp1/3P4/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQq - 0 2");
    }

    #[test]
    fn test_board_import_export_advanced() {
        let mut board = Board::default();
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
    fn test_board_legal_moves_advanced_1() {
        let mut board = Board::default();
        assert_eq!(board.perft_verbosed(1), [20, 0, 0, 0, 0]);
        assert_eq!(board.perft_verbosed(2), [400, 0, 0, 0, 0]);
        assert_eq!(board.perft_verbosed(3), [8902, 34, 0, 0, 0]);
        assert_eq!(board.perft_verbosed(4), [197281, 1576, 0, 0, 0]);
    }

    #[test]
    fn test_board_legal_moves_advanced_2() {
        // https://www.chessprogramming.org/Perft_Results, Position 3
        let mut board = Board::import("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
        assert_eq!(board.perft_verbosed(1), [14, 1, 0, 0, 0]);
        assert_eq!(board.perft_verbosed(2), [191, 14, 0, 0, 0]);
        assert_eq!(board.perft_verbosed(3), [2812, 209, 2, 0, 0]);
        assert_eq!(board.perft_verbosed(4), [43238, 3348, 123, 0, 0]);
    }

    #[test]
    fn test_board_legal_moves_advanced_3() {
        // https://www.chessprogramming.org/Perft_Results, Position 4
        let mut board = Board::import("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
        assert_eq!(board.perft_verbosed(1), [6, 0, 0, 0, 0]);
        assert_eq!(board.perft_verbosed(2), [264, 87, 0, 6, 48]);
        assert_eq!(board.perft_verbosed(3), [9467, 1021, 4, 0, 120]);
        assert_eq!(board.perft_verbosed(4), [422333, 131393, 0, 7795, 60032]);
    }

    #[test]
    fn test_board_legal_moves_advanced_4() {
        // https://www.chessprogramming.org/Perft_Results, Position 4, Mirrored
        let mut board = Board::import("r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1");
        assert_eq!(board.perft_verbosed(1), [6, 0, 0, 0, 0]);
        assert_eq!(board.perft_verbosed(2), [264, 87, 0, 6, 48]);
        assert_eq!(board.perft_verbosed(3), [9467, 1021, 4, 0, 120]);
        assert_eq!(board.perft_verbosed(4), [422333, 131393, 0, 7795, 60032]);
    }

    #[test]
    fn test_board_legal_moves_advanced_5() {
        // https://www.chessprogramming.org/Perft_Results, Kiwipete (the best)
        let mut board = Board::import("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        assert_eq!(board.perft_verbosed(1), [48, 8, 0, 2, 0]);
        assert_eq!(board.perft_verbosed(2), [2039, 351, 1, 91, 0]);
        assert_eq!(board.perft_verbosed(3), [97862, 17102, 45, 3162, 0]);
        assert_eq!(board.perft_verbosed(4), [4085603, 757163, 1929, 128013, 15172]);
    }

    #[test]
    fn test_board_legal_moves_advanced_6() {
        // http://www.talkchess.com/forum3/viewtopic.php?t=42463
        let mut board = Board::import("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8");
        assert_eq!(board.perft(1), 44);
        assert_eq!(board.perft(2), 1486);
        assert_eq!(board.perft(3), 62379);
        assert_eq!(board.perft(4), 2103487);
    }

    #[test]
    fn test_board_legal_moves_advanced_7() {
        // https://www.chessprogramming.org/Perft_Results, Position 6
        let mut board = Board::import("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10");
        assert_eq!(board.perft(1), 46);
        assert_eq!(board.perft(2), 2079);
        assert_eq!(board.perft(3), 89890);
        assert_eq!(board.perft(4), 3894594);
    }

    #[test]
    fn test_board_legal_moves_advanced_8() {
        // more from peterellisjones' gist
        let mut board = Board::import("3k4/3p4/8/K1P4r/8/8/8/8 b - - 0 1");
        assert_eq!(board.perft(6), 1134888);
    }

    #[test]
    fn test_board_legal_moves_advanced_9() {
        // more from peterellisjones' gist
        let mut board = Board::import("8/8/4k3/8/2p5/8/B2P2K1/8 w - - 0 1");
        assert_eq!(board.perft(6), 1015133);
    }

    #[test]
    fn test_board_legal_moves_advanced_10() {
        // more from peterellisjones' gist
        let mut board = Board::import("8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1");
        assert_eq!(board.perft(6), 1440467);
    }

    #[test]
    fn test_board_legal_moves_advanced_11() {
        // more from peterellisjones' gist
        let mut board = Board::import("5k2/8/8/8/8/8/8/4K2R w K - 0 1");
        assert_eq!(board.perft(6), 661072);
    }    

    #[test]
    fn test_board_legal_moves_advanced_12() {
        // more from peterellisjones' gist
        let mut board = Board::import("3k4/8/8/8/8/8/8/R3K3 w Q - 0 1");
        assert_eq!(board.perft(6), 803711);
    }

    #[test]
    fn test_board_legal_moves_advanced_13() {
        let mut board = Board::import("r2qk2r/p2np2p/2p1bb1n/1Q1p2P1/1q1P1p2/2NBP3/P1P1NBPP/R2QK2R b KQkq - 0 1");
        assert_eq!(board.perft(4), 4299200);
    }

    #[test]
    fn test_board_legal_moves_advanced_14() {
        // more from peterellisjones' gist
        let mut board = Board::import("r3k2r/8/3Q4/8/8/5q2/8/R3K2R b KQkq - 0 1");
        assert_eq!(board.perft(4), 1720476);
    }

    #[test]
    fn test_board_legal_moves_advanced_15() {
        let mut board = Board::import("q3k2B/2P5/2P3P1/8/8/2p3p1/2p5/b3K2Q w - - 0 1");
        assert_eq!(board.perft(5), 4441461);
    }    

    #[test]
    #[ignore]
    fn test_board_legal_moves_heavy_1() {
        let mut board = Board::default();
        assert_eq!(board.perft(5), 4865609);
    }

    #[test]
    #[ignore]
    fn test_board_legal_moves_heavy_2() {
        // Kiwipete again
        let mut board = Board::import("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        assert_eq!(board.perft(5), 193690690);
    }

    #[test]
    #[ignore]
    fn test_board_legal_moves_heavy_3() {
        let mut board = Board::default();
        assert_eq!(board.perft(6), 119060324);
    }

    #[test]
    #[ignore]
    fn test_board_legal_moves_heavy_4() {
        // Kiwipete again
        let mut board = Board::import("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        assert_eq!(board.perft(6), 8031647685);
    }

    #[test]
    fn test_board_sliding_straight_path() {
        let ar_true  = [[0, 7], [7, 0], [63, 7], [7, 63], [56, 63], [63, 56], [56, 0], [0, 56], [27, 51], [33, 38]];
        let ar_false = [[50, 1], [0, 15], [63, 54], [56, 1], [43, 4], [9, 2], [61, 32], [8, 17], [15, 16], [0, 57], [0, 8]];
        let board = Board::default();
        for case in ar_true.into_iter() {
            assert_ne!(board.get_sliding_straight_path(case[0], case[1]), 0);
            assert_ne!(board.get_sliding_straight_path_unsafe(case[0], case[1]), 0);
        }
        for case in ar_false.into_iter() {
            assert_eq!(board.get_sliding_straight_path(case[0], case[1]), 0);
        }
    }

    #[test]
    fn test_board_sliding_diagonal_path() {
        let ar_true  = [[7, 56], [63, 0], [0, 63], [56, 7], [26, 53], [39, 53], [39, 60], [25, 4], [44, 8]];
        let ar_false = [[0, 10], [56, 6], [39, 31], [3, 40], [2, 23], [5, 34], [63, 1], [62, 0], [49, 46], [23, 16], [2, 3], [7, 14], [1, 8]];
        let board = Board::default();
        for case in ar_true.into_iter() {
            assert_ne!(board.get_sliding_diagonal_path(case[0], case[1]), 0);
            assert_ne!(board.get_sliding_diagonal_path_unsafe(case[0], case[1]), 0);
        }
        for case in ar_false.into_iter() {
            assert_eq!(board.get_sliding_diagonal_path(case[0], case[1]), 0);
        }
    }
    
    #[bench]
    fn perft_5(b: &mut Bencher) {
        let depth = 5;
        let mut board = Board::default();
        let mut x = 0;
        b.iter(|| x = board.perft(depth));
        println!("perft {} result: {}", depth, x);
    }
}