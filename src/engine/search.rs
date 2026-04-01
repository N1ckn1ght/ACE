use std::{cmp::{max, min}, collections::HashSet, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Instant};
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
    pub abort:			Arc<AtomicBool>,		// stop search signal
    searchmoves:        Vec<u32>,
    
    /* Cache for evaluated positions as leafs (eval() result) or branches (search result with given a/b) */
    cache:		        Vec<EvalHash>,
    cached_cnt:         u64,
    cache_size_bits:    usize,
    cache_mask:         u64,
    
    /* Cache for already made in board moves to track drawish positions */
    history_vec:		Vec<u64>,				// previous board hashes stored here to call more quick hash_iter() function
    history_set:		HashSet<u64>,			// for fast checking if this position had occured before in this line
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
    tpv:				[[u32; HALF_DEPTH_LIMIT]; HALF_DEPTH_LIMIT],
                                                // expected lines of moves length
    tpv_len:			[usize; HALF_DEPTH_LIMIT],
                                                // quiet moves that cause a beta cutoff
    killer:				[[u32; HALF_DEPTH_LIMIT]; 2],
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
        let cache_size_bits = 25;  // just an initial value
        let cache_mask = (1 << cache_size_bits) - 1;

        Self {
            board,
            cache:	            vec![EvalHash::default(); 1 << cache_size_bits],
            cached_cnt:         0,
            history_vec:	    cache_perm_vec,
            history_set:	    HashSet::default(),
            zobrist,
            ts:				    Instant::now(),
            tl:				    0,
            abort:			    Arc::new(AtomicBool::new(true)),
            nodes:			    0,
            nl:                 0,
            ply:			    0,
            tpv:			    [[0; HALF_DEPTH_LIMIT]; HALF_DEPTH_LIMIT],
            tpv_len:		    [0; HALF_DEPTH_LIMIT],
            killer:			    [[0; HALF_DEPTH_LIMIT]; 2],
            tpv_flag:		    false,
            mate_flag:		    false,
            cur_depth:          0,
            cache_size_bits,
            cache_mask,
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
        strict_search: bool,  // search for mate on given depth_target
        searchmoves: Option<&[&str]>
    ) -> (String, Option<String>) {
        self.ts = Instant::now();
        self.tl = time_limit_ms;
        self.nl = node_limit;
        self.abort.store(false, Ordering::Relaxed);
        self.nodes = 0;
        for line in self.tpv.iter_mut() { for node in line.iter_mut() { *node = 0 } };
        for len in self.tpv_len.iter_mut() { *len = 0 };
        for num in self.killer.iter_mut() { for mov in num.iter_mut() { *mov = 0 } };

        let mut alpha = -EVAL_INF;
        let mut beta  =  EVAL_INF;
        if strict_search {
            self.cur_depth = depth_target;
            self.mate_flag = true;
        } else {
            self.cur_depth = 1;
            self.mate_flag = false;
        }
        self.searchmoves = vec![];
        if searchmoves.is_some() {
            let plm = self.get_pseudo_legal_moves();  // assuming...
            self.searchmoves = vec![];
            for move_str in searchmoves.unwrap().iter() {
                self.searchmoves.push(move_transform_back(move_str, &plm, self.board.turn).unwrap());
            }
        }
        let mut k = 1;
        let mut score;
        let baw = 300;  // divide by 400 to get centipawns
        loop {
            self.tpv_flag = true;
            let temp = self.search(eval, alpha, beta, self.cur_depth);
            if self.abort.load(Ordering::Relaxed) {
                log(&format!("Abort signal reached! Nodecount: {}", self.nodes));
                break;
            } else {
                score = temp;
            }
            if self.tpv_len[0] != 0 {
                self.post(score);
            }
            if !(-EVAL_PRE_MATE..=EVAL_PRE_MATE).contains(&score) {
                if self.mate_flag {
                    break;
                }
                log("Mate detected.");
                alpha = -EVAL_INF;
                beta = EVAL_INF;
                self.mate_flag = true;
                continue;
            }
            if score <= alpha || score >= beta {
                if k > 15 {
                    alpha = -EVAL_INF;
                    beta = EVAL_INF;
                    log("Alpha/beta fail! Using INFINITE values now.");
                    continue;
                }
                k *= 2;
                alpha = alpha + baw * k - baw * (k * 2);
                beta = beta - baw * k + baw * (k * 2);
                log(&format!("Alpha/beta fail! Using x{} from base aspiration now.", k));
                continue;
            }

            // self.last_score = score_to_gui(score, false);
            alpha = score - baw;
            beta = score + baw;
            k = 1;
            self.cur_depth += 1;
            if self.cur_depth > depth_target || self.ts.elapsed().as_millis() as u64 > self.tl {
                break;
            }
            // TODO: don't exit search in ponder!!?
        }

        log(&format!("Approximate time spent: {} ms", self.ts.elapsed().as_millis() + 1));
        if self.tpv_len[0] > 1 {
            return (move_transform(self.tpv[0][0], self.board.turn), Some(move_transform(self.tpv[0][1], !self.board.turn)));
        }
        (move_transform(self.tpv[0][0], self.board.turn), None)
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

    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.cache.resize(1 << self.cache_size_bits, EvalHash::default());
        self.cached_cnt = 0;
    }

    pub fn make_move(&mut self, mov: u32) {
        let prev_hash = *self.history_vec.last().unwrap();
        self.history_set.insert(prev_hash);
        self.board.make_move(mov);
        let hash = self.zobrist.cache_iter(&self.board, mov, prev_hash);
        self.history_vec.push(hash);
    }

    pub fn undo_move(&mut self) {
        self.board.undo_move();
        self.history_vec.pop();
        self.history_set.remove(self.history_vec.last().unwrap());
    }

    fn update(&mut self) {
        if self.nl != 0 && self.nodes > self.nl {
            self.abort.store(true, Ordering::Relaxed);
            return;
        }
        if self.nodes & NODES_BETWEEN_UPDATES == 0 {
            if self.ts.elapsed().as_millis() as u64 > self.tl {
                self.abort.store(true, Ordering::Relaxed);
                return;
            }
            if self.nodes & POST_INTERVAL == 0 {
                self.post_regular();
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
        if !root_node && (self.board.hmc > 99 || self.history_set.contains(&hash)) {
            return 0;
        }

        // Exit search (tl, nl, listen)
        self.update();

        let hash_is_same = self.cache[hash_index].is_same(hash);
        // if not a "prove"-search
        if hash_is_same && !root_node && beta - alpha < 2 {
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
        if depth == 0 {
            return self.extension(eval, alpha, beta);
        }

        self.nodes += 1;

        if self.ply + 1 > HALF_DEPTH_LIMIT {
            return eval.eval(&self.board);
        }

        let in_check = self.board.is_in_check();
        // Null move prune
        if !in_check && !root_node && depth > 2 {
            self.ply += 1;
            self.board.turn = !self.board.turn;
            self.history_set.insert(*self.history_vec.last().unwrap());
            self.history_vec.push(*self.history_vec.last().unwrap() ^ self.zobrist.hash_turn ^ self.zobrist.hash_en_passant[self.board.en_passant]);
            let old_en_passant = self.board.en_passant;
            self.board.en_passant = 0;

            let score = -self.search(eval, -beta, -beta + 1, depth - 3);	// reduction = 2

            self.board.turn = !self.board.turn;
            self.board.en_passant = old_en_passant;
            self.history_vec.pop();
            self.history_set.remove(self.history_vec.last().unwrap());
            self.ply -= 1;

            if self.abort.load(Ordering::Relaxed) {
                return 0;
            }
            if score >= beta {
                return beta;
            }
        }

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
        moves.reverse();
        
        let mut hf_cur = HF_LOW;
        depth += in_check as u8;
        // a/b with lmr and pv proving
        for (i, mov) in moves.iter().enumerate() {
            self.make_move(*mov);
            self.ply += 1;
            let mut score = if i != 0 && depth > 2 && !(*mov > ME_PROMISING_MIN || in_check) {
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
            if alpha < -EVAL_PRE_MATE {
                self.cache[hash_index].score -= self.ply as i16;
            } else if alpha > EVAL_PRE_MATE {
                self.cache[hash_index].score += self.ply as i16;
            }
        }

        alpha // fail low
    }

    /// See only captures and check lines 'til the end
    /// 
    /// Evaluate all quiet leafs
    fn extension<E: Eval>(
        &mut self,
        eval: &E,
        mut alpha: i16,
        beta: i16
    ) -> i16 {
        // Exit search extension (tl, nl, listen)
        self.update();
        self.nodes += 1;

        // cuttin even before we get a list of moves
        alpha = max(alpha, eval.eval(&self.board));
        if alpha >= beta {
            return beta; // fail high
        }

        let mut moves = self.board.get_legal_moves();
        
        // if mate or stalemate
        if moves.is_empty() {
            if self.board.is_in_check() {
                return -EVAL_MATE + self.ply as i16;
            }
            return 0;
        }

        moves.sort();
        moves.reverse();

        for mov in moves.iter() {
            self.make_move(*mov);
            // extension will consider checks as well as captures
            if *mov < ME_CAPTURE_MIN && !self.board.is_in_check() {
                self.undo_move();
                continue;
            }
            self.ply += 1;
            alpha = max(alpha, -self.extension(eval, -beta, -alpha));
            self.ply -= 1;
            self.undo_move();
            if self.abort.load(Ordering::Relaxed) {
                return 0;
            }
            if alpha >= beta {
                return beta; // fail high
            }
        }

        alpha // fail low
    }

    fn post(&self, score: i16) {
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
                return ("mate".to_owned(), 1 + (EVAL_MATE + score) / 2);
            }
            return ("cp".to_owned(), score);
        }
        if score > EVAL_PRE_MATE {
            return ("mate".to_owned(), 1 + (EVAL_MATE - score) / 2);
        }
        ("cp".to_owned(), score)
    }

    fn calc_cache_size(megabytes: u32) -> u32 {

        0
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
}

/* CACHE related section */

const HF_PRECISE: u8 = 1;
const HF_LOW: u8 = 2;
const HF_HIGH: u8 = 4;

#[derive(Copy, Clone)]
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
}

impl Default for EvalHash {
    fn default() -> Self {
        Self {
            hash_upper_part: 0,
            hash_lower_part: 0,
            score: 0,
            depth: 0,
            flag: 0
        }
    }
}