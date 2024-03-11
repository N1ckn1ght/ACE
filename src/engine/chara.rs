// The main module of the chess engine.
// ANY changes to the board MUST be done through the character's methods!

use std::{cmp::max, collections::{HashMap, HashSet}, time::Instant};
use rand::{rngs::ThreadRng, Rng};
use crate::frame::{util::*, board::Board};
use super::{weights::Weights, zobrist::Zobrist};

/* CONSTANTS FOR STATIC EVALUATION */
pub const CENTER: [u64; 2] = [0b0000000000000000000110000001100000011000000000000000000000000000, 0b0000000000000000000000000001100000011000000110000000000000000000];
pub const STRONG: [u64; 2] = [0b0000000001111110011111100011110000000000000000000000000000000000, 0b0000000000000000000000000000000000111100011111100111111000000000];
pub const CENTR1: [u64; 2] = [0b0000000000111100001111000011110000111100001111000000000000000000, 0b0000000000000000001111000011110000111100001111000011110000000000];
pub const CENTR2: [u64; 2] = [0b0111111001111110011111100111111001111110011111100111111000000000, 0b0000000001111110011111100111111001111110011111100111111001111110];

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
													// quiet moves that cause a beta cutoff
	pub killer:				[[u32; HALF_DEPTH_LIMIT]; 2],
	pub tpv_flag:			bool,					// if this is a principle variation (in search)
	pub mate_flag:			bool,					// if mate is present

	/* Static eval addon */
	pub castled:			[bool; 2]				// white used castle, black used castle
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
			killer:			[[0; HALF_DEPTH_LIMIT]; 2],
			tpv_flag:		false,
			mate_flag:		false,
			castled:		[false, false]
		}
    }

	pub fn make_move(&mut self, mov: u32) {
		if mov & (MSE_CASTLE_SHORT | MSE_CASTLE_LONG) != 0 {
			self.castled[self.board.turn as usize] = true;
		}
		let prev_hash = *self.history_vec.last().unwrap();
		self.history_set.insert(prev_hash);
		self.board.make_move(mov);
		let hash = self.zobrist.cache_iter(self.board, mov, prev_hash);
		self.history_vec.push(hash);
	}

	pub fn revert_move(&mut self) {
		if self.board.move_history.last().unwrap() & (MSE_CASTLE_SHORT | MSE_CASTLE_LONG) != 0 {
			self.castled[!self.board.turn as usize] = false;
		}
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
		self.mate_flag = false;
		self.nodes = 0;
		for line in self.tpv.iter_mut() { for node in line.iter_mut() { *node = 0 } };
		for len in self.tpv_len.iter_mut() { *len = 0 };
		for num in self.killer.iter_mut() { for mov in num.iter_mut() { *mov = 0 } };
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
				if self.mate_flag {
					break;
				}
				println!("#DEBUG\tMate detected.");
				alpha = -INF;
				beta = INF;
				self.mate_flag = true;
				continue;
			}
			if score <= alpha || score >= beta {
				if k > 15 {
					alpha = -INF;
					beta = INF;
					println!("#DEBUG\tAlpha/beta fail! Using INFINITE values now.");
					continue;
				}
				k *= 2;
				alpha = alpha + base_aspiration_window * k - base_aspiration_window * (k * 2);
				beta = beta - base_aspiration_window * k + base_aspiration_window * (k * 2);
				println!("#DEBUG\tAlpha/beta fail! Using x{} from base aspiration now.", k);
				continue;
			}

			println!("#DEBUG\t--------------------------------");
			println!("#DEBUG\tSearched half-depth: -{}-, score: {}, nodes: {}", depth, score_transform(score, self.board.turn), self.nodes);

			for (j, killer) in self.killer.iter().enumerate() {
				print!("#DEBUG\tKiller {}:", j);
				for (i, mov) in killer.iter().enumerate().take(depth as usize) {
					if *mov != 0 {
						print!(" {}", move_transform(*mov, self.board.turn ^ (i & 1 != 0)));
					} else {
						print!(" -");
					}
				}
				println!();
			}

			print!("#DEBUG\tExpected line:");
			for (i, mov) in self.tpv[0].iter().enumerate().take(max(self.tpv_len[0], 1)) {
				print!(" {}", move_transform(*mov, self.board.turn ^ (i & 1 != 0)));
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
		println!("#DEBUG\tCache limits (in thousands), leaves: {}/{}, branches: {}/{}", self.cache_leaves.len() / 1000, CACHED_LEAVES_LIMIT / 1000, self.cache_branches.len() / 1000, CACHED_BRANCHES_LIMIT / 1000);
		println!("#DEBUG\tReal time spent: {} ms", self.ts.elapsed().as_millis());
		println!("#DEBUG\tReal half-depth: {} to {}, score: {}, nodes: {}", max(self.tpv_len[0], 1), depth - 1, score_transform(score, self.board.turn), self.nodes);
		print!("#DEBUG\tExpected line:");
		for (i, mov) in self.tpv[0].iter().enumerate().take(max(self.tpv_len[0], 1)) {
			print!(" {}", move_transform(*mov, self.board.turn ^ (i & 1 != 0)));
		}
		println!();
		println!("#DEBUG\t--------------------------------");
		EvalMove::new(self.tpv[0][0], score)
	}

	fn search(&mut self, mut alpha: i32, beta: i32, mut depth: i16) -> i32 {
		self.tpv_len[self.hmc] = self.hmc;

		let hash = *self.history_vec.last().unwrap();
		if self.hmc != 0 && (self.board.hmc == 50 || self.history_set.contains(&hash)) {
			return self.w.rand;
		}
		
		// if not a "prove"-search
		if self.hmc != 0 && beta - alpha < 2 && self.cache_branches.contains_key(&hash) {
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

		// follow principle variation first
		if self.tpv_flag {
			self.tpv_flag = false;
			for mov in moves.iter_mut() {
				if *mov == self.tpv[0][self.hmc] {
					self.tpv_flag = true;
					*mov |= MFE_PV1;
					break;
				}
			}
		}
		for mov in moves.iter_mut() {
			move_quiet_pawn_derank(mov, self.board.bbs[P] | self.board.bbs[P2], self.board.turn);
			if *mov == self.killer[0][self.hmc] {
				*mov |= MFE_KILLER1;
				continue;
			}
			if *mov == self.killer[1][self.hmc] { 
				*mov |= MFE_KILLER2;
				continue;
			}
		}
		moves.sort();
		moves.reverse();
		for mov in moves.iter_mut() {
			move_quiet_pawn_derank(mov, self.board.bbs[P] | self.board.bbs[P2], self.board.turn);
		}
		
		let mut hf_cur = HF_LOW;
		depth += in_check as i16;
		// a/b with lmr and pv proving
		for (i, mov) in moves.iter().enumerate() {
			self.make_move(*mov);
			self.hmc += 1;
			let mut score = if i != 0 && depth > 3 && (*mov > ME_CAPTURE_MIN || *mov & !MFE_CLEAR != 0 || in_check) {
				-self.search(-beta, -alpha, depth - 2 - (i > 7 && i + 9 > moves.len()) as i16)
			} else {
				alpha + 1
			};
			if score > alpha {
				score = -self.search(-alpha - 1, -alpha, depth - 1);
				if score > alpha && score < beta {
					score = -self.search(-beta, -alpha, depth - 1)
				}
			}
			self.hmc -= 1;
			self.revert_move();
			if self.abort {
				return 0;
			}
			if score > alpha {
				alpha = score;
				hf_cur = HF_PRECISE;

				// score is better, use this move as principle (expected) variation
				// also copy next halfmove pv into this and adjust its length
				self.tpv[self.hmc][self.hmc] = *mov & MFE_CLEAR;
				let mut next = self.hmc + 1;
				while next < self.tpv_len[self.hmc + 1] {
					self.tpv[self.hmc][next] = self.tpv[self.hmc + 1][next];	
					next += 1;
				}
				self.tpv_len[self.hmc] = self.tpv_len[self.hmc + 1];
			
				if alpha >= beta {
					self.cache_branches.insert(hash, EvalBr::new(score, depth, HF_HIGH));
					if *mov < ME_CAPTURE_MIN {
						self.killer[1][self.hmc] = self.killer[0][self.hmc];
						self.killer[0][self.hmc] = *mov & MFE_CLEAR;
					}
					return beta; // fail high
				}
			}
		}

		self.cache_branches.insert(hash, EvalBr::new(alpha, depth, hf_cur));	
		if alpha < -LARGM {
			self.cache_branches.get_mut(&hash).unwrap().score -= self.hmc as i32;	
		} else if alpha > LARGM {
			self.cache_branches.get_mut(&hash).unwrap().score += self.hmc as i32;
		}

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
			if *mov < ME_CAPTURE_MIN && !self.board.is_in_check() {
				self.revert_move();
				continue;
			}
			self.hmc += 1;
			alpha = max(alpha, -self.extension(-beta, -alpha));
			self.hmc -= 1;
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
		1) Search MUST determine if this position is already happened before, eval won't return 0 in case of repetition or 50 useless moves.
		2) Search MUST determine if the game ended! Eval does NOT evaluate staled/mated positions specifically.
		3) Eval is not great on evaluating checks and detecting possibilities - it's HCE, wdy want?
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
		// 56 - full board, 30 - most likely, endgame?..

		if counter < 4 && self.board.bbs[P] | self.board.bbs[P2] == 0 {
			self.cache_leaves.insert(hash, 0);
			return 0;
		}

		let mut score: i32 = 0;
		let mut score_pd: [i32; 2] = [0, 0];
		// [18 - 56] range
		let phase_diff = f32::min((max(18, counter) - 18) as f32 * 0.025, 1.0);

		let mut pattacks     = [0; 2];
		let mut mobility     = [0; 2];
		let mut pins         = [0; 2];
		let mut sof			 = [0; 2];
		let mut ppt          = [0; 2];
		let mut pass         = [0; 2];

		// quality of life fr
		let bptr = &self.board.bbs;
		let mptr = &self.board.maps;

		let sides = [self.board.get_occupancies(false), self.board.get_occupancies(true)];
		let occup = sides[0] | sides[1];
		let kbits = [gtz(bptr[K]), gtz(bptr[K2])];

		let rpin = [bptr[N] | bptr[B] | bptr[Q], bptr[N2] | bptr[B2] | bptr[Q2]]; // if attack is on Q, it's profitable (most likely will be detected by extension)
		let bpin = [bptr[N] | bptr[R] | bptr[Q], bptr[N2] | bptr[R2] | bptr[Q2]]; // if attack is on R/Q, it's profitable
		let rvic = [bptr[K] | bptr[Q],           bptr[K2] | bptr[Q2]];
		let bvic = [rvic[0] | bptr[R],           rvic[1]  | bptr[R2]];

		/* SCORE APPLICATION BEGIN */

		// pawn quick detections
		for (ally, mut bb) in [bptr[P], bptr[P2]].into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let sq = pop_bit(&mut bb);
				score_pd[0] += self.w.heatmap[0][P | ally][sq];
				score_pd[1] += self.w.heatmap[1][P | ally][sq];
				if bb & mptr.files[sq] != 0 {
					score += self.w.p_doubled[ally];
				}
				if bptr[P | ally] & mptr.flanks[sq] == 0 {
					score += self.w.p_isolated[ally];
				} else if bptr[P | ally] & mptr.flanks[sq] & mptr.ranks[sq] != 0 {
					score += self.w.p_phalanga[ally];
				}
				pattacks[ally] |= mptr.attacks_pawns[ally][sq];
				if (mptr.files[sq] | mptr.flanks[sq]) & mptr.fwd[ally][sq] & bptr[P | enemy] == 0 {
					score += self.w.p_passing[ally][sq >> 3];
					pass[ally] |= 1 << sq;
					ppt[ally] |= mptr.files[sq] & mptr.fwd[ally][sq];
				}
				sof[ally] |= mptr.files[sq];

				let mut profit = mptr.attacks_pawns[ally][sq] & sides[enemy] & !bptr[P | enemy];
				if profit != 0 {
					pop_bit(&mut profit);
					if profit != 0 {
						score += self.w.g_atk_pro[ally];
					} else {
						score += self.w.g_atk_pro_double[ally];
					}
				}
			}
		}
		// 8 consequtive IFs for nails detection
		if get_bit(bptr[P], 10) != 0 && get_bit(sides[1], 18) != 0 {
			score += self.w.p_semiblocked[0];
		}
		if get_bit(bptr[P], 13) != 0 && get_bit(sides[1], 21) != 0 {
			score += self.w.p_semiblocked[0];
		}
		if get_bit(bptr[P], 11) != 0 && get_bit(occup, 19) != 0 {
			score += self.w.p_blocked[0];
		}
		if get_bit(bptr[P], 12) != 0 && get_bit(occup, 20) != 0 {
			score += self.w.p_blocked[0];
		}
		if get_bit(bptr[P2], 50) != 0 && get_bit(sides[0], 42) != 0 {
			score += self.w.p_semiblocked[1];
		}
		if get_bit(bptr[P2], 53) != 0 && get_bit(sides[0], 45) != 0 {
			score += self.w.p_semiblocked[1];
		}
		if get_bit(bptr[P2], 51) != 0 && get_bit(occup, 43) != 0 {
			score += self.w.p_blocked[1];
		}
		if get_bit(bptr[P2], 52) != 0 && get_bit(occup, 44) != 0 {
			score += self.w.p_blocked[1];
		}
		score += (pattacks[0] & (mptr.attacks_king[kbits[1]] | bptr[K2])).count_ones() as i32 * self.w.g_atk_near_king[0][0];
		score += (pattacks[1] & (mptr.attacks_king[kbits[0]] | bptr[K ])).count_ones() as i32 * self.w.g_atk_near_king[1][0];
		score += (pattacks[0] & CENTER[0]).count_ones() as i32 * self.w.p_atk_center[0];
		score += (pattacks[1] & CENTER[1]).count_ones() as i32 * self.w.p_atk_center[1];
		let mut outpost_sqs = [pattacks[0] & STRONG[0], pattacks[1] & STRONG[1]];
		for (ally, mut bb) in outpost_sqs.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let sq = pop_bit(&mut bb);
				if mptr.flanks[sq] & mptr.fwd[ally][sq] & bptr[P | enemy] != 0 {
					del_bit(&mut outpost_sqs[ally], sq);
					continue;
				}
				score += self.w.p_outpost[ally];
				if mptr.step_pawns[ally][sq] & bptr[P | enemy] != 0 {
					score += self.w.p_outpost_block[enemy];
				}
			}
		}

		for (ally, mut bb) in [bptr[Q], bptr[Q2]].into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let sq = pop_bit(&mut bb);
				score_pd[0] += self.w.heatmap[0][Q | ally][sq];
				score_pd[1] += self.w.heatmap[1][Q | ally][sq];
				
				let opr = self.board.get_sliding_straight_opportunities(sq, occup) & self.board.get_sliding_diagonal_opportunities(sq, occup);
				let atk = opr & !sides[ally];
				mobility[ally] += atk.count_ones();

				if get_bit(sof[ally], sq) == 0 {
					if get_bit(sof[enemy], sq) == 0 {
						score += self.w.rq_open[ally];
					} else {
						score += self.w.rq_semiopen[ally];
					}
				}
				if atk & !sof[ally] != 0 {
					if atk & (!sof[ally] & !sof[enemy]) != 0 {
						score += self.w.rq_atk_open[ally];
					} else {
						score += self.w.rq_atk_semiopen[ally];
					}
				}
				if opr & (mptr.attacks_king[kbits[enemy]] | bptr[K | enemy]) != 0 {
					score += self.w.g_atk_near_king[ally][4];
				}
				if atk & (ppt[enemy] | ppt[ally]) != 0 {
					score += self.w.g_atk_ppt[ally];
				}
				if get_bit(ppt[enemy], sq) != 0 {
					score += self.w.g_ppawn_block[ally];
				}
				if opr & CENTER[ally] != 0 {
					score_pd[0] += self.w.g_atk_center[0][ally];
					score_pd[1] += self.w.g_atk_center[1][ally];
				}

				let mut rook_pinned_to   = self.board.get_sliding_straight_attacks(sq, occup & !atk, sides[ally]) & !atk & bptr[K | enemy];
				let mut bishop_pinned_to = self.board.get_sliding_diagonal_attacks(sq, occup & !atk, sides[ally]) & !atk & bptr[K | enemy];
				while rook_pinned_to != 0 {
					let csq = pop_bit(&mut rook_pinned_to);
					pins[enemy] |= self.get_sliding_diagonal_path_unsafe(sq, csq) & rpin[enemy];
				}
				while bishop_pinned_to != 0 {
					let csq = pop_bit(&mut bishop_pinned_to);
					pins[enemy] |= self.get_sliding_diagonal_path_unsafe(sq, csq) & bpin[enemy];
				}

				let profit = atk & bptr[K | enemy];
				if profit != 0 {
					score += self.w.g_atk_pro[ally];
				}
			}
		}

		for (ally, mut bb) in [bptr[R], bptr[R2]].into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let sq = pop_bit(&mut bb);
				score_pd[0] += self.w.heatmap[0][R | ally][sq];
				score_pd[1] += self.w.heatmap[1][R | ally][sq];

				let opr = self.board.get_sliding_straight_opportunities(sq, occup);
				let atk = opr & !sides[ally];
				mobility[ally] += atk.count_ones();
				
				if get_bit(sof[ally], sq) == 0 {
					if get_bit(sof[enemy], sq) == 0 {
						score += self.w.rq_open[ally];
					} else {
						score += self.w.rq_semiopen[ally];
					}
				}
				if atk & !sof[ally] != 0 {
					if atk & (!sof[ally] & !sof[enemy]) != 0 {
						score += self.w.rq_atk_open[ally];
					} else {
						score += self.w.rq_atk_semiopen[ally];
					}
				}
				if opr & (mptr.attacks_king[kbits[enemy]] | bptr[K | enemy]) != 0 {
					score += self.w.g_atk_near_king[ally][3];
				}
				if atk & (ppt[enemy] | ppt[ally]) != 0 {
					score += self.w.g_atk_ppt[ally];
				}
				if atk & ppt[ally] & rvic[enemy] != 0 {
					score += self.w.g_atk_pro_ppb[ally];
				}
				if get_bit(ppt[enemy], sq) != 0 {
					score += self.w.g_ppawn_block[ally];
				}
				if opr & CENTER[ally] != 0 {
					score_pd[0] += self.w.g_atk_center[0][ally];
					score_pd[1] += self.w.g_atk_center[1][ally];
				}
				
				let mut pinned_to = self.board.get_sliding_straight_attacks(sq, occup & !atk, sides[ally]) & !atk & rvic[enemy];
				while pinned_to != 0 {
					let csq = pop_bit(&mut pinned_to);
					pins[enemy] |= self.get_sliding_diagonal_path_unsafe(sq, csq) & rpin[enemy];
				}

				let mut profit = atk & rvic[enemy];
				if profit != 0 {
					pop_bit(&mut profit);
					if profit != 0 {
						score += self.w.g_atk_pro[ally];
					} else {
						score += self.w.g_atk_pro_double[ally];
					}
				}
			}
		}

		for (ally, mut bb) in [bptr[B], bptr[B2]].into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let sq = pop_bit(&mut bb);
				score_pd[0] += self.w.heatmap[0][B | ally][sq];
				score_pd[1] += self.w.heatmap[0][B | ally][sq];

				let opr = self.board.get_sliding_diagonal_opportunities(sq, occup);
				let atk = opr & !sides[ally];
				mobility[ally] += atk.count_ones();

				if get_bit(outpost_sqs[ally], sq) != 0 {
					score += self.w.nb_outpost[ally];
				}
				if outpost_sqs[ally] & atk != 0 {
					score += self.w.nb_outpost_reach[ally];
				}
				if opr & (mptr.attacks_king[kbits[enemy]] | bptr[K | enemy]) != 0 {
					score += self.w.g_atk_near_king[ally][2];
				}
				if atk & (ppt[enemy] | ppt[ally]) != 0 {
					score += self.w.g_atk_ppt[ally];
				}
				if atk & ppt[ally] & bvic[enemy] != 0 {
					score += self.w.g_atk_pro_ppb[ally];
				}
				if get_bit(ppt[enemy], sq) != 0 {
					score += self.w.g_ppawn_block[ally];
				}
				if opr & CENTER[ally] != 0 {
					score_pd[0] += self.w.g_atk_center[0][ally];
					score_pd[1] += self.w.g_atk_center[1][ally];
				}
				
				let mut pinned_to = self.board.get_sliding_diagonal_attacks(sq, occup & !atk, sides[ally]) & !atk & bvic[enemy];
				while pinned_to != 0 {
					let csq = pop_bit(&mut pinned_to);
					pins[enemy] |= self.get_sliding_diagonal_path_unsafe(sq, csq) & bpin[enemy];
				}

				let mut profit = atk & bvic[enemy];
				if profit != 0 {
					pop_bit(&mut profit);
					if profit != 0 {
						score += self.w.g_atk_pro[ally];
					} else {
						score += self.w.g_atk_pro_double[ally];
					}
				}
			}
		}

		for (ally, mut bb) in [bptr[N], bptr[N2]].into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let sq = pop_bit(&mut bb);
				score_pd[0] += self.w.heatmap[0][N | ally][sq];
				score_pd[1] += self.w.heatmap[0][N | ally][sq];

				let opr = mptr.attacks_knight[sq];
				let atk = opr & !sides[ally];
				mobility[ally] += atk.count_ones();

				if get_bit(outpost_sqs[ally], sq) != 0 {
					score += self.w.nb_outpost[ally];
				}
				if outpost_sqs[ally] & mptr.attacks_knight[sq] != 0 {
					score += self.w.nb_outpost_reach[ally];
				}
				if opr & (mptr.attacks_king[kbits[enemy]] | bptr[K | enemy]) != 0 {
					score += self.w.g_atk_near_king[ally][1];
				}
				if atk & (ppt[enemy] | ppt[ally]) != 0 {
					score += self.w.g_atk_ppt[ally];
				}
				if atk & ppt[ally] & bvic[enemy] != 0 {
					score += self.w.g_atk_pro_ppb[ally];
				}
				if get_bit(ppt[enemy], sq) != 0 {
					score += self.w.g_ppawn_block[ally];
				}
				if opr & CENTER[ally] != 0 {
					score_pd[0] += self.w.g_atk_center[0][ally];
					score_pd[1] += self.w.g_atk_center[1][ally];
				}

				let mut profit = atk & bvic[enemy];
				if profit != 0 {
					pop_bit(&mut profit);
					if profit != 0 {
						score += self.w.g_atk_pro[ally];
					} else {
						score += self.w.g_atk_pro_double[ally];
					}
				}
			}
		}

		// lazy ^ 2 checks, not even count bits :(
		if pattacks[0] & pins[1] != 0 {
			score += self.w.g_atk_pro_pinned[0];
		}
		if pattacks[1] & pins[0] != 0 {
			score += self.w.g_atk_pro_pinned[1];
		}
		if pattacks[0] & ppt[0] & sides[1] & !bptr[P2] != 0 {
			score += self.w.g_ppawn_block[0];
		}
		if pattacks[1] & ppt[1] & sides[0] & !bptr[P ] != 0 {
			score += self.w.g_ppawn_block[1];
		}

		for (ally, mut bb) in [bptr[B], bptr[B2]].into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let sq = pop_bit(&mut bb);
				let mut atk = self.board.get_sliding_diagonal_attacks(sq, occup, sides[ally]) & pins[enemy];
				while atk != 0 {
					pop_bit(&mut atk);
					score += self.w.g_atk_pro_pinned[ally];
				}
			}
		}

		for (ally, mut bb) in [bptr[R], bptr[R2]].into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let sq = pop_bit(&mut bb);
				let mut atk = self.board.get_sliding_straight_attacks(sq, occup, sides[ally]) & pins[enemy];
				while atk != 0 {
					pop_bit(&mut atk);
					score += self.w.g_atk_pro_pinned[ally];
				}
			}
		}

		for (ally, mut bb) in [bptr[Q], bptr[Q2]].into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let sq = pop_bit(&mut bb);
				let mut atk = (self.board.get_sliding_diagonal_attacks(sq, occup, sides[ally]) | self.board.get_sliding_straight_attacks(sq, occup, sides[ally])) & pins[enemy];
				while atk != 0 {
					pop_bit(&mut atk);
					score += self.w.g_atk_pro_pinned[ally];
				}
			}
		}

		score_pd[0] += (self.board.get_sliding_diagonal_attacks(kbits[0], occup, sides[0]) | self.board.get_sliding_straight_attacks(kbits[0], occup, sides[0])).count_ones() as i32;
		score_pd[0] += (self.board.get_sliding_diagonal_attacks(kbits[1], occup, sides[1]) | self.board.get_sliding_straight_attacks(kbits[1], occup, sides[1])).count_ones() as i32;
		if mptr.attacks_king[kbits[0]] & (pass[0] | pass[1]) != 0 {
			score_pd[0] += self.w.k_pawn_dist1[0][0];
			score_pd[1] += self.w.k_pawn_dist1[1][0];
		} else if mptr.rad2[kbits[0]] & (pass[0] | pass[1]) != 0 {
			score_pd[0] += self.w.k_pawn_dist2[0][0];
			score_pd[1] += self.w.k_pawn_dist2[1][0];
		}
		if mptr.attacks_king[kbits[1]] & (pass[0] | pass[1]) != 0 {
			score_pd[0] += self.w.k_pawn_dist1[0][1];
			score_pd[1] += self.w.k_pawn_dist1[1][1];
		} else if mptr.rad2[kbits[1]] & (pass[0] | pass[1]) != 0 {
			score_pd[0] += self.w.k_pawn_dist2[0][1];
			score_pd[1] += self.w.k_pawn_dist2[1][1];
		}
		if ((kbits[0] & 7) as i32 - (kbits[1] & 7) as i32).abs() + ((kbits[0] >> 3) as i32  - (kbits[1] >> 3) as i32).abs() == 2 {
			score_pd[0] += self.w.k_opposition[0][!self.board.turn as usize];
			score_pd[1] += self.w.k_opposition[1][!self.board.turn as usize];
		}

		if bptr[K] != 0 && bptr[Q] != 0 {
			score += self.w.s_qnight[0];
		}
		if bptr[K2] != 0 && bptr[Q2] != 0 {
			score += self.w.s_qnight[1];
		}
		if bptr[B] != 0 && (bptr[B] & bptr[B] - 1) != 0 {
			score += self.w.s_bishop_pair[0];
		}
		if bptr[B2] != 0 && (bptr[B2] & bptr[B2] - 1) != 0 {
			score += self.w.s_bishop_pair[1];
		}
		if self.castled[0] {
			score_pd[0] += self.w.s_castled[0][0];
			score_pd[1] += self.w.s_castled[1][0];
		}
		if self.castled[1] {
			score_pd[0] += self.w.s_castled[0][1];
			score_pd[1] += self.w.s_castled[1][1];
		}
		if bptr[K] & CENTR1[0] != 0 {
			score_pd[0] += self.w.k_center_dist1[0][0];
			score_pd[1] += self.w.k_center_dist1[1][0];
		} else if bptr[K] & CENTR2[0] != 0 {
			score_pd[0] += self.w.k_center_dist2[0][0];
			score_pd[1] += self.w.k_center_dist2[1][0];
		}
		if bptr[K2] & CENTR1[1] != 0 {
			score_pd[0] += self.w.k_center_dist1[0][1];
			score_pd[1] += self.w.k_center_dist1[1][1];
		} else if bptr[K2] & CENTR2[1] != 0 {
			score_pd[0] += self.w.k_center_dist2[0][1];
			score_pd[1] += self.w.k_center_dist2[1][1];
		}

		score += ((score_pd[0] as f32 * phase_diff) + (score_pd[1] as f32 * (1.0 - phase_diff))) as i32;
		score += self.w.s_mobility * (mobility[0].count_ones() as i32 - mobility[1].count_ones() as i32);
		if self.board.turn ^ (score > 0) {
			score += score / self.w.s_turn_div;
		} else {
			score -= score / self.w.s_turn_div;
		}
		score += self.w.s_turn[self.board.turn as usize];

		score -= self.w.rand;
		score += self.rng.gen_range(0..=((self.w.rand as u32) << 1)) as i32;

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
}


