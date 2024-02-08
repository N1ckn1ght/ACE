// The main module of the chess engine.
// ANY changes to the board MUST be done through the character's methods!

use std::{cmp::{max, min}, collections::{HashMap, HashSet}, time::Instant};
use rand::{rngs::ThreadRng, Rng};
use crate::frame::{util::*, board::Board};
use crate::engine::{weights::Weights, zobrist::Zobrist, search::search};

// TODO: use i16 instead of floats for faster add/sub operations?
pub struct Chara {
	/* These weights are stored with respect to the colour, black pieces will provide negative values
		- Usual order is:
		- [ Game Phase (0-1) ][ Piece (0-11) ][ Square (0-63) ]	*/
	pub pieces_weights:		[[[f32; 64]; 12]; 2],	// add/subtract eval depending on static positional heatmaps in mittielspiel/endspiel
	pub mobility_weights:	[[[f32; 32]; 12]; 2],	// add/subtract eval depending on piece mobility in mittlespiel/endspiel
	pub turn_weights:		[[f32; 2]; 2],			// mult/add weights per turn if not a 100% draw
	pub align_weights:		[[[f32; 6]; 2]; 2],		// add weight per b/r/q pieces aligned (x-ray) against king / against near square in mittelspiel/endspiel
													// 		for straight aligns it checks if the lane is open (no ally pawn in the way)
	pub battery_weights:	[[f32; 4]; 2],			// add weight per batteries: b+q / r+r horizontal / r+r vertical / r+q vertical
													//		note: in case of b+q, r+q, the weight is added per queen; for rooks it's doubled
	// pub rook_lane_open:	f32,					// add weight, small self-explanatory bonus (obsolete for now because of crazy mobility bonus?)
	/* TODO: Better pawn structure evaluation (hash them separately) */
	pub dp_weight:			f32,					// mult weight per pawn if it has other pawns of same color at this file (use as penalty)
	pub pp_weights:			[f32; 3],				// mult weight per passing / protected by pawn / passing + protected by pawn (unique bonus)
	pub outpost_weights:	[[f32; 2]; 2],			// add weight per passing knight / bishop that's defended by pawn
	pub dan_possible:		[[f32; 3]; 2],			// add weight per knight if it is able to check / able to royal fork / able to fork heavy pieces
	pub baw:				f32,					// base aspiration window
	pub random_range:		f32,					// +-(0 - random_range) value per evaluated position on leaves
	/* Temporary cache; meaning: stores evals and self-cleans when no RAM :( */
	pub cache:				HashMap<u64, Eval>,		// evaluated position stored here (RIP RAM)
	/* Permanent cache; meaning: only already made moves, will take back reverted ones as well */
	pub cache_perm_vec:		Vec<u64>,				// previous board hashes stored here to call more quick hash_iter() function
	pub cache_perm_set:		HashSet<u64>,			// for fast checking if this position had occured before in this line
													// note: it's always 1 hash behind
	/* Accessible constants */
	pub zobrist:			Zobrist,
	pub rng:				ThreadRng
}

/* For more understandable indexing in eval() */
const TURN_MULT:		usize = 0;
const TURN_ADD:			usize = 1;
const ALIGN_BISHOP:		usize = 0;
const ALIGN_ROOK:		usize = 1;
const ALIGN_QUEEN:		usize = 2;
const ALIGN_NEAR:		usize = 3;
const BATTERY_BQ:		usize = 0;
const BATTERY_RRH:		usize = 1;
const BATTERY_RRV:		usize = 2;
const BATTERY_RQV:		usize = 3;
const PAWN_PASSING:		usize = 0;
const PAWN_PROTECTED:	usize = 1;
const PAWN_COMBINED:	usize = 2;
const OUTPOST_KNIGHT:	usize = 0;
const OUTPOST_BISHOP:	usize = 1;
const DAN_CHECK:		usize = 0;
const DAN_ROYAL_FORK:	usize = 1;
const DAN_FORK:			usize = 2;

