use std::{cmp::{max, min}, collections::HashMap, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::{Duration, Instant}};
use crate::{engine::hc_eval::{EVAL_MATE, EVAL_PRE_MATE}, frame::{board::Board, util::*}};
use super::{hc_eval::EVAL_INF, zobrist::Zobrist};


const DEFAULT_VEC_CAPACITY: usize = 300;

#[derive(Debug, PartialEq)]
pub enum GameResult {
    InProgress,
    WhiteWon,
    Draw,
    BlackWon
}

pub trait Eval {
    /// Return static evaluation score on a given board
    fn eval(&self, board: &Board) -> i16;
}

pub struct Search {
    board:				Board,

    /* Handles */
    pub abort:			Arc<AtomicBool>,		// stop search signal (if true, exit immediately)
    pub ponder:         Arc<AtomicBool>,        // kinda "don't stop search" signal (even if mate)
    searchmoves:        Vec<u32>,
    pub do_post:        bool,
    
    /* Cache for evaluated positions as leafs (eval() result) or branches (search result with given a/b) */
    cache:		        Vec<EvalHash>,
    cached_cnt:         u64,
    cache_size_bits:    u8,
    cache_mask:         u64,
    
    /* Cache for already made in board moves to track drawish positions */
    history_vec:		Vec<u64>,				// previous board hashes stored here to call more quick hash_iter() function
    history_set:		HashMap<u64, u8>,   	// for fast checking if this position had occured before in this line
                                                // note: it's always 1 hash behind

    /* Accessible constants */
    zobrist:			Zobrist,

    /* Search trackers */
    ts:					Instant,				// timer start
    tl:					u64,					// time limit in ms
    nodes:				u64,					// nodes searched
    nl:                 u64,                    // node_limit_set
    ply:				usize,					// current distance to root of the search
                                                // expected lines of moves
    tpv:				[[u32; HARD_DEPTH_LIMIT]; HARD_DEPTH_LIMIT],
                                                // expected lines of moves length
    tpv_len:			[usize; HARD_DEPTH_LIMIT],
                                                // quiet moves that cause a beta cutoff
    killer:				[[u32; HARD_DEPTH_LIMIT]; 2],
    tpv_flag:			bool,					// if this is a principle variation (in search)
    mate_flag:			bool,					// if mate is present
    cur_depth:          u8,                     // current depth of the iterative dfs (comm-related)
}

impl Search {
    // Currently uses the board within itself, so doesn't take one as an argument
    pub fn init() -> Self {
        let board = Board::default();
        let zobrist = Zobrist::default();
        let mut cache_perm_vec = Vec::with_capacity(DEFAULT_VEC_CAPACITY);
        cache_perm_vec.push(zobrist.cache_new(&board));
        let cache_size_bits = EvalHash::calc_cache_size_bits_from_mb(24);

        Self {
            board,
            cache:              vec![EvalHash::default(); 1 << cache_size_bits],
            cached_cnt:         0,
            history_vec:	    cache_perm_vec,
            history_set:	    HashMap::default(),
            zobrist,
            ts:				    Instant::now(),
            tl:				    0,
            abort:			    Arc::new(AtomicBool::new(true)),
            ponder:             Arc::new(AtomicBool::new(false)),
            do_post:            true,
            nodes:			    0,
            nl:                 0,
            ply:			    0,
            tpv:			    [[0; HARD_DEPTH_LIMIT]; HARD_DEPTH_LIMIT],
            tpv_len:		    [0; HARD_DEPTH_LIMIT],
            killer:			    [[0; HARD_DEPTH_LIMIT]; 2],
            tpv_flag:		    false,
            mate_flag:		    false,
            cur_depth:          0,
            cache_size_bits,
            cache_mask:         (1 << cache_size_bits) - 1,
            searchmoves:        vec![]
        }
    }