#[cfg(test)]
mod tests {
    use super::*;

	#[test]
	fn test_chara_eval_initial() {
		let mut board = Board::default();
		let mut chara = Chara::init(&mut board);
		let moves = chara.board.get_legal_moves();
		chara.make_move(move_transform_back("e2e4", &moves, chara.board.turn).unwrap());
		let moves = chara.board.get_legal_moves();
		let mov = move_transform_back("e7e5", &moves, chara.board.turn).unwrap();
		chara.make_move(mov);
		let eval = chara.eval();
		assert_eq!(eval, chara.w.s_turn[0]);
	}

	#[test]
	fn test_chara_eval_initial_2() {
		let fens = [
			"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
			"rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
			"rnbqkb1r/pppp1ppp/8/4p2n/4P2N/8/PPPP1PPP/RNBQKB1R w KQkq - 4 4",
			"r3k2r/pbppnpp1/1p1bn2p/4p1q1/4P1Q1/1P1BN2P/PBPPNPP1/R3K2R w KQkq - 2 11",
			"4k2r/p4ppp/8/8/8/8/P4PPP/4K2R w Kk - 0 1",
			// "8/5P2/p3k3/6P1/1p6/3K3P/2p5/8 w - - 0 1" - assymetric because of PeSto
		];
		for fen in fens.into_iter() {
			let mut board = Board::import(fen);
			let mut chara = Chara::init(&mut board);
			let eval = chara.eval();
			assert_eq!(eval, chara.w.s_turn[0]);
		}

		for fen in fens.into_iter() {
			let mut board = Board::import(fen);
			board.turn = !board.turn;
			let mut chara = Chara::init(&mut board);
			let eval = chara.eval();
			assert_eq!(eval, chara.w.s_turn[0]);
		}
	}

