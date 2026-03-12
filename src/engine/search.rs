use std::{cmp::{max, min}, collections::HashSet, sync::mpsc::Receiver, time::Instant};
use rand::{rngs::ThreadRng};
use crate::frame::{util::*, board::Board};
use super::zobrist::Zobrist;


const DEFAULT_VEC_CAPACITY: usize = 300;

enum GameResult {
    InProgress,
    WhiteWon,
    Draw,
    BlackWon
}

pub trait Eval {
    /// Return static evaluation score on a given board
    fn eval(&self, board: &Board) -> i32;
}

pub struct Search<'a> {
    board:				Board,
    eval:               Box<dyn Eval>,
    baw:                i32,                    // aspiration window base
    
    /* Cache for evaluated positions as leafs (eval() result) or branches (search result with given a/b) */
    cache:		        Vec<EvalHash>,
    
    /* Cache for already made in board moves to track drawish positions */
    history_vec:		Vec<u64>,				// previous board hashes stored here to call more quick hash_iter() function
    history_set:		HashSet<u64>,			// for fast checking if this position had occured before in this line
                                                // note: it's always 1 hash behind

    /* Accessible constants */
    zobrist:			Zobrist,
    rng:				ThreadRng,

    /* Search trackers */
    ts:					Instant,				// timer start
    tl:					u128,					// time limit in ms
    abort:				bool,					// stop search signal
    nodes:				u64,					// nodes searched
    hmc:				usize,					// current distance to root of the search
                                                // expected lines of moves
    tpv:				[[u32; HALF_DEPTH_LIMIT]; HALF_DEPTH_LIMIT],
                                                // expected lines of moves length
    tpv_len:			[usize; HALF_DEPTH_LIMIT],
                                                // quiet moves that cause a beta cutoff
    killer:				[[u32; HALF_DEPTH_LIMIT]; 2],
    tpv_flag:			bool,					// if this is a principle variation (in search)
    mate_flag:			bool,					// if mate is present
    cur_depth:          i16,                    // current depth of the iterative dfs (comm-related)

    /* Comms */
    rx:		            &'a Receiver<String>,
    last_score:         i32,                    // last score for the current thinking side (?)

    /* Options */

    cache_size_bits:    i32,
    rand:               i32
}

