// The main module of the chess engine.
// ANY changes to the board MUST be done through the character's methods!

use std::{cmp::max, collections::{HashMap, HashSet}, time::Instant};
use rand::{rngs::ThreadRng, Rng};
use crate::frame::{util::*, board::Board};
use super::{weights::Weights, zobrist::Zobrist};

pub struct Chara<'a> {
	pub board:				&'a mut Board,
	pub w:					Weights,
	
	/* Cache for evaluated positions as leafs (eval() result) or branches (search result with given a/b) */
	pub cache_leaves:		HashMap<u64, i32>,
	pub cache_branches:		HashMap<u64, EvalBr>,
	
	/* Cache for already made in board moves to track drawish positions */
	pub history_vec:		Vec<u64>,				// previous board hashes stored here to call more quick hash_iter() function
	pub history_set:		HashSet<u64>,			// for fast checking if this position had occured before in this line
													// note: it's always 1 hash behind
	
	/* Accessible constants */
	pub zobrist:			Zobrist,
	pub rng:				ThreadRng,

	/* Search trackers */
	pub book:				i16,					// opening book: 0 - forbidden, 1 - unusable in this game, 2 - will be used
	pub ts:					Instant,				// timer start
	pub tl:					u128,					// time limit in ms
	pub abort:				bool,					// stop search signal
	pub nodes:				u64,					// nodes searched
	pub hmc:				usize,					// current distance to root of the search
													// expected lines of moves
	pub tpv:				[[u32; HALF_DEPTH_LIMIT]; HALF_DEPTH_LIMIT],
													// expected lines of moves length
	pub tpv_len:			[usize; HALF_DEPTH_LIMIT],
	pub tpv_flag:			bool,					// if this is a principle variation (in search)
}

