use std::cmp::max;
use crate::{engine::{hc_weights::HCWeights, search::Eval}, frame::{board::Board, util::*}};


const CENTER: [u64; 2] = [0b0000000000000000000110000001100000011000000000000000000000000000, 0b0000000000000000000000000001100000011000000110000000000000000000];
const STRONG: [u64; 2] = [0b0000000001111110011111100011110000000000000000000000000000000000, 0b0000000000000000000000000000000000111100011111100111111000000000];

impl Eval for HCEval {
    fn eval(&self, board: &Board) -> i32 {
        HCEval::eval(self, board)
    }
}

/// Provides eval(&Board) -> i32
/// 
/// Hand-crafted static evaluation
pub struct HCEval {
    w: HCWeights
}

impl Default for HCEval {
    fn default() -> Self {
        Self {
            w: HCWeights::init()
        }
    }
}

impl HCEval {
    /// Initialize using hand-crafted weights
    pub fn init(weights: Option<HCWeights>) -> Self {
        let w = match weights {
            Some(ws) => {
                ws
            },
            None => {
                HCWeights::init()
            }
        };
        Self {
            w
        }
    }

    /// Return static evaluation score on a given board
    pub fn eval(&self, board: &Board) -> i32 {
        let counter = (board.bbs[N] | board.bbs[N2]).count_ones() * 3 + 
            (board.bbs[B] | board.bbs[B2]).count_ones() * 3 + 
            (board.bbs[R] | board.bbs[R2]).count_ones() * 4 +
            (board.bbs[Q] | board.bbs[Q2]).count_ones() * 8;
        // 56 - full board, 30 - most likely, endgame?..

        if counter < 4 && board.bbs[P] | board.bbs[P2] == 0 {
            return 0;
        }

        let mut score: i32 = 0;
        let mut score_pd: [i32; 2] = [0, 0];
        // [18 - 56] range
        let phase_diff = f32::min((max(18, counter) - 18) as f32 * 0.0264, 1.0);

        let mut pattacks     = [0; 2];
        let mut mobility     = [0; 2];
        let mut pins         = [0; 2];
        let mut sof			 = [0; 2];
        let mut ppt          = [0; 2];
        let mut pass         = [0; 2];

        // quality of life fr
        let bptr = &board.bbs;
        let mptr = &board.maps;

        let sides = [board.get_occupancies(false), board.get_occupancies(true)];
        let occup = sides[0] | sides[1];
        let kbits = [gtz(bptr[K]), gtz(bptr[K2])];

        let rpin = [bptr[N] | bptr[B] | bptr[Q], bptr[N2] | bptr[B2] | bptr[Q2]]; // if attack is on Q, it's profitable (most likely will be detected by extension)
        let bpin = [bptr[N] | bptr[R] | bptr[Q], bptr[N2] | bptr[R2] | bptr[Q2]]; // if attack is on R/Q, it's profitable
        let rvic = [bptr[K] | bptr[Q],           bptr[K2] | bptr[Q2]];
        let bvic = [rvic[0] | bptr[R],           rvic[1]  | bptr[R2]];

        /* SCORE APPLICATION BEGINS */

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
                if bptr[P | ally] & mptr.flanks[sq] & mptr.ranks[sq] != 0 {
                    score += self.w.p_phalanga[ally];
                } else {
                    let mut flanks = 0;
                    if sq & 7 != 0 {
                        flanks += board.get_sliding_straight_opportunities(sq - 1, bptr[P] | bptr[P2]);
                    }
                    if sq & 7 != 7 {
                        flanks += board.get_sliding_straight_opportunities(sq + 1, bptr[P] | bptr[P2]);
                    }
                    if flanks & mptr.flanks[sq] & bptr[P | ally] == 0 {
                        score += self.w.p_isolated[ally];
                    }
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
                if mptr.steps_pawns[ally][sq] & bptr[P | enemy] != 0 {
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
                
                let opr = board.get_sliding_straight_opportunities(sq, occup) & board.get_sliding_diagonal_opportunities(sq, occup);
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

                let mut rook_pinned_to   = board.get_sliding_straight_attacks(sq, occup & !atk, sides[ally]) & !atk & bptr[K | enemy];
                let mut bishop_pinned_to = board.get_sliding_diagonal_attacks(sq, occup & !atk, sides[ally]) & !atk & bptr[K | enemy];
                while rook_pinned_to != 0 {
                    let csq = pop_bit(&mut rook_pinned_to);
                    pins[enemy] |= board.get_sliding_diagonal_path_unsafe(sq, csq) & rpin[enemy];
                }
                while bishop_pinned_to != 0 {
                    let csq = pop_bit(&mut bishop_pinned_to);
                    pins[enemy] |= board.get_sliding_diagonal_path_unsafe(sq, csq) & bpin[enemy];
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

                let opr = board.get_sliding_straight_opportunities(sq, occup);
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
                
                let mut pinned_to = board.get_sliding_straight_attacks(sq, occup & !atk, sides[ally]) & !atk & rvic[enemy];
                while pinned_to != 0 {
                    let csq = pop_bit(&mut pinned_to);
                    pins[enemy] |= board.get_sliding_diagonal_path_unsafe(sq, csq) & rpin[enemy];
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

                let opr = board.get_sliding_diagonal_opportunities(sq, occup);
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
                
                let mut pinned_to = board.get_sliding_diagonal_attacks(sq, occup & !atk, sides[ally]) & !atk & bvic[enemy];
                while pinned_to != 0 {
                    let csq = pop_bit(&mut pinned_to);
                    pins[enemy] |= board.get_sliding_diagonal_path_unsafe(sq, csq) & bpin[enemy];
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
                let mut atk = board.get_sliding_diagonal_attacks(sq, occup, sides[ally]) & pins[enemy];
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
                let mut atk = board.get_sliding_straight_attacks(sq, occup, sides[ally]) & pins[enemy];
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
                let mut atk = (board.get_sliding_diagonal_attacks(sq, occup, sides[ally]) | board.get_sliding_straight_attacks(sq, occup, sides[ally])) & pins[enemy];
                while atk != 0 {
                    pop_bit(&mut atk);
                    score += self.w.g_atk_pro_pinned[ally];
                }
            }
        }

        score_pd[0] += self.w.heatmap[0][K ][kbits[0]];
        score_pd[0] += self.w.heatmap[0][K2][kbits[1]];
        score_pd[1] += self.w.heatmap[1][K ][kbits[0]];
        score_pd[1] += self.w.heatmap[1][K2][kbits[1]];

        score_pd[0] += self.w.k_mobility_as_q[0][0] * (board.get_sliding_diagonal_attacks(kbits[0], occup, sides[0]) | board.get_sliding_straight_attacks(kbits[0], occup, sides[0])).count_ones() as i32;
        score_pd[0] += self.w.k_mobility_as_q[0][1] * (board.get_sliding_diagonal_attacks(kbits[1], occup, sides[1]) | board.get_sliding_straight_attacks(kbits[1], occup, sides[1])).count_ones() as i32;

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
        if bptr[P] | bptr[P2] != 0 && ((kbits[0] & 7) as i32 - (kbits[1] & 7) as i32).abs() + ((kbits[0] >> 3) as i32  - (kbits[1] >> 3) as i32).abs() == 2 {
            score_pd[0] += self.w.k_opposition[0][!board.turn as usize];
            score_pd[1] += self.w.k_opposition[1][!board.turn as usize];
        }
        if bptr[K] != 0 && bptr[Q] != 0 {
            score += self.w.s_qnight[0];
        }
        if bptr[K2] != 0 && bptr[Q2] != 0 {
            score += self.w.s_qnight[1];
        }
        if bptr[B] != 0 && (bptr[B] & (bptr[B] - 1)) != 0 {
            score += self.w.s_bishop_pair[0];
        }
        if bptr[B2] != 0 && (bptr[B2] & (bptr[B2] - 1)) != 0 {
            score += self.w.s_bishop_pair[1];
        }

        score += ((score_pd[0] as f32 * phase_diff) + (score_pd[1] as f32 * (1.0 - phase_diff))) as i32;
        score += self.w.s_mobility * (mobility[0].count_ones() as i32 - mobility[1].count_ones() as i32);

        if board.turn ^ (score > 0) {
            score += score / self.w.s_turn_div;
        } else {
            score -= score / self.w.s_turn_div;
        }
        score += self.w.s_turn[board.turn as usize];

        /* SCORE APPLICATION ENDS */

        if board.turn {
            return -score;
        }
        score
    }
}