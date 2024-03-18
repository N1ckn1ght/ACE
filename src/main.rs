 #![feature(test)]

extern crate test;

mod gen;
mod frame;
mod engine;

use std::io;
use crate::frame::util::*;
use crate::gen::{leaping::init_leaping_attacks, magic::init_magics, secondary::init_secondary_maps};
use crate::frame::board::Board;
use crate::engine::chara::Chara;

use rand::Rng;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    init_secondary_maps();

    // memtest();

    // comms!
}

// fn memtest() {
//     let mut board = Board::default();
//     let mut chara = Chara::init(&mut board);

//     //hypertest
//     let mut rng = rand::thread_rng();
//     while chara.cache_branches.len() < CACHED_BRANCHES_LIMIT {
//         chara.cache_branches.insert(rng.gen::<u64>(), EvalBr::new(rng.gen::<i32>(), rng.gen::<i16>(), rng.gen::<i16>()));
//     }
//     while chara.cache_leaves.len() < CACHED_LEAVES_LIMIT {
//         chara.cache_leaves.insert(rng.gen::<u64>(), rng.gen::<i32>());
//     }
// }