impl<'a> Chara<'a> {
	// It's a little messy, but any evals are easily modifiable.
	// No need in modifying something twice to be applied for the different coloir, neither there's a need to write something twice in eval()
    pub fn init(board: &'a mut Board) -> Chara<'a> {
		let zobrist = Zobrist::default();
		let mut cache_perm_vec = Vec::with_capacity(300);
		cache_perm_vec.push(zobrist.cache_new(board));

		Self {
			board,
			w:				Weights::init(),
			cache_leaves:	HashMap::default(),
			cache_branches:	HashMap::default(),
			history_vec:	cache_perm_vec,
			history_set:	HashSet::default(),
			zobrist,
			rng:			rand::thread_rng(),
			book:			2,
			ts:				Instant::now(),
			tl:				0,
			abort:			false,
			nodes:			0,
			hmc:			0,
			tpv:			[[0; HALF_DEPTH_LIMIT]; HALF_DEPTH_LIMIT],
			tpv_len:		[0; HALF_DEPTH_LIMIT],
			tpv_flag:		false
		}
    }

	pub fn make_move(&mut self, mov: u32) {
		let prev_hash = *self.history_vec.last().unwrap();
		self.history_set.insert(prev_hash);
		self.board.make_move(mov);
		let hash = self.zobrist.cache_iter(self.board, mov, prev_hash);
		self.history_vec.push(hash);
	}

	pub fn revert_move(&mut self) {
		self.board.revert_move();
		self.history_vec.pop();
		self.history_set.remove(self.history_vec.last().unwrap());
	}

	// get the best move
	pub fn think(&mut self, base_aspiration_window: i32, time_limit_ms: u128, depth_limit: i16) -> EvalMove {
		self.ts = Instant::now();

		if self.book > 1 {

			println!("#DEBUG\tMove from an opening book.");

			println!("#DEBUG\tRefuting opening book.");
			self.book = 1;
		}

		self.tl = time_limit_ms;
		self.abort = false;
		self.nodes = 0;
		for line in self.tpv.iter_mut() { for node in line.iter_mut() { *node = 0 } };
		for len in self.tpv_len.iter_mut() { *len = 0 };
		let mut alpha = -INF;
		let mut beta  =  INF;
		let mut depth = 1;
		let mut k = 1;
		let mut score = 0;
		loop {
			self.tpv_flag = true;
			let temp = self.search(alpha, beta, depth);
			if !self.abort {
				score = temp;	
			} else {
				println!("#DEBUG\tAbort signal reached!");
				break;
			}
			if score > LARGM {
				println!("#DEBUG\tMate detected.");
				break;
			}
			if score <= alpha || score >= beta {
				alpha = alpha + base_aspiration_window * k - base_aspiration_window * (k << 2);
				beta = beta - base_aspiration_window * k + base_aspiration_window * (k << 2);
				k <<= 2;
				println!("#DEBUG\tAlpha/beta fail! Using x{} from base aspiration now.", k);
				continue;
			}

			println!("#DEBUG\t--------------------------------");
			println!("#DEBUG\tSearched half-depth: -{}-, score: {}, nodes: {}", depth, score, self.nodes);
			print!("#DEBUG\tExpected line:");
			for mov in self.tpv[0].iter().take(self.tpv_len[0]) {
				print!(" {}", move_transform(*mov));
			}
			println!();

			alpha = score - base_aspiration_window;
			beta = score + base_aspiration_window;
			k = 1;
			depth += 1;
			if depth > depth_limit {
				break;
			}
		}
		println!("#DEBUG\t--------------------------------");
		println!("#DEBUG\tReal time spent: {} ms", self.ts.elapsed().as_millis());
		println!("#DEBUG\tCache limits (in thousands), leaves: {}/{}, branches: {}/{}", self.cache_leaves.len() / 1000, CACHED_LEAVES_LIMIT / 1000, self.cache_branches.len() / 1000, CACHED_BRANCHES_LIMIT / 1000);
		println!("#DEBUG\tReal half-depth: {} to {}, score: {}, nodes: {}", max(self.tpv_len[0], 1), depth - 1, score, self.nodes);
		print!("#DEBUG\tExpected line:");
		for mov in self.tpv[0].iter().take(max(self.tpv_len[0], 1)) {
			print!(" {}", move_transform(*mov));
		}
		println!();
		println!("#DEBUG\t--------------------------------");
		EvalMove::new(self.tpv[0][0], score)
	}

	fn search(&mut self, mut alpha: i32, beta: i32, depth: i16) -> i32 {
		self.tpv_len[self.hmc] = self.hmc;

		let hash = *self.history_vec.last().unwrap();
		if self.hmc != 0 && (self.board.hmc == 50 || self.history_set.contains(&hash)) {
			return 0;
		}
		
		let is_pv = beta - alpha > 1;

		if self.hmc != 0 && !is_pv && self.cache_branches.contains_key(&hash) {
			let br = self.cache_branches[&hash];
			if br.depth >= depth {
				if br.flag & HF_PRECISE != 0 {
					if br.score < -LARGM {
						return br.score + self.hmc as i32;
					} else if br.score > LARGM {
						return br.score - self.hmc as i32;
					}
					return br.score;
				}
				if br.flag & HF_LOW != 0 && br.score <= alpha {
					return alpha;
				}
				if br.flag & HF_HIGH != 0 && br.score >= beta {
					return beta;
				}
			}
		}

		if self.nodes & NODES_BETWEEN_COMMS == 0 {
			self.listen();
		}
		if depth <= 0 {
			return self.extension(alpha, beta);
		}
		self.nodes += 1;
		if self.hmc + 1 > HALF_DEPTH_LIMIT {
			return self.eval();
		}

		let in_check = self.board.is_in_check();

		// Null move prune
		if !in_check && self.hmc != 0 && depth > 2 {
			self.hmc += 1;
			self.board.turn = !self.board.turn;
			self.history_set.insert(*self.history_vec.last().unwrap());
			self.history_vec.push(*self.history_vec.last().unwrap() ^ self.zobrist.hash_turn ^ self.zobrist.hash_en_passant[self.board.en_passant]);
			let old_en_passant = self.board.en_passant;
			self.board.en_passant = 0;

			let score = -self.search(-beta, -beta + 1, depth - 3);	// reduction = 2

			self.board.turn = !self.board.turn;
			self.board.en_passant = old_en_passant;
			self.history_vec.pop();
			self.history_set.remove(self.history_vec.last().unwrap());
			self.hmc -= 1;

			if self.abort {
				return 0;
			}
			if score >= beta {
				return beta;
			}
		}

		let mut moves = self.board.get_legal_moves();
		if moves.is_empty() {
			if in_check {
				self.cache_branches.insert(hash, EvalBr::new(-LARGE, 0, HF_PRECISE));
				return -LARGE + self.hmc as i32;
			}
			self.cache_branches.insert(hash, EvalBr::new(0, 0, HF_PRECISE));
			return 0;
		}

		// todo: fix this bs :(
		moves.sort();
		// follow principle variation first
		if self.tpv_flag {
			self.tpv_flag = false;
			if is_pv {
				let mut i = 0;
				let len = moves.len();
				while i < len {
					if moves[i] == self.tpv[0][self.hmc] {
						self.tpv_flag = true;
						break;
					}
					i += 1;
				}
				while i < len - 1 {
					moves.swap(i, i + 1);
					i += 1;
				}
			}
		}
		moves.reverse();
		
		let mut hf_cur = HF_LOW;
		let reduction = 2;
		// a/b with lmr and pv proving
		for (i, mov) in moves.iter().enumerate() {
			self.make_move(*mov);
			self.hmc += 1;
			let mut score = if i > 3 && depth > 2 && (*mov > MSE_CAPTURE_MIN || in_check) {
				-self.search(-beta, -alpha, depth - reduction)
			} else {
				alpha + 1
			};
			if score > alpha {
				score = -self.search(-alpha - 1, -alpha, depth - 1); // is_pv = false
				if score > alpha && score < beta {
					score = -self.search(-beta, -alpha, depth - 1)
				}
			}
			self.hmc -= 1;
			self.revert_move();
			if self.abort {
				return (alpha + beta) >> 1;
			}
			if score > alpha {
				alpha = score;
				hf_cur = HF_PRECISE;

				// score is better, use this move as principle (expected) variation
				// also copy next halfmove pv into this and adjust its length
				self.tpv[self.hmc][self.hmc] = *mov;
				let mut next = self.hmc + 1;
				while next < self.tpv_len[self.hmc + 1] {
					self.tpv[self.hmc][next] = self.tpv[self.hmc + 1][next];	
					next += 1;
				}
				self.tpv_len[self.hmc] = self.tpv_len[self.hmc + 1];
			}
			if alpha >= beta {
				self.cache_branches.insert(hash, EvalBr::new(score, depth, HF_HIGH));
				self.tpv_flag = true;
				return beta; // fail high
			}
		}

		self.cache_branches.insert(hash, EvalBr::new(alpha, depth, hf_cur));	
		if alpha < -LARGM {
			self.cache_branches.get_mut(&hash).unwrap().score -= self.hmc as i32;	
		} else if alpha > LARGM {
			self.cache_branches.get_mut(&hash).unwrap().score += self.hmc as i32;
		}

		panic!("test");
		alpha // fail low
	}

	fn extension(&mut self, mut alpha: i32, beta: i32) -> i32 {
		if self.nodes & NODES_BETWEEN_COMMS == 0 {
			self.listen();
		}
		self.nodes += 1;

		// cuttin even before we get a list of moves
		alpha = max(alpha, self.eval());
		if alpha >= beta {
			return beta; // fail high
		}

		let mut moves = self.board.get_legal_moves();
		
		// if mate or stalemate
		if moves.is_empty() {
			if self.board.is_in_check() {
				return -LARGE + self.hmc as i32;
			}
			return 0;
		}

		moves.sort();
		moves.reverse();

		for mov in moves.iter() {
			self.make_move(*mov);
			// extension will consider checks as well as captures
			if *mov < MSE_CAPTURE_MIN && !self.board.is_in_check() {
				self.revert_move();
				continue;
			}
			alpha = max(alpha, -self.extension(-beta, -alpha));
			self.revert_move();
			if self.abort {
				return 0;
			}
			if alpha >= beta {
				return beta; // fail high
			}
		}

		alpha // fail low
	}

	/* Warning!
		Before calling this function, consider the following:
		1) Search MUST use check extension! Eval does NOT evaluate checks or free captures specifically!
		2) Search MUST determine if the game ended! Eval does NOT evaluate staled/mated positions specifically!
	*/
	fn eval(&mut self) -> i32 {
		let hash = *self.history_vec.last().unwrap();

		if self.cache_leaves.contains_key(&hash) {
			return self.cache_leaves[&hash];
		}

		/* SETUP SCORE APPLICATION */

		let counter = (self.board.bbs[N] | self.board.bbs[N2]).count_ones() * 3 + 
					  (self.board.bbs[B] | self.board.bbs[B2]).count_ones() * 3 + 
					  (self.board.bbs[R] | self.board.bbs[R2]).count_ones() * 4 +
					  (self.board.bbs[Q] | self.board.bbs[Q2]).count_ones() * 8;

		if counter < 4 && self.board.bbs[P] | self.board.bbs[P2] == 0 {
			self.cache_leaves.insert(hash, 0);
			return 0;
		}

		let phase = (counter < 31) as usize;

		let mut score: i32 = 0;
		let mut mobilities = [0; 2]; // no special moves of any sort are included
		let sides = [self.board.get_occupancies(false), self.board.get_occupancies(true)];
		let occup = sides[0] | sides[1];
		let mptr = &self.board.maps;
		let pawns = [self.board.bbs[P], self.board.bbs[P2]];
		let knights = [self.board.bbs[N], self.board.bbs[N2]];
		let bishops = [self.board.bbs[B], self.board.bbs[B2]];
		let rooks = [self.board.bbs[R], self.board.bbs[R2]];
		let queens = [self.board.bbs[Q], self.board.bbs[Q2]];
		let king = [self.board.bbs[K], self.board.bbs[K2]];
		let kbits = [gtz(king[0]), gtz(king[1])];
		let mut cattacks = [0, 0];

		/* SCORE APPLICATION BEGIN */

		for (ally, mut bb) in pawns.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.w.pieces[phase][P | ally][csq];
				mobilities[ally] += (mptr.attacks_pawns[ally][csq] & sides[enemy]).count_ones() as i32;
				if get_bit(sides[ally], csq + 8 - (ally << 4)) == 0 {
					mobilities[ally] += 1;
					if self.is_easily_protected(csq, pawns[ally], occup, ally, enemy) {
						if self.is_passing(csq, pawns[enemy], ally) {
							score += self.w.good_pawn[phase][ally];
						}
					} else if mptr.piece_pb[ally][csq - 8 + (ally << 4)] & pawns[ally] == 0 {
						score += self.w.bad_pawn[phase][ally];
					}
				} else if mptr.piece_pb[ally][csq - 8 + (ally << 4)] & pawns[ally] == 0 {
						score += self.w.bad_pawn[phase][ally];
				} else {
					score += self.w.bad_pawn[phase][ally] >> 2;
				}
				if mptr.files[csq] & bb != 0 {
					score += self.w.bad_pawn[phase][ally];
				}
			}
		}

