// The main module of the chess engine.
// ANY changes to the board MUST be done through the character's methods!

use std::{cmp::max, collections::{HashMap, HashSet}, time::Instant};
use rand::{rngs::ThreadRng, Rng};
use crate::frame::{util::*, board::Board};
use super::{weights::Weights, zobrist::Zobrist};

pub struct Chara<'a> {
	pub board:				&'a mut Board,

	/* These weights are stored with respect to the colour, black pieces will provide negative values
		- Usual order is:
		- [ Game Phase (0-1) ][ Piece (0-11) ][ Square (0-63) ]	*/
	pub pieces_weights:		[[[i32; 64]; 14]; 2],	// add/subtract eval depending on static positional heatmaps in mittielspiel/endspiel
	pub turn_mult:			[f32; 2],
	pub turn_add:			[i32; 2],				// works when it's not a 100% draw
	
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
	pub new_game:			bool,					// do we try looking into opening book?
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
		let w = Weights::default();

		/* Transform PW (flip for white, negative for black) and apply coefficients */

		let mut pieces_weights: [[[i32; 64]; 14]; 2] = [[[0; 64]; 14]; 2];
		for i in 0..2 {
			for j in 0..6 {
				for k in 0..64 {
					pieces_weights[i][(j << 1) + 2][k] =  w.pieces_weights_square_related[i][j][flip(k)] + w.pieces_weights_const[i][j];
					pieces_weights[i][(j << 1) + 3][k] = -w.pieces_weights_square_related[i][j][k      ] - w.pieces_weights_const[i][j];
				}
			}
		}

		/* Transform other W*/

		let mut turn_mult	= [w.turn_mult_pre; 2];
		turn_mult[1] = 1.0 / turn_mult[0];
		let mut turn_add	= [w.turn_add_pre;  2];
		turn_add [1] =     - turn_add [0];
		
		let zobrist = Zobrist::default();
		let mut cache_perm_vec = Vec::with_capacity(300);
		cache_perm_vec.push(zobrist.cache_new(board));

		Self {
			board,
			pieces_weights,
			turn_mult,
			turn_add,
			cache_leaves:	HashMap::default(),
			cache_branches:	HashMap::default(),
			history_vec:	cache_perm_vec,
			history_set:	HashSet::default(),
			zobrist,
			rng:			rand::thread_rng(),
			new_game:		true,
			ts:				Instant::now(),
			tl:				0,
			abort:			false,
			nodes:			0,
			hmc:			0,
			tpv:			[[0; HALF_DEPTH_LIMIT]; HALF_DEPTH_LIMIT],
			tpv_len:		[0; HALF_DEPTH_LIMIT],
			tpv_flag:		false,
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
		
		if self.new_game {
			// println!("#DEBUG\tMove from an opening book.");
		}
		self.new_game = false;

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
			if score > LARGE - HALF_DEPTH_LIMIT as i32 {
				// mate found lol
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
		println!("#DEBUG\tReal time spent: {} ms", self.ts.elapsed().as_millis());
		println!("#DEBUG\tReal half-depth: {} to {}, score: {}, nodes: {}", max(self.tpv_len[0], 1), depth - 1, score, self.nodes);
		print!("#DEBUG\tExpected line:");
		for mov in self.tpv[0].iter().take(max(self.tpv_len[0], 1)) {
			print!(" {}", move_transform(*mov));
		}
		println!();
		EvalMove::new(self.tpv[0][0], score)
	}

	fn search(&mut self, mut alpha: i32, beta: i32, depth: i16) -> i32 {
		self.tpv_len[self.hmc] = self.hmc;

		let hash = *self.history_vec.last().unwrap();
		if self.board.hmc == 50 || self.history_set.contains(&hash) {
			return 0;
		}
		
		if self.hmc != 0 && self.cache_branches.contains_key(&hash) {
			let br = self.cache_branches[&hash];
			if br.depth >= depth {
				let mut score = br.score;
				if score < -LARGE {
					score += self.hmc as i32;
				} else if score > LARGE {
					score -= self.hmc as i32;
				}
				if br.flag & HF_PRECISE != 0 {
					return score;
				}
				if br.flag & HF_LOW != 0 && score <= alpha {
					return alpha;
				}
				if br.flag & HF_HIGH != 0 && score >= beta {
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

		let mut moves = self.board.get_legal_moves();
		if moves.is_empty() {
			if self.board.is_in_check() {
				return -LARGE + self.hmc as i32;
			}
			return 0;
		}

		moves.sort();
		// follow principle variation first
		let mut reduction = 3;			//  0-8 no reduction,  9-16 1 depth reduction, 33+ it's gonna be extension
		if self.tpv_flag {
			reduction = 4;				// 0-15 no reduction, 16-32 1 depth reduction (e.g. from starting pos it's for knights), 33+ 2 depth reduction
			self.tpv_flag = false;
			if beta - alpha > 1 {
				let mut i = 0;
				while i < moves.len() - 1 {
					if moves[i] == self.tpv[0][self.hmc] {
						reduction = 6;	// no reduction
						break;
					}
					i += 1;
				}
				while i < moves.len() - 1 {
					moves.swap(i, i + 1);
					i += 1;
				}
			}
		}
		moves.reverse();
		
		let mut hf_cur = HF_LOW;
		for (i, mov) in moves.iter().enumerate() {
			self.make_move(*mov);
			self.hmc += 1;
			let score = -self.search(-beta, -alpha, depth - 1 - (i as i16 >> reduction));
			self.hmc -= 1;
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
			self.revert_move();
			if alpha >= beta {
				self.cache_branches.insert(hash, EvalBr::new(score, depth, HF_HIGH));
				self.tpv_flag = true;
				return beta; // fail high
			}
			if self.abort {
				// this may really ruin a game in just 1 move, but what should I do ;w;
				// return 0;
				// return (alpha + beta) >> 1;
				return beta;
			}
		}

		self.cache_branches.insert(hash, EvalBr::new(alpha, depth, hf_cur));
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
			self.hmc += 1;
			alpha = max(alpha, -self.extension(-beta, -alpha));
			self.hmc -= 1;
			self.revert_move();
			if alpha >= beta {
				return beta;
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
		let sides = [self.board.get_occupancies(false), self.board.get_occupancies(true)];
		let occup = sides[0] | sides[1];
		let kbits = [gtz(self.board.bbs[K]), gtz(self.board.bbs[K2])];

		/* SCORE APPLICATION BEGIN */

		let pawns = [self.board.bbs[P], self.board.bbs[P2]];
		for (ally, mut bb) in pawns.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.pieces_weights[phase][P | ally][csq];
			}
		}

		let knights = [self.board.bbs[N], self.board.bbs[N2]];
		for (ally, mut bb) in knights.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.pieces_weights[phase][N | ally][csq];
			}
		}

		let bishops = [self.board.bbs[B], self.board.bbs[B2]];
		for (ally, mut bb) in bishops.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.pieces_weights[phase][B | ally][csq];
			}
		}

		let rooks = [self.board.bbs[R], self.board.bbs[R2]];
		for (ally, mut bb) in rooks.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.pieces_weights[phase][R | ally][csq];
			}
		}

		let queens = [self.board.bbs[Q], self.board.bbs[Q2]];
		for (ally, mut bb) in queens.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.pieces_weights[phase][Q | ally][csq];
			}
		}

		score += self.turn_add[self.board.turn as usize];

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
}


#[cfg(test)]
mod tests {
    use test::Bencher;

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
		assert_eq!(eval, chara.turn_add[0]);
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
			assert_eq!(eval, chara.turn_add[0]);
		}
	}
}