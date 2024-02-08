use std::cmp::{min, max};
use crate::frame::{util::*, board::Board};
use crate::engine::chara::Chara;

const DEPTH_FALLBACK: i16 = 4;

pub fn search(chara: &mut Chara, board: &mut Board, mut alpha: f32, mut beta: f32, maximize: bool, depth: i16, 
    mut quiet_extension: bool, last_move: u64) -> Eval {

    // IF THE TARGET DEPTH IS REACHED
    if board.no >= depth {
        if move_get_capture(last_move) == E {
            if board.is_in_check() {
                quiet_extension = true;
            } else if !quiet_extension {
                return chara.eval(board);
            } else {
                quiet_extension = false;
                // failsafe
                if board.no >= depth + DEPTH_FALLBACK {
                    return chara.eval(board);
                }
            }
        }
    }

    // IF THE POSITION IS ARLEADY CACHED (and evaluated)
    let hash = *chara.cache_perm_vec.last().unwrap();
    if chara.cache.contains_key(&hash) {
        let mut eval = chara.cache[&hash];
        // There's a little bit suspicious draw check based on a floating point inaccuracy
        // TODO: fix depth issue
        // if correct_mate(&mut eval, board) || eval.depth <= board.no || (eval.mate == 0 && eval.score == 0.0) {
        if correct_mate(&mut eval, board) || (eval.mate == 0 && eval.score == 0.0) {
            return eval;
        }
    }

    // IF THE GAME HAS ENDED
    let moves = board.get_legal_moves();
    if moves.len() == 0 {
        if board.is_in_check() {
            let mut mate = 1;
            if !board.turn {
                mate = -1;
            }
            return Eval::new(0.0, mate, board.no);
        }
    }

    // ALPHA/BETA PRUNING
    let mut eval;

    if maximize {
        eval = Eval::new(0.0, -1, depth);
        for mov in moves.into_iter() {
            chara.make_move(board, mov);
            eval = max(eval, search(chara, board, alpha, beta, false, depth, quiet_extension, mov));
            chara.revert_move(board);
            alpha = max(alpha, eval);
            if alpha >= beta {
                break;
            }
        }
    } else {
        eval = Eval::new(0.0, 1, depth);
        for mov in moves.into_iter() {
            chara.make_move(board, mov);
            eval = min(eval, search(chara, board, alpha, beta, true, depth, quiet_extension, mov));
            chara.revert_move(board);
            beta = min(beta, eval);
            if beta <= alpha {
                break;
            }
        }
    }

    correct_mate(&mut eval, board);
    return eval;
}