		for (ally, mut bb) in knights.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.w.pieces[phase][N | ally][csq];
				let atk = mptr.attacks_knight[csq] & !sides[ally];
				cattacks[ally] |= atk;
				mobilities[ally] += atk.count_ones() as i32;
				if self.is_outpost(csq, pawns[ally], pawns[enemy], ally == 1) {
					score += self.w.outpost[ally][0];
				}
				if (mptr.attacks_dn[csq] & (king[enemy] | queens[enemy] | rooks[enemy])).count_ones() > 1 {
					score += self.w.knight_fork[ally];
				}
			}
		}

		for (ally, mut bb) in bishops.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.w.pieces[phase][B | ally][csq];
				let real_atk = self.board.get_sliding_diagonal_attacks(csq, occup, sides[ally]);
				cattacks[ally] |= real_atk;
				mobilities[ally] += real_atk.count_ones() as i32;
				let imag_atk = self.board.get_sliding_diagonal_attacks(csq, sides[ally], sides[ally]);
				let mut targets = imag_atk & (rooks[enemy] | queens[enemy] | king[enemy]);
				while targets != 0 {
					let tsq = pop_bit(&mut targets);
					if self.get_sliding_diagonal_path_unsafe(csq, tsq) & sides[ally] & pawns[enemy] == 0 {
						score += self.w.bpin[ally];
					}
				}
				if self.is_outpost(csq, pawns[ally], pawns[enemy], ally == 1) {
					score += self.w.outpost[ally][1];
				}
			}
		}

		for (ally, mut bb) in rooks.into_iter().enumerate() {
			// let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.w.pieces[phase][R | ally][csq];
				let real_atk = self.board.get_sliding_straight_attacks(csq, occup, sides[ally]);
				mobilities[ally] += real_atk.count_ones() as i32;
				if real_atk & bb & mptr.files[csq] != 0 && mptr.files[csq] & pawns[ally] == 0 {
					score += self.w.open_battery[ally];
				}
			}
		}
		
		for (ally, mut bb) in queens.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.w.pieces[phase][Q | ally][csq];
				let real_atk = self.board.get_sliding_diagonal_attacks(csq, occup, sides[ally]);
				score += (cattacks[ally] & real_atk & sides[enemy]).count_ones() as i32 * self.w.queen_int[ally];
				if real_atk & king[ally] != 0 {
					score += self.w.bpin[enemy];
				}
			}
		}

		if phase == 0{
			mobilities[0] -= (mptr.attacks_king[kbits[0]] & !sides[0]).count_ones() as i32;
			mobilities[1] -= (mptr.attacks_king[kbits[1]] & !sides[1]).count_ones() as i32;
		} else {
			mobilities[0] += (mptr.attacks_king[kbits[0]] & !sides[0]).count_ones() as i32;
			mobilities[1] += (mptr.attacks_king[kbits[1]] & !sides[1]).count_ones() as i32;
		}
		score += self.w.mobility * (mobilities[0] - mobilities[1]);
		if score > 0 {
			score += score >> self.w.turn_fact;
		} else {
			score -= score >> self.w.turn_fact;
		}
		score += self.w.turn[self.board.turn as usize];

		score -= self.w.random_fact;
		score += self.rng.gen_range(0..=(self.w.random_fact << 1) as u32) as i32;

		/* SCORE APPLICATION END */
		
		if self.board.turn {
			score = -score;
		}

		self.cache_leaves.insert(hash, score);
		score
	}

	fn listen(&mut self) {
		if self.ts.elapsed().as_millis() > self.tl {
			self.abort = true;
		}

		// delet this
		// println!("\n#DEBUG\tcache of leaves ({} entries ).", self.cache_leaves.len());
		// println!("#DEBUG\tcache of branches ({} entries ).\n", self.cache_branches.len());

		if self.cache_leaves.len() > CACHED_LEAVES_LIMIT {
			println!("#DEBUG\tClearing cache of leaves ({} entries dropped).", self.cache_leaves.len());
			self.cache_leaves.clear();
		} else if self.cache_branches.len() > CACHED_BRANCHES_LIMIT {
			println!("#DEBUG\tClearing cache of branches ({} entries dropped).", self.cache_branches.len());
			self.cache_branches.clear();
		}

		// TODO
	}

	/* Auxiliary (used by eval() mostly) */

	#[inline]
    pub fn get_sliding_straight_path_unsafe(&self, sq1: usize, sq2: usize) -> u64 {
        self.board.get_sliding_straight_attacks(sq1, 1 << sq2, 0) & self.board.get_sliding_straight_attacks(sq2, 1 << sq1, 0)
	}

    #[inline]
    pub fn get_sliding_diagonal_path_unsafe(&self, sq1: usize, sq2: usize) -> u64 {
        self.board.get_sliding_diagonal_attacks(sq1, 1 << sq2, 0) & self.board.get_sliding_diagonal_attacks(sq2, 1 << sq1, 0)
	}

    #[inline]
    pub fn is_passing(&self, sq: usize, enemy_pawns: u64, ally_colour: usize) -> bool {
        self.board.maps.piece_passing[ally_colour][sq] & enemy_pawns == 0
    }

    #[inline]
    pub fn is_protected(&self, sq: usize, ally_pawns: u64, enemy_colour: usize) -> bool {
        self.board.maps.attacks_pawns[enemy_colour][sq] & ally_pawns != 0
    }

    #[inline]
    pub fn is_outpost(&self, sq: usize, ally_pawns: u64, enemy_pawns: u64, colour: bool) -> bool {
        self.board.maps.piece_pb[colour as usize][sq] & enemy_pawns == 0 && self.is_protected(sq, ally_pawns, !colour as usize)
    }

    pub fn is_easily_protected(&self, sq: usize, ally_pawns: u64, occupancy: u64, ally_colour: usize, enemy_colour: usize) -> bool {
        // if there are existing pawns on necessary lanes at all
        if self.board.maps.piece_pb[enemy_colour][sq] & ally_pawns == 0 {
            return false;
        }
        // if the piece is already protected enough
        let mut mask = self.board.maps.attacks_pawns[enemy_colour][sq];
        if mask & ally_pawns != 0 {
            return true;
        }
        // if there's nothing standing in the way of any pawn to protect the piece
        while mask != 0 {
            let csq = pop_bit(&mut mask);
            let lane = self.board.maps.files[csq] & self.board.maps.piece_pb[enemy_colour][sq];
            let pbit = lane & ally_pawns;
            if pbit == 0 {
                continue;
            }
            // if we are white - we are interested in leading bit (it's the closest one)
            // otherwise we need trailing bit
            let path = if ally_colour == 0 {
                self.get_sliding_straight_path_unsafe(csq, glz(pbit))
            } else {
                self.get_sliding_straight_path_unsafe(csq, gtz(pbit))
            };
            if path & occupancy == 0 {
                return true;
            }
        }
        false
    }
}