impl Chara {
	// It's a little messy, but any evals are easily modifiable.
	// No need in modifying something twice to be applied for the different coloir, neither there's a need to write something twice in eval()
    pub fn init(board: &Board, aggressiveness: f32, greed: f32, aspiration_window: f32, random_range: f32) -> Chara {
		let mut w = Weights::default();

		let piece_wmult: f32					= 1.2  * greed;
		let piece_square_related_wmult: f32		= 0.75 / (aggressiveness * 0.5  + 0.5 );	// really slight balance-out
		let mobility_wmult: f32					= 1.0  * (aggressiveness * 0.5  + 0.5 );
		let align_wmult: f32					= 1.0  *  aggressiveness;
		let battery_wmult: f32					= 1.0  *  aggressiveness;
		let pawn_structure_wmult: f32			= 0.5  * (aggressiveness * 0.5  + 0.5 );
		let minor_piece_pos_wmult: f32			= 1.0  * (aggressiveness * 0.5  + 0.5 );

		// transform (flip for white, negative for black) and apply coefficients
		let mut pieces_weights: [[[f32; 64]; 12]; 2] = [[[0.0; 64]; 12]; 2];
		for i in 0..2 {
			for j in 0..6 {
				for k in 0..64 {
					pieces_weights[i][ j << 1     ][k] =  w.pieces_weights_square_related[i][j][flip(k)] * piece_square_related_wmult + w.pieces_weights_const[i][j] * piece_wmult;
					pieces_weights[i][(j << 1) | 1][k] = -w.pieces_weights_square_related[i][j][k      ] * piece_square_related_wmult - w.pieces_weights_const[i][j] * piece_wmult;
				}
			}
		}

		// transform and apply coefficients
		let mut mobility_weights = [[[0.0; 32]; 12]; 2];
		for j in 0..5 {
			for k in 0..32 {
				for i in 0..2 {
					mobility_weights[i][ (j + 1) << 1     ][k] =  w.mobility_weights_const[j][k] * mobility_wmult;
					mobility_weights[i][((j + 1) << 1) | 1][k] = -w.mobility_weights_const[j][k] * mobility_wmult;
				}
			}
		}
		for k in 0..32 {
			mobility_weights[1][10][k] =  w.mobility_weights_const[5][k] * mobility_wmult;
			mobility_weights[1][11][k] = -w.mobility_weights_const[5][k] * mobility_wmult;
		}

		/* Apply coefficients */
		w.turn_weights_pre.iter_mut().for_each(|w| *w = *w * mobility_wmult);
		w.align_weights_pre.iter_mut().for_each(|arr| arr.iter_mut().for_each(|w| *w = *w * align_wmult));
		w.battery_weights_pre.iter_mut().for_each(|w| *w = *w * battery_wmult);
		w.dp_weight += -0.25 + (-pawn_structure_wmult).exp2() * 0.5;
		w.pp_weights.iter_mut().for_each(|w| *w = *w * pawn_structure_wmult);
		w.outpost_weight_pre.iter_mut().for_each(|w| *w = *w * minor_piece_pos_wmult);
		w.dan_possible_pre.iter_mut().for_each(|w| *w = *w * minor_piece_pos_wmult);

		/* Transform */
		let mut turn_weights	= [w.turn_weights_pre.clone(); 2];
		turn_weights[1][1]		= -turn_weights[0][1];
		turn_weights[1][0]		= 1.0 / turn_weights[0][0];
		let mut battery_weights = [w.battery_weights_pre.clone(); 2];
		battery_weights[1].iter_mut().for_each(|w| *w = *w * -1.0);
		let mut outpost_weights	= [w.outpost_weight_pre.clone(); 2];
		outpost_weights[1].iter_mut().for_each(|w| *w = *w * -1.0);
		let mut dan_possible	= [w.dan_possible_pre.clone(); 2];
		dan_possible[1].iter_mut().for_each(|w| *w = *w * -1.0);
		let mut align_weights 	= [[[0.0; 6]; 2]; 2];
		for i in 0..2 {
			for k in 0..6 {
				align_weights[i][0][k] =  w.align_weights_pre[i][k];
				align_weights[i][1][k] = -w.align_weights_pre[i][k];
			}
		}

		let zobrist = Zobrist::default();
		let mut cache_perm_vec = Vec::with_capacity(300);
		cache_perm_vec.push(zobrist.cache_new(board));

		Self {
			pieces_weights,
			mobility_weights,
			turn_weights,
			align_weights,
			battery_weights,
			dp_weight:		w.dp_weight.clone(),
			pp_weights:		w.pp_weights.clone(),
			outpost_weights,
			dan_possible,
			baw:			aspiration_window,
			random_range,
			cache:			HashMap::default(),
			cache_perm_vec,
			cache_perm_set: HashSet::default(),
			zobrist,
			rng:			rand::thread_rng()
		}
    }