    /// Use this public function to initiate search and pass limitations
    /// 
    /// Input (searchmoves) and output goes in string format (e.g. e2e4, e7e5)
    /// 
    /// Returns bestmove and Option(ponder) with some, if present in tpv
    pub fn go<E: Eval>(
        &mut self,
        eval: &E,
        time_limit_ms: u64,
        depth_target: u8,
        node_limit: u64,  // pass 0 if None
        searchmoves: Option<&[&str]>
    ) -> EngineOutput {
        self.ts = Instant::now();
        self.tl = time_limit_ms;
        self.nl = node_limit;
        self.abort.store(false, Ordering::Relaxed);
        self.nodes = 1;
        for line in self.tpv.iter_mut() { for node in line.iter_mut() { *node = 0 } };
        for len in self.tpv_len.iter_mut() { *len = 0 };
        for num in self.killer.iter_mut() { for mov in num.iter_mut() { *mov = 0 } };

        let mut alpha = -EVAL_INF;
        let mut beta  =  EVAL_INF;
        self.cur_depth = 1;
        self.searchmoves = vec![];
        if let Some(moves) = searchmoves {
            let plm = self.get_pseudo_legal_moves();  // assuming...
            self.searchmoves = vec![];
            for move_str in moves.iter() {
                self.searchmoves.push(move_transform_back(move_str, &plm, self.board.turn).unwrap());
            }
        }
        let mut k = 2;
        let mut score = 0;
        let baw = 75;  // in centipawns

        loop {
            self.tpv_flag = true;
            let temp = self.search(eval, alpha, beta, self.cur_depth);
            if self.abort.load(Ordering::Relaxed) {
                log(&format!("Abort signal reached! Nodecount: {}", self.nodes));
                break;
            } else {
                score = temp;
            }
            self.post(score);
            if !(-EVAL_PRE_MATE..=EVAL_PRE_MATE).contains(&score) {
                if self.mate_flag {
                    break;
                }
                log("Mate detected.");
                self.mate_flag = true;
                if self.cur_depth == 1 {
                    break;
                }
                continue;
            }
            if score <= alpha || score >= beta {
                if k > 16 {
                    alpha = -EVAL_INF;
                    beta = EVAL_INF;
                    log("Alpha/beta fail! Using INFINITE values now.");
                    continue;
                }
                alpha = alpha + baw * k - baw * (k * 2);
                beta = beta - baw * k + baw * (k * 2);
                log(&format!("Alpha/beta fail! Using x{} from base aspiration now.", k));
                k *= k;
                continue;
            }

            // self.last_score = score_to_gui(score, false);
            alpha = score - baw;
            beta = score + baw;
            k = 2;
            self.cur_depth += 1;
            if self.cur_depth > depth_target || (self.ts.elapsed().as_millis() as u64 > self.tl && !self.ponder.load(Ordering::Relaxed)) {
                break;
            }
        }

        // do not exit search while ponder even if it is mate
        while self.ponder.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(1));
        }

        let ui_score = Self::score_to_uci(score);
        log(&format!("Approximate time spent: {} ms", self.ts.elapsed().as_millis() as u64));
        EngineOutput { 
            bestmove: move_transform(self.tpv[0][0], self.board.turn),
            ponder: if self.cur_depth > 2 {Some(move_transform(self.tpv[0][1], !self.board.turn))} else {None},
            score_type: ui_score.0,
            score_value: ui_score.1
        }
    }

    pub fn set_pos(&mut self, fen: Option<&str>) {
        self.clear_history();
        self.board.set_pos(fen.unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"));
        self.history_vec.pop();
        self.history_vec.push(self.zobrist.cache_new(&self.board));
    }

    pub fn clear_history(&mut self) {
        self.history_set.clear();
        self.history_vec = Vec::with_capacity(DEFAULT_VEC_CAPACITY);
        self.history_vec.push(self.zobrist.cache_new(&self.board));
    }

    pub fn set_cache_size(&mut self, megabytes: u32) {
        self.cache_size_bits = EvalHash::calc_cache_size_bits_from_mb(megabytes);
        self.cache_mask = (1 << self.cache_size_bits) - 1;
        self.clear_cache();
    }

    pub fn clear_cache(&mut self) {
        self.cache = vec![EvalHash::default(); 1 << self.cache_size_bits];
        self.cached_cnt = 0;
    }

    pub fn make_move(&mut self, mov: u32) {
        let prev_hash = *self.history_vec.last().unwrap();
        *self.history_set.entry(prev_hash).or_insert(0) += 1;
        self.board.make_move(mov);
        let hash = self.zobrist.cache_iter(&self.board, mov, prev_hash);
        self.history_vec.push(hash);
    }

    pub fn undo_move(&mut self) {
        self.board.undo_move();
        self.history_vec.pop();
        let last = self.history_vec.last().unwrap();
        let count = self.history_set.get_mut(last).unwrap();
        *count -= 1;
        if *count == 0 {
            self.history_set.remove(last);
        }
    }

    fn update(&mut self) {
        if self.nl != 0 && self.nodes > self.nl {
            self.abort.store(true, Ordering::Relaxed);
            return;
        }
        if self.nodes & NODES_BETWEEN_UPDATES == 0 {
            if self.nodes & POST_INTERVAL == 0 {
                self.post_regular();
            }
            if self.ponder.load(Ordering::Relaxed) {
                return;
            }
            if self.ts.elapsed().as_millis() as u64 > self.tl {
                self.abort.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Launch a search with given a/b to a certain depth limit
    /// 
    /// Takes an object with eval implemented as a parameter
    /// 
    /// Pass -INF/+INF to get a precise result (e.g. mate)
    fn search<E: Eval>(
        &mut self,
        eval: &E,
        mut alpha: i16,
        beta: i16,
        mut depth: u8
    ) -> i16 {
        let root_node = self.ply == 0;

        self.tpv_len[self.ply] = self.ply;

        let hash = *self.history_vec.last().unwrap();
        let hash_index = (hash & self.cache_mask) as usize;
        if !root_node && (self.board.hmc > 99 || self.history_set.contains_key(&hash)) {
            return 0;
        }

        let hash_is_same = self.cache[hash_index].is_same(hash);
        // if not a "prove"-search
        if hash_is_same && !root_node && beta < alpha + 2 {
            let br = self.cache[hash_index];
            if br.depth >= depth {
                if br.flag & HF_PRECISE != 0 {
                    if br.score < -EVAL_PRE_MATE {
                        return br.score + self.ply as i16;
                    } else if br.score > EVAL_PRE_MATE {
                        return br.score - self.ply as i16;
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

        let in_check = self.board.is_in_check();
        if in_check {
            depth += 1;  // bruh
        } else if depth == 0 {
            return self.extension(eval, alpha, beta, true);
        }

        // Exit search (tl, nl, listen)
        self.update();
        self.nodes += 1;

        // sus
        if self.ply & HARD_DEPTH_LIMIT != 0 {
            return eval.eval(&self.board);
        }

        // Disabled, needs more testing (including mate search autotests AND elo strength test)
        // // Null move prune
        // // if: 1) not in check, 2) not the first damn node, 3) not the (possible) PV, 4) not search for mate, 5) we have enough depth
        // if !in_check && !root_node && !self.tpv_flag && depth > 3 && !self.mate_flag {
        //     // !!! check if the previous move was already a null-move
        //     // at this point, self.history_vec.len() have to be more than 1
        //     if self.history_vec[self.history_vec.len() - 2] ^ self.zobrist.hash_turn ^ self.zobrist.hash_en_passant[0] != *self.history_vec.last().unwrap() {
        //         // the null move itself
        //         self.board.turn = !self.board.turn;
        //         self.ply += 1;
        //         // remove the en passant option manually
        //         let old_en_passant = self.board.en_passant;
        //         self.board.en_passant = 0;

        //         // make the hash for the changed color and pass it as well
        //         let prev_hash = *self.history_vec.last().unwrap();
        //         *self.history_set.entry(prev_hash).or_insert(0) += 1;
        //         self.history_vec.push(prev_hash ^ self.zobrist.hash_turn ^ self.zobrist.hash_en_passant[0]);

        //         // calc a quick reducted score
        //         let reduction = 2 + depth / 5;
        //         let score = -self.search(eval, -beta, -beta + 1, depth - reduction);

        //         // restore the original board state
        //         self.ply -= 1;
        //         self.board.turn = !self.board.turn;
        //         self.board.en_passant = old_en_passant;

        //         // remove the null-move from stack
        //         self.history_vec.pop();
        //         let count = self.history_set.get_mut(&hash).unwrap();
        //         if *count == 1 {
        //             self.history_set.remove(&hash);
        //         } else {
        //             *count -= 1;
        //         }

        //         if self.abort.load(Ordering::Relaxed) {
        //             return 0;
        //         }
        //         if score >= beta {
        //             if score > EVAL_PRE_MATE {
        //                 // alpha = EVAL_PRE_MATE - 1;
        //             } else {
        //                 return score;
        //             }
        //         }
        //     }
        // }

        let mut moves = self.board.get_legal_moves();
        // uci "go searchmoves" de momento
        if root_node && !self.searchmoves.is_empty() {
            moves.retain(|x| self.searchmoves.contains(x));
        }
        
        if moves.is_empty() {
            if in_check {
                return -EVAL_MATE + self.ply as i16;
            }
            return 0;
        }

        // follow principle variation first
        if self.tpv_flag {
            self.tpv_flag = false;
            for mov in moves.iter_mut() {
                if *mov == self.tpv[0][self.ply] {
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

            if *mov == self.killer[0][self.ply] {
                *mov |= MFE_KILLER1;
                continue;
            }
            if *mov == self.killer[1][self.ply] { 
                *mov |= MFE_KILLER2;
                continue;
            }
        }
        moves.sort();
        
        let mut hf_cur = HF_LOW;
        // a/b with lmr and pv proving
        for (i, mov) in moves.iter().rev().enumerate() {
            self.make_move(*mov);
            self.ply += 1;

            // LMR if: [1) not search for mate] 2) not in check 3) not a capture 4) not the first move 5) it's physically possible
            let mut score = if !in_check && *mov < ME_PROMISING_MIN && i != 0 && depth > 2 && !self.mate_flag {
                -self.search(
                    eval,
                    -beta,
                    -alpha,
                    depth - 2 - (depth > 3 && i > 7 && i + 9 > moves.len()) as u8
                )
            } else {
                alpha + 1
            };
            if score > alpha {
                score = -self.search(eval, -alpha - 1, -alpha, depth - 1);
                if score > alpha && score < beta {
                    score = -self.search(eval, -beta, -alpha, depth - 1)
                }
            }

            self.ply -= 1;
            self.undo_move();
            if self.abort.load(Ordering::Relaxed) {
                return 0;
            }
            if score > alpha {
                alpha = score;
                hf_cur = HF_PRECISE;

                // score is better, use this move as principle (expected) variation
                // also copy next halfmove pv into this and adjust its length
                self.tpv[self.ply][self.ply] = *mov & MFE_CLEAR;
                let mut next = self.ply + 1;
                while next < self.tpv_len[self.ply + 1] {
                    self.tpv[self.ply][next] = self.tpv[self.ply + 1][next];	
                    next += 1;
                }
                self.tpv_len[self.ply] = self.tpv_len[self.ply + 1];
            
                if alpha >= beta {
                    // cache usage (1/2)
                    if hash_is_same || depth > min(self.cache[hash_index].depth, 4) {
                        if self.cache[hash_index].depth == 0 {
                            self.cached_cnt += 1;
                        }
                        self.cache[hash_index] = EvalHash::new(hash, score, depth, HF_HIGH);
                    }

                    if *mov < ME_CAPTURE_MIN {
                        self.killer[1][self.ply] = self.killer[0][self.ply];
                        self.killer[0][self.ply] = *mov & MFE_CLEAR;
                    }
                    return beta; // fail high
                }
            }
        }

        // cache usage (2/2)
        if hash_is_same || depth > min(self.cache[hash_index].depth, 4) {
            if self.cache[hash_index].depth == 0 {
                self.cached_cnt += 1;
            }
            self.cache[hash_index] = EvalHash::new(hash, alpha, depth, hf_cur);	
        }

        alpha  // fail low
    }

    /// Looks through captures 'til the end
    /// 
    /// Gets out of checks
    /// 
    /// On a root node gives checks as well
    /// 
    /// Evaluates all quiet leafs
    fn extension<E: Eval>(
        &mut self,
        eval: &E,
        mut alpha: i16,
        beta: i16,
        root_ext: bool
    ) -> i16 {
        // Exit search extension (tl, nl, listen)
        self.update();
        self.nodes += 1;

        let in_check = self.board.is_in_check();
        if !in_check {
            alpha = max(alpha, eval.eval(&self.board));
            // cuttin even before we get a list of moves
            if alpha >= beta {
                return beta;  // fail high
            }
        }

        let mut moves = self.board.get_legal_moves();
        // if mate or stalemate
        if moves.is_empty() {
            if in_check {
                return -EVAL_MATE + self.ply as i16;
            }
            return 0;
        }

        moves.sort();

        for mov in moves.iter().rev() {
            if root_ext {
                self.make_move(*mov);
                if !in_check && *mov < ME_CAPTURE_MIN && !self.board.is_in_check() {
                    // if are not in check, and this move is not promising, and this move does not lead to a check, drop it
                    self.undo_move();
                    continue;
                }
            } else {
                // if not a root node of search extension, giving checks will lead to perpetual
                if !in_check && *mov < ME_CAPTURE_MIN {
                    continue;
                }
                // also make move after conditioning because it's possible here and it's faster
                self.make_move(*mov);
            }

            self.ply += 1;
            alpha = max(alpha, -self.extension(eval, -beta, -alpha, false));
            self.ply -= 1;
            self.undo_move();

            if self.abort.load(Ordering::Relaxed) {
                return 0;
            }
            if alpha >= beta {
                return beta;  // fail high
            }
        }

        alpha  // fail low
    }

    fn post(&self, score: i16) {
        if !self.do_post {
            return;
        }
        let (st, sv) = Self::score_to_uci(score);
        let time = self.ts.elapsed().as_millis() as u64;
        let cache_max = 1 << self.cache_size_bits;
        print!("info depth {} score {} {} nodes {} nps {} hashfull {} time {} pv",
            self.cur_depth,
            st, sv,
            self.nodes,
            self.nodes * 1000 / time.max(1),
            self.cached_cnt * 1000 / cache_max,
            time
        );
        for (i, mov) in self.tpv[0].iter().enumerate().take(max(self.tpv_len[0], 1)) {
            print!(" {}", move_transform(*mov, (i & 1 != 0) ^ self.board.turn));
        }
        println!();
    }

    fn post_regular(&self) {
        if !self.do_post {
            return;
        }
        let time = self.ts.elapsed().as_millis() as u64;
        let cache_max = 1 << self.cache_size_bits;
        println!("info nodes {} nps {} hashfull {}",
            self.nodes,
            self.nodes * 1000 / time.max(1),
            self.cached_cnt * 1000 / cache_max
        );
    }

    fn score_to_uci(score: i16) -> (String, i16) {
        if score < 0 {
            if score < -EVAL_PRE_MATE {
                return ("mate".to_owned(), -((EVAL_MATE + score) / 2));
            }
            return ("cp".to_owned(), score);
        }
        if score > EVAL_PRE_MATE {
            return ("mate".to_owned(), 1 + (EVAL_MATE - score) / 2);
        }
        ("cp".to_owned(), score)
    }

    /* Aux */

    pub fn get_legal_moves(&mut self) -> Vec<u32> {
        self.board.get_legal_moves()        
    }

    pub fn get_pseudo_legal_moves(&self) -> Vec<u32> {
        self.board.get_pseudo_legal_moves()
    }

    #[inline]
    pub fn get_turn(&self) -> bool {
        self.board.turn
    }

    /// 0-depth eval, if position is not quiet then this eval is incorrect
    /// 
    /// Returns score and is_quiet, so you don't have to prepare position
    pub fn get_static_eval<E: Eval>(&mut self, eval: &E) -> EvalOutput {
        let score = eval.eval(&self.board);
        let q_score = self.extension(eval, -EVAL_INF, EVAL_INF, false);
        let qr_score = self.extension(eval, -EVAL_INF, EVAL_INF, true);

        EvalOutput {
            score,
            q_score,
            qr_score,
            is_quiet: self.get_result() == GameResult::InProgress && score == q_score && !self.board.is_in_check()
        }
    }

    pub fn get_result(&mut self) -> GameResult {
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

    pub fn make_move_safe(&mut self, movstr: &str) -> bool {
        let moves = self.board.get_legal_moves();
        let mov = move_transform_back(movstr, &moves, self.board.turn);
        if mov.is_none() {
            return false;
        }
        self.make_move(mov.unwrap());
        true
    }

    pub fn undo_move_safe(&mut self) -> bool {
        if self.history_vec.len() < 2 {
            return false;
        }
        self.undo_move();
        true
    }

    pub fn export_fen(&self) -> String {
        self.board.export_fen()
    }
}

/* CACHE related section */

const HF_PRECISE: u8 = 1;
const HF_LOW: u8 = 2;
const HF_HIGH: u8 = 4;

#[derive(Copy, Clone, Default)]
struct EvalHash {
    pub hash_upper_part: u32,
    pub hash_lower_part: u32,
    pub score: i16, 
    pub depth: u8,
    pub flag: u8
}

impl EvalHash {    
    pub fn new(hash: u64, score: i16, depth: u8, flag: u8) -> Self {
        EvalHash {
            hash_upper_part: (hash >> 32) as u32,
            hash_lower_part: hash as u32,
            score,
            depth,
            flag
        }
    }

    pub fn is_same(&self, hash: u64) -> bool {
        hash as u32 == self.hash_lower_part && (hash >> 32) as u32 == self.hash_upper_part
    }

    pub fn calc_cache_size_bits_from_mb(megabytes: u32) -> u8 {
        let eh = 96;
        let rsz = ((megabytes as u64) * 8 * 1024 * 1024).div_ceil(eh);
        (63 - rsz.leading_zeros()) as u8
    }
}

/* I/O */

pub struct EngineOutput {
    pub bestmove: String,
    pub ponder: Option<String>,  // if depth > 1
    pub score_type: String,      // cp/mate (score_to_uci)
    pub score_value: i16         // score (score_to_uci)
}

pub struct EvalOutput {
    pub score: i16,              // pure static score evaluation value
    pub q_score: i16,            // result of search extension (root_ext = false), use this
    pub qr_score: i16,           // result of search extension (root_ext = true)
    pub is_quiet: bool           // position considered quiet if: captures make it worse, not under check, game is not ended
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::hc_eval::HCEval;

    #[test]
    fn test_engine_relative_performance() {
        const EXPECTED_PERCENTAGE: u128 = 110;  // with working Null-Move Pruning it should be less than 100

        let mut board = Board::default();
        let mut avg1 = 0;
        let mut val = 0;  // force compiler to calc just in case
        for _ in 0..5 {
            let ts = Instant::now();
            let x = board.perft(4);
            avg1 += ts.elapsed().as_millis();
            val += x;
        }
        assert!(val > 1000);
        let mut engine = Search::init();
        let eval = HCEval::init(None);
        let mut avg2 = 0;
        let mut val2 = 0;  // force compiler to calc just in case
        for _ in 0..5 {
            let ts = Instant::now();
            let eo = engine.go(&eval, u64::MAX >> 1, 4, 0, None);
            avg2 += ts.elapsed().as_millis();
            val2 += eo.score_value;
            assert_ne!(eo.bestmove, "a1a1");
        }
        assert!(val2 >= 0);
        println!("{} {}", avg1, avg2);
        assert!(avg2 < avg1 * EXPECTED_PERCENTAGE / 100);
    }

    #[test]
    fn test_engine_search_mate_2() {
        let mut engine = Search::init();
        let eval = HCEval::init(None);
        engine.set_pos(Some("8/4p3/B1ppP3/1nPp4/Npk2N2/pR2p3/K3P3/3R4 w - - 0 1"));
        let res = engine.go(&eval, 400, 4, 50_000, None);
        assert_eq!(res.bestmove, "f4d3");
        assert_eq!(res.score_type, "mate");
        assert_eq!(res.score_value, 2);
    }

    #[test]
    fn test_engine_search_mate_3() {
        let mut engine = Search::init();
        let eval = HCEval::init(None);
        engine.set_pos(Some("4qrk1/p1r1Bppp/4b3/2p3Q1/8/3P4/PPP2PPP/R3R1K1 w - - 3 19"));
        let res = engine.go(&eval, 4_000, 6, 1_000_000, None);
        assert_eq!(res.bestmove, "e7f6");
        assert_eq!(res.score_type, "mate");
        assert_eq!(res.score_value, 3);
    }

    #[test]
    fn test_engine_search_mate_m2() {
        let mut engine = Search::init();
        let eval = HCEval::init(None);
        engine.set_pos(Some("4qrk1/p1r2ppp/4bB2/2p3Q1/8/3P4/PPP2PPP/R3R1K1 b - - 4 19"));
        let res = engine.go(&eval, 4_000, 6, 1_000_000, None);
        assert_eq!(res.bestmove, "g7g6");
        assert_eq!(res.score_type, "mate");
        assert_eq!(res.score_value, -2);
    }

    #[test]
    fn test_engine_search_mate_8() {
        let mut engine = Search::init();
        let eval = HCEval::init(None);
        engine.set_pos(Some("8/8/8/4n1p1/6P1/5B2/1kr5/4K3 b - - 0 68"));
        let res = engine.go(&eval, 48_000, 64, 12_000_000, None);
        assert_eq!(res.bestmove, "e5f3");
        assert_eq!(res.score_type, "mate");
        assert_eq!(res.score_value, 8);
    }

    #[test]
    fn test_engine_search_mate_m7() {
        let mut engine = Search::init();
        let eval = HCEval::init(None);
        engine.set_pos(Some("8/8/8/6p1/6P1/5n2/1kr5/4K3 w - - 0 69"));
        let res = engine.go(&eval, 48_000, 64, 12_000_000, None);
        assert_eq!(res.bestmove, "e1f1");
        assert_eq!(res.score_type, "mate");
        assert_eq!(res.score_value, -7);
    }

    #[test]
    fn test_engine_search_mate_6_shortcut() {
        let mut engine = Search::init();
        let eval = HCEval::init(None);
        engine.set_pos(Some("8/1p6/1R3Pk1/3K4/8/8/8/8 w - - 6 91"));
        let res = engine.go(&eval, 16_000, 64, 1_250_000, None);  // nl is low here
        assert!(res.bestmove == "d5e5" || res.bestmove == "d5e6");
        assert_eq!(res.score_type, "mate");
        assert!(res.score_value >= 6);  // it's fine if it sees it in 8 or something
    }

    #[test]
    #[ignore]  // Test this with --release, otherwise you'll die of age
    fn test_engine_search_mate_6() {
        let mut engine = Search::init();
        let eval = HCEval::init(None);
        engine.set_pos(Some("8/1p6/1R3Pk1/3K4/8/8/8/8 w - - 6 91"));
        let res = engine.go(&eval, 600_000, 64, 64_000_000, None);  // nl is low here as well
        assert!(res.bestmove == "d5e5" || res.bestmove == "d5e6");
        assert_eq!(res.score_type, "mate");
        assert_eq!(res.score_value, 6);
    }

    #[test]
    #[ignore]  // Test this with --release, otherwise you'll die of age
    fn test_engine_search_mate_7_sm() {
        let mut engine = Search::init();
        let eval = HCEval::init(None);
        engine.set_cache_size(96);
        engine.set_pos(Some("8/1p3k2/1R3P2/8/2K5/8/8/8 w - - 4 90"));
        let res = engine.go(&eval, 600_000, 64, 64_000_000, Some(&["c4d5"]));
        assert_eq!(res.bestmove, "c4d5");  // duh
        assert_eq!(res.score_type, "mate");
        assert_eq!(res.score_value, 7);
    }
}