#[cfg(test)]
mod tests {
    use super::*;

	#[test]
	fn test_chara_eval_initial() {
		let mut board = Board::default();
		let mut chara = Chara::init(&mut board);
		let moves = chara.board.get_legal_moves();
		chara.make_move(move_transform_back("e2e4", &moves).unwrap());
		let moves = chara.board.get_legal_moves();
		let mov = move_transform_back("e7e5", &moves).unwrap();
		chara.make_move(mov);
		let eval = chara.eval();
		assert_eq!(eval, chara.w.turn[0]);
	}

	#[test]
	fn test_chara_eval_initial_2() {
		let fens = [
			"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
			"rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
			"rnbqkb1r/pppp1ppp/8/4p2n/4P2N/8/PPPP1PPP/RNBQKB1R w KQkq - 4 4",
			"r3k2r/pbppnpp1/1p1bn2p/4p1q1/4P1Q1/1P1BN2P/PBPPNPP1/R3K2R w KQkq - 2 11"
		];
		for fen in fens.into_iter() {
			let mut board = Board::import(fen);
			let mut chara = Chara::init(&mut board);
			let eval = chara.eval();
			assert_eq!(eval, chara.w.turn[0]);
		}
	}

	#[test]
	fn test_chara_eval_initial_3() {
		let mut board = Board::default();
		let chara = Chara::init(&mut board);
		// this test is bad :D
		assert_eq!(chara.w.pieces[0][K][gtz(chara.board.bbs[K])], 10);
	}