	pub fn think(&mut self, board: &mut Board, tl: u128, last_eval: Eval) -> Vec<EvalMove> {
		let ts = Instant::now();

		let mut moves = board.get_legal_moves();
		if moves.len() == 0 {
			return vec![];
		}
		moves.sort();
		let mut moves_evaluated = vec![];
		let maximize = !board.turn;	// 0 -> white to move -> maximize

		let mut alpha = f32::max(0.0, last_eval.score + self.baw);
		let mut beta  = f32::min(0.0, last_eval.score - self.baw);

		let mut depth = 2;
		let mut quit = false;

		loop {

			if maximize {
				for mov in moves.into_iter() {
					self.make_move(board, mov);
					let temp = search(self, board, alpha, beta, false, board.no + depth - 1, false, mov);
					self.revert_move(board);
					moves_evaluated.push(EvalMove::new(mov, temp));
					// or if got sigkill :/
					if ts.elapsed().as_millis() > tl {
						quit = true;
						break;
					}
					alpha = max(alpha, temp);
					if alpha >= beta {
						break;
					}
				}
			} else {
				for mov in moves.into_iter() {
					self.make_move(board, mov);
					let temp = search(self, board, alpha, beta, true , board.no + depth - 1, false, mov);
					self.revert_move(board);
					moves_evaluated.push(EvalMove::new(mov, temp));
					// or if got sigkill :/
					if ts.elapsed().as_millis() > tl {
						quit = true;
						break;
					}
					beta = min(beta, temp);
					if beta <= alpha {
						break;
					}
				}
			}
	
			moves_evaluated.sort_by(|a: &EvalMove, b: &EvalMove| a.eval.cmp(&b.eval));
			if !board.turn {
				moves_evaluated.reverse();
			}

			if quit {
				break;
			}
			
			depth += 2;
		}

		//	moves_evaluated.iter_mut().for_each(|em|);	- TODO: fix mate counter
		moves_evaluated
	}

	pub fn make_move(&mut self, board: &mut Board, mov: u64) {
		let prev_hash = *self.cache_perm_vec.last().unwrap();
		self.cache_perm_set.insert(prev_hash);
		board.make_move(mov);
		let hash = self.zobrist.cache_iter(board, mov, prev_hash);
		self.cache_perm_vec.push(hash);
	}
	
	// pub fn make_move_by_hash(&mut self, board: &mut Board, mov: u64, hash: u64) { }

	pub fn revert_move(&mut self, board: &mut Board) {
		board.revert_move();
		self.cache_perm_vec.pop();
		self.cache_perm_set.remove(&self.cache_perm_vec.last().unwrap());
	}

