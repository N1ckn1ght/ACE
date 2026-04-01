pub mod zobrist;
pub mod hc_weights;
pub mod hc_eval;
pub mod search;
pub mod clock;


#[cfg(test)]
mod tests {
    use crate::{engine::search::{Eval, GameResult, Search}, frame::util::{log, move_transform_back}};

    pub fn eval_wa_test<E: Eval>(eval: &E, depth: u8, nodes: u64) {
        let mut engine = Search::init();
        let mut res = engine.get_result();
        while res == GameResult::InProgress {
            let movstr = engine.go(eval, u32::MAX as u64, depth, nodes, false, None);
            let legals = engine.get_legal_moves();
            let mov = move_transform_back(&movstr.0, &legals, engine.get_turn());
            assert!(mov.is_some());
            engine.make_move(mov.unwrap());
            res = engine.get_result();
        }
        log(&format!("{:?}", res));
        assert_ne!(res, GameResult::BlackWon);
        // assert_eq!(res, GameResult::Draw);
    }
}