	#[test]
    fn test_board_aux() {
        let ar_true  = [[0, 7], [7, 0], [63, 7], [7, 63], [56, 63], [63, 56], [56, 0], [0, 56], [27, 51], [33, 38]];
        // let ar_false = [[50, 1], [0, 15], [63, 54], [56, 1], [43, 4], [9, 2], [61, 32], [8, 17], [15, 16], [0, 57], [0, 8]];
        let mut board = Board::default();
		let chara = Chara::init(&mut board);
        for case in ar_true.into_iter() {
            // assert_ne!(board.get_sliding_straight_path(case[0], case[1]), 0);
            assert_ne!(chara.get_sliding_straight_path_unsafe(case[0], case[1]), 0);
        }
        // for case in ar_false.into_iter() {
        //     assert_eq!(board.get_sliding_straight_path(case[0], case[1]), 0);
        // }

        let ar_true  = [[7, 56], [63, 0], [0, 63], [56, 7], [26, 53], [39, 53], [39, 60], [25, 4], [44, 8]];
        // let ar_false = [[0, 10], [56, 6], [39, 31], [3, 40], [2, 23], [5, 34], [63, 1], [62, 0], [49, 46], [23, 16], [2, 3], [7, 14], [1, 8]];
        for case in ar_true.into_iter() {
            // assert_ne!(board.get_sliding_diagonal_path(case[0], case[1]), 0);
            assert_ne!(chara.get_sliding_diagonal_path_unsafe(case[0], case[1]), 0);
        }
        // for case in ar_false.into_iter() {
        //     assert_eq!(board.get_sliding_diagonal_path(case[0], case[1]), 0);
        // }

        let board = Board::default();
        assert_eq!(board.is_in_check(), false);
        let board = Board::import("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3");
        assert_eq!(board.is_in_check(), true);
    }
}