	#[test]
	fn test_chara_eval_initial_3() {
		let wfens = [
			"4k3/8/1pp5/8/7P/6P1/8/4K3 w - - 0 1",
			"rnbq1rk1/ppp2ppp/5n2/2bpp3/4P3/2N2N2/PPPPBPPP/R1BQK2R w KQ - 0 1"
		];
		let bfens = [
			"4k3/8/6p1/7p/8/1PP5/8/4K3 b - - 0 1",
			"r1bqk2r/ppppbppp/2n2n2/4p3/2BPP3/5N2/PPP2PPP/RNBQ1RK1 b kq - 0 1"
		];
		for i in 0..wfens.len() {
			let mut board1 = Board::import(wfens[i]);
			let mut board2 = Board::import(bfens[i]);
			let mut chara1 = Chara::init(&mut board1);
			let mut chara2 = Chara::init(&mut board2);
			assert_eq!(chara1.eval(), chara2.eval());
		}
	}

	#[test]
    fn test_board_aux() {
        let ar_true  = [[0, 7], [7, 0], [63, 7], [7, 63], [56, 63], [63, 56], [56, 0], [0, 56], [27, 51], [33, 38]];
        let mut board = Board::default();
		let chara = Chara::init(&mut board);
        for case in ar_true.into_iter() {
            assert_ne!(chara.get_sliding_straight_path_unsafe(case[0], case[1]), 0);
        }

        let ar_true  = [[7, 56], [63, 0], [0, 63], [56, 7], [26, 53], [39, 53], [39, 60], [25, 4], [44, 8]];
        for case in ar_true.into_iter() {
            assert_ne!(chara.get_sliding_diagonal_path_unsafe(case[0], case[1]), 0);
        }

        let board = Board::default();
        assert_eq!(board.is_in_check(), false);
        let board = Board::import("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3");
        assert_eq!(board.is_in_check(), true);
    }
}