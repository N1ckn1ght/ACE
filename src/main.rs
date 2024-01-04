#![feature(test)]

extern crate test;

mod util;
mod gen;
mod maps;
mod board;

use std::time::Instant;

use crate::board::Board;
use crate::gen::{magic::init_magics, leaping::init_leaping_attacks};
use crate::util::*;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();

    /* Used for testing, remove later */

    let mut board = Board::new();
    let now = Instant::now();
    let x = board.perft(6);
    println!("{}\t{}", x, now.elapsed().as_secs());

    // let moves = board.get_legal_moves();
    // for (tabs, mov) in moves.iter().enumerate() {
    //     if tabs % 5 == 0 {
    //         println!();
    //     }
    //     print!("{}\t", move_transform(*mov));
    // }
    // println!("\n");
    // visualise(&board.bbs, 12);
}