impl<'a> Search<'a> {
    // Currently uses the board within itself, so doesn't take one as an argument
    pub fn init<E: Eval + 'static>(eval: E, rx: &'a Receiver<String>) -> Self {
        let board = Board::default();
        let zobrist = Zobrist::default();
        let mut cache_perm_vec = Vec::with_capacity(DEFAULT_VEC_CAPACITY);
        cache_perm_vec.push(zobrist.cache_new(&board));

        Self {
            board,
            eval:               Box::new(eval),
            baw:                300, // pretty much default value, divide by 400 to get centipawns
            cache:	            vec![EvalHash::default(); 1 << CACHE_SIZE],
            history_vec:	    cache_perm_vec,
            history_set:	    HashSet::default(),
            zobrist,
            rng:			    rand::thread_rng(),
            ts:				    Instant::now(),
            tl:				    0,
            abort:			    false,
            nodes:			    0,
            hmc:			    0,
            tpv:			    [[0; HALF_DEPTH_LIMIT]; HALF_DEPTH_LIMIT],
            tpv_len:		    [0; HALF_DEPTH_LIMIT],
            killer:			    [[0; HALF_DEPTH_LIMIT]; 2],
            tpv_flag:		    false,
            mate_flag:		    false,
            cur_depth:          0,
            rx,
            last_score:         0
        }
    }

    fn update(&mut self) {
        if self.ts.elapsed().as_millis() > self.tl {
            self.abort = true;
        }

        // TODO
    }

    pub fn think(&mut self, base_aspiration_window: i32, time_limit_ms: u128, depth_limit: i16) -> EvalMove {
        self.ts = Instant::now();
        self.tl = time_limit_ms;
        self.abort = false;
        self.mate_flag = false;
        self.nodes = 0;
        for line in self.tpv.iter_mut() { for node in line.iter_mut() { *node = 0 } };
        for len in self.tpv_len.iter_mut() { *len = 0 };
        for num in self.killer.iter_mut() { for mov in num.iter_mut() { *mov = 0 } };
        let mut alpha = -INF;
        let mut beta  =  INF;
        self.cur_depth = 1;
        let mut k = 1;
        let mut score = 0;
        loop {
            self.tpv_flag = true;
            let temp = self.search(alpha, beta, self.cur_depth);
            if !self.abort {
                score = temp;	
            } else {
                println!("#DEBUG\tAbort signal reached!");
                break;
            }
            // self.last_score = score_to_gui(score, false);
            if self.tpv_len[0] != 0 {
                self.post();
            }
            if !(-LARGM..=LARGM).contains(&score) {
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

            // self.last_score = score_to_gui(score, false);
            alpha = score - base_aspiration_window;
            beta = score + base_aspiration_window;
            k = 1;
            self.cur_depth += 1;
            if self.cur_depth > depth_limit || self.ts.elapsed().as_millis() > self.tl {
                break;
            }
        }

        let approx = self.ts.elapsed().as_millis() + 1;
        println!("#DEBUG\tApproximate time spent: {} ms", approx);
        EvalMove::new(self.tpv[0][0], score)
    }

    pub fn clear(&mut self) {
        self.board = Board::default();
        self.history_set.clear();
        self.history_vec = Vec::with_capacity(DEFAULT_VEC_CAPACITY);
        self.history_vec.push(self.zobrist.cache_new(&self.board));
        self.cur_depth = 0;
        self.nodes = 0;
        // self.last_score = 0;
    }

    pub fn reset_cache(&mut self) {
        self.cache.clear();
        self.cache.resize(1 << self.cache_size_bits, EvalHash::default());
    }

    pub fn set_pos(&mut self, fen: &str) {
        self.clear();
        self.board = Board::import(fen);
        self.history_vec.pop();
        self.history_vec.push(self.zobrist.cache_new(&self.board));
    }

    pub fn make_move(&mut self, mov: u32) {
        let prev_hash = *self.history_vec.last().unwrap();
        self.history_set.insert(prev_hash);
        self.board.make_move(mov);
        let hash = self.zobrist.cache_iter(&self.board, mov, prev_hash);
        self.history_vec.push(hash);
    }

    pub fn revert_move(&mut self) {
        self.board.revert_move();
        self.history_vec.pop();
        self.history_set.remove(self.history_vec.last().unwrap());
    }

    fn search(&mut self, mut alpha: i32, beta: i32, mut depth: i16) -> i32 {
        self.tpv_len[self.hmc] = self.hmc;

        let hash = *self.history_vec.last().unwrap();
        let hash_index = (hash & TEMP_PRE_CALC_CACHE_BITMASK) as usize;
        if self.hmc != 0 && (self.board.hmc > 99 || self.history_set.contains(&hash)) {
            return 0;
        }

        let hash_is_same = self.cache[hash_index].hash == hash;

        // if not a "prove"-search
        if hash_is_same && self.hmc != 0 && beta - alpha < 2 {
            let br = self.cache[hash_index];
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

        if self.nodes & NODES_BETWEEN_UPDATES == 0 {
            self.update();
        }
        if depth <= 0 {
            return self.extension(alpha, beta);
        }
        self.nodes += 1;
        if self.hmc + 1 > HALF_DEPTH_LIMIT {
            return self.static_eval();
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
                return -LARGE + self.hmc as i32;
            }
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
            // additional pre sort flag that could be removed later
            *mov |= MFE_HEURISTIC;
            
            if move_get_piece(*mov) | 1 != P2 {
                // if not a pawn goes into attacked by enemy pawn square, it's probably a poor move (expect it's killer)
                if self.board.maps.attacks_pawns[self.board.turn as usize][move_get_to(*mov, self.board.turn)] & self.board.bbs[P | !self.board.turn as usize] != 0 {
                    *mov &= !MFE_HEURISTIC;
                }
            } else if (
                 self.board.turn && get_bit(RANK_7, move_get_from(*mov, true )) != 0 && get_bit(RANK_6, move_get_to(*mov, true )) != 0
            ) || (
                !self.board.turn && get_bit(RANK_2, move_get_from(*mov, false)) != 0 && get_bit(RANK_3, move_get_to(*mov, false)) != 0
            ) {
                // derank quiet pawn moves as well (1 square forward from start position)
                *mov &= !MFE_HEURISTIC;
            }

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
        
        let mut hf_cur = HF_LOW;
        depth += in_check as i16;
        // a/b with lmr and pv proving
        for (i, mov) in moves.iter().enumerate() {
            self.make_move(*mov);
            self.hmc += 1;
            let mut score = if i != 0 && depth > 2 && !(*mov > ME_PROMISING_MIN || in_check) {
                -self.search(-beta, -alpha, depth - 2 - (depth > 3 && i > 7 && i + 9 > moves.len()) as i16)
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
                    if hash_is_same || depth > min(self.cache[hash_index].depth, 4) {
                        self.cache[hash_index] = EvalHash::new(hash, score, depth, HF_HIGH);
                    }
                    if *mov < ME_CAPTURE_MIN {
                        self.killer[1][self.hmc] = self.killer[0][self.hmc];
                        self.killer[0][self.hmc] = *mov & MFE_CLEAR;
                    }
                    return beta; // fail high
                }
            }
        }

        if hash_is_same || depth > min(self.cache[hash_index].depth, 4) {
            self.cache[hash_index] = EvalHash::new(hash, alpha, depth, hf_cur);	
            if alpha < -LARGM {
                self.cache[hash_index].score -= self.hmc as i32;
            } else if alpha > LARGM {
                self.cache[hash_index].score += self.hmc as i32;
            }
        }

        alpha // fail low
    }

    fn extension(&mut self, mut alpha: i32, beta: i32) -> i32 {
        if self.nodes & NODES_BETWEEN_UPDATES == 0 {
            self.update();
        }
        self.nodes += 1;

        // cuttin even before we get a list of moves
        alpha = max(alpha, self.static_eval());
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
    fn static_eval(&mut self) -> i32 {
        self.eval.eval(&self.board)
    }

    /* Play functions */

    fn get_result(&mut self) -> GameResult {
        let moves = self.board.get_legal_moves();
        if moves.is_empty() {
            if self.board.is_in_check() {
                if self.board.turn {
                    return GameResult::WhiteWon;
                }
                return GameResult::BlackWon;
            }
            return GameResult::Draw;
        }
        if self.board.hmc > 99 {
            return GameResult::Draw;
        }
        // TODO: autodraw on move repetition
        GameResult::InProgress
    }

    fn post(&self) {
        // let scu = self.last_score;
        let scu = 0;
        let started_black = true;
        print!("{} {} {} {}", self.cur_depth, scu, self.ts.elapsed().as_millis() / 10, self.nodes);
        for (i, mov) in self.tpv[0].iter().enumerate().take(max(self.tpv_len[0], 1)) {
            print!(" {}", move_transform(*mov, (i & 1 != 0) ^ started_black));
        }
        println!();
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

}