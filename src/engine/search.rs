use std::cmp::max;
use crate::frame::{util::*, board::Board};
use crate::engine::chara::Chara;

pub fn search(
    chara: &mut Chara,
    board: &mut Board,
    mut alpha: f32,
    mut beta: f32,
    depth: i16
) -> EvalBr {

    /* SEARCH CONDITION */

    if depth == 0 {
        return extension(chara, board, alpha, beta, true);
    }

    /* ALREADY CACHED POSITION CHECK */

    let hash = *chara.history_vec.last().unwrap();
    if chara.cache_branches.contains_key(&hash) {
        let mut eval = chara.cache_branches[&hash];
        if depth <= eval.depth {
            return eval;
        }
    }

    let mut moves = board.get_legal_moves();

    /* GAME END CHECK */

    if moves.len() == 0 {
        if board.is_in_check() {
            let mut score = f32::MIN;
            if board.turn {
                score = f32::MAX;
            }
            return EvalBr::new(score, 0);
        }
    }

    /* PRE-SORTING (captures first) */

    moves.sort();
    moves.reverse();

    /* ALPHA/BETA PRUNING */

    let mut eval = EvalBr::new(f32::MIN, 0);

    for mov in moves.into_iter() {
        chara.make_move(board, mov);
        eval = max(eval, -search(chara, board, -beta, -alpha, depth - 1));
        chara.revert_move(board);
        alpha = f32::max(alpha, eval.score);
        if alpha >= beta {
            break;
        }
    }

    eval.depth += 1;
    chara.cache_branches.insert(hash, eval);
    return eval;
}

pub fn extension(
    chara: &mut Chara,
    board: &mut Board,
    mut alpha: f32,
    mut beta: f32,
    capture: bool
) -> EvalBr {
    
    /* EXTENSION CONDITION */

    if !(capture || board.is_in_check()) {
        return chara.eval(board);
    }

    /* ALREADY CACHED POSITION CHECK */

    let hash = *chara.history_vec.last().unwrap();
    if chara.cache_branches.contains_key(&hash) {
        return chara.cache_branches[&hash];
    }

    let mut moves = board.get_legal_moves();

    /* GAME END CHECK */

    if moves.len() == 0 {
        if board.is_in_check() {
            let mut score = f32::MIN;
            if board.turn {
                score = f32::MAX;
            }
            return EvalBr::new(score, 0);
        }
    }

    /* PRE-SORTING (captures first) */

    moves.sort();
    moves.reverse();

    /* ALPHA/BETA PRUNING */

    let mut eval = chara.eval(board);

    for mov in moves.into_iter() {
        alpha = f32::max(alpha, eval.score);
        if alpha >= beta {
            break;
        }
        chara.make_move(board, mov);
        eval = max(eval, -extension(chara, board, -beta, -alpha, mov < CAPTURE_MINIMUM));
        chara.revert_move(board);
    }

    eval.is_extent = true;
    chara.cache_branches.insert(hash, eval);
    eval
}