	/* Warning! 
		1) Search MUST use check extension! Eval does NOT evaluate checks!
		2) Search MUST determine if the game ended! Eval should NOT evaluate staled/mated positions specifically!
	 */
	pub fn eval(&mut self, board: &Board) -> Eval {
		let hash = *self.cache_perm_vec.last().unwrap();
		if self.cache.contains_key(&hash) {
			return self.cache[&hash];
		}

		/* CHECK FOR ANISH GIRI,
			SETUP SCORE APPLICATION */

		if board.hmc == 50 || self.cache_perm_set.contains(&hash) {
			return Eval::new(0.0, 0, board.no);
		}

		let counter = (board.bbs[N] | board.bbs[N2]).count_ones() * 3 + 
					  (board.bbs[B] | board.bbs[B2]).count_ones() * 3 + 
					  (board.bbs[R] | board.bbs[R2]).count_ones() * 4 +
					  (board.bbs[Q] | board.bbs[Q2]).count_ones() * 8;

		if counter < 4 && board.bbs[P] | board.bbs[P2] == 0 {
			self.cache.insert(hash, Eval::new(0.0, 0, board.no));
			return Eval::new(0.0, 0, board.no);
		}

		let phase = (counter < 31) as usize;

		let mut score: f32 = 0.0;
		let sides = [board.get_occupancies(false), board.get_occupancies(true)];
		let occup = sides[0] | sides[1];
		let kbits = [gtz(board.bbs[K]), gtz(board.bbs[K2])];

		/* SCORE APPLICATION BEGIN */

		let pawns = [board.bbs[P], board.bbs[P2]];
		for (ally, mut bb) in pawns.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				let mut ts = self.pieces_weights[phase][P | ally][csq];
				/* Mult penalty per multiplied pawn on the same file */
				if board.maps.files[csq] & board.bbs[P | ally] != 0 {
					ts *= self.dp_weight;
				}
				/* Mult bonus per passing and/or easily protected pawns */
				if board.is_passing(csq, board.bbs[P | enemy], ally) {
					if board.is_easily_protected(csq, board.bbs[P | ally], occup, ally, enemy) {
						ts *= self.pp_weights[PAWN_COMBINED];
					} else {
						ts *= self.pp_weights[PAWN_PASSING];
					}
				} else if board.is_easily_protected(csq, board.bbs[P | ally], occup, ally, enemy) {
					ts *= self.pp_weights[PAWN_PROTECTED];
				}
				score += ts;
			}
		}

		let knights = [board.bbs[N], board.bbs[N2]];
		for (ally, mut bb) in knights.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				score += self.pieces_weights[phase][N | ally][csq];
				score += self.mobility_weights[phase][N | ally][(board.maps.attacks_knight[csq] & !sides[ally]).count_ones() as usize];
				/* Add score if it's an outpost */
				if board.is_outpost(csq, board.bbs[P | ally], board.bbs[P | enemy], ally != 0) {
					score += self.outpost_weights[ally][OUTPOST_KNIGHT];
				}
				/* Add score if some promising forks are existing */
				if board.maps.attacks_dn[csq] & board.bbs[K | enemy] != 0 {
					score += self.dan_possible[ally][DAN_CHECK];
					if board.maps.attacks_dn[csq] & board.bbs[Q | enemy] != 0 {
						score += self.dan_possible[ally][DAN_ROYAL_FORK];
					}
				} else if board.maps.attacks_dn[csq] & board.bbs[Q | enemy] != 0 && board.maps.attacks_dn[csq] & board.bbs[R | enemy] != 0 {
					score += self.dan_possible[ally][DAN_FORK];
				}
			}
		}

		let bishops = [board.bbs[B], board.bbs[B2]];
		for (ally, mut bb) in bishops.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				let real_atk = board.get_sliding_diagonal_attacks(csq, occup, sides[ally]);
				let imag_atk = board.get_sliding_diagonal_attacks(csq, 0, sides[ally]);
				score += self.pieces_weights[phase][B | ally][csq];
				score += self.mobility_weights[phase][B | ally][real_atk.count_ones() as usize];
				/* Add score if it's an outpost */
				if board.is_outpost(csq, board.bbs[P | ally], board.bbs[P | enemy], ally != 0) {
					score += self.outpost_weights[ally][OUTPOST_BISHOP];
				}
				/* Add score if it's aligned against a king */
				if imag_atk & board.bbs[K | enemy] != 0 && board.get_sliding_diagonal_path_unsafe(csq, kbits[enemy]) & board.bbs[P | ally] == 0 {
					score += self.align_weights[phase][ally][ALIGN_BISHOP];
				} else {
					let asq = imag_atk & board.maps.attacks_king[kbits[enemy]];
					if asq != 0 && board.get_sliding_diagonal_path_unsafe(csq, gtz(asq)) & board.bbs[P | ally] == 0 {
						score += self.align_weights[phase][ally][ALIGN_BISHOP + ALIGN_NEAR];
					}
				}
				/* Battery score will be counted per queen */
			}
		}

		let rooks = [board.bbs[R], board.bbs[R2]];
		for (ally, mut bb) in rooks.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				let real_atk = board.get_sliding_straight_attacks(csq, occup, sides[ally]);
				let imag_atk = board.get_sliding_straight_attacks(csq, 0, sides[ally]);
				score += self.pieces_weights[phase][R | ally][csq];
				score += self.mobility_weights[phase][R | ally][(real_atk).count_ones() as usize];
				/* Add score if it's aligned against a king */
				if imag_atk & board.bbs[K | enemy] != 0 && board.get_sliding_straight_path_unsafe(csq, kbits[enemy]) & board.bbs[P | ally] == 0 {
					score += self.align_weights[phase][ally][ALIGN_ROOK];
				} else {
					let asq = imag_atk & board.maps.attacks_king[kbits[enemy]];
					if asq != 0 && board.get_sliding_straight_path_unsafe(csq, gtz(asq)) & board.bbs[P | ally] == 0 {
						score += self.align_weights[phase][ally][ALIGN_ROOK + ALIGN_NEAR];
					}
				}
				/* Add score if it's in a rook battery */
				if real_atk & board.maps.files[csq] & board.bbs[R | ally] != 0 {
					score += self.battery_weights[ally][BATTERY_RRV];
				} else if real_atk & board.maps.ranks[csq] & board.bbs[R | ally] != 0 {
					score += self.battery_weights[ally][BATTERY_RRH];
				}
			}
		}

		let queens = [board.bbs[Q], board.bbs[Q2]];
		for (ally, mut bb) in queens.into_iter().enumerate() {
			let enemy = (ally == 0) as usize;
			while bb != 0 {
				let csq = pop_bit(&mut bb);
				let rdatk = board.get_sliding_diagonal_attacks(csq, occup, sides[ally]);
				let idatk = board.get_sliding_diagonal_attacks(csq, 0, sides[ally]);
				let rsatk = board.get_sliding_straight_attacks(csq, occup, sides[ally]);
				let isatk = board.get_sliding_straight_attacks(csq, 0, sides[ally]);
				score += self.pieces_weights[phase][Q | ally][csq];
				score += self.mobility_weights[phase][Q | ally][(rdatk | rsatk).count_ones() as usize];
				/* Add score if it's aligned against a king */
				if idatk & board.bbs[K | enemy] != 0 && board.get_sliding_diagonal_path_unsafe(csq, kbits[enemy]) & board.bbs[P | ally] == 0 {
					score += self.align_weights[phase][ally][ALIGN_QUEEN];
				} else if isatk & board.bbs[K | enemy] != 0 && board.get_sliding_straight_path_unsafe(csq, kbits[enemy]) & board.bbs[P | ally] == 0 {
					score += self.align_weights[phase][ally][ALIGN_QUEEN];
				} else {
					let asq = idatk & board.maps.attacks_king[kbits[enemy]];
					if asq != 0 {
						if board.get_sliding_diagonal_path_unsafe(csq, gtz(asq)) & board.bbs[P | ally] == 0 {
							score += self.align_weights[phase][ally][ALIGN_QUEEN + ALIGN_NEAR];
						}
					} else {
						let asq = isatk & board.maps.attacks_king[kbits[enemy]];
						if asq != 0 && board.get_sliding_straight_path_unsafe(csq, gtz(asq)) & board.bbs[P | ally] == 0 {
							score += self.align_weights[phase][ally][ALIGN_QUEEN + ALIGN_NEAR];
						}
					}
				}
				/* Add score if it's in a battery */
				if rsatk & board.maps.files[csq] & board.bbs[R | ally] != 0 {
					score += self.battery_weights[ally][BATTERY_RQV];
				}
				if rdatk & board.bbs[B | ally] != 0 {
					score += self.battery_weights[ally][BATTERY_BQ];
				}
			}
		}

		score += self.mobility_weights[phase][K ][(board.maps.attacks_king[kbits[0]] & !sides[0]).count_ones() as usize];
		score += self.mobility_weights[phase][K2][(board.maps.attacks_king[kbits[1]] & !sides[1]).count_ones() as usize];

		score *= self.turn_weights[(board.turn ^ (score < 0.0)) as usize][TURN_MULT];
		score += self.turn_weights[board.turn as usize][TURN_ADD];
		score += self.rng.gen::<f32>() * self.random_range * 2.0 - self.random_range;

		/* SCORE APPLICATION END */

		if self.cache.len() > CACHE_LIMIT {
			self.cache.clear();
		}
		self.cache.insert(hash, Eval::new(score, 0, board.no));
		Eval::new(score, 0, board.no)
	}
}


#[cfg(test)]
mod tests {
    use super::*;

	#[test]
	fn test_chara_eval_initial() {
		let mut board = Board::default();
		let mut chara = Chara::init(&mut board, 0.0, 1.0);
		let moves = board.get_legal_moves();
		board.make_move(move_transform_back("e2e4", &moves).unwrap());
		let moves = board.get_legal_moves();
		let mov = move_transform_back("e7e5", &moves).unwrap();
		board.make_move(mov);
		let eval = chara.eval(&board);
		assert_eq!((eval.score * 1000.0).round(), (chara.turn_weights[0][TURN_ADD] * 1000.0).round());
		assert_eq!(eval.depth, 2);
		assert_eq!(eval.mate, 0);
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
			let mut chara = Chara::init(&mut board, 0.0, 1.0);
			let eval = chara.eval(&board);
			assert_eq!((eval.score * 1000.0).round(), (chara.turn_weights[0][TURN_ADD] * 1000.0).round());
		}
	}
}