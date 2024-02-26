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
    
    // test boards:
    // let mut board = Board::import("rn1qkbnr/ppp3pp/3p1p2/4N3/2B1P3/2N5/PPPP1PPP/R1BbK2R w KQkq - 0 6"); // mate in +2
    // let mut board = Board::import("r3k2r/8/8/8/8/8/8/4K3 b - - 0 1"); // mate in -2
    // let mut board = Board::import("8/8/5k2/8/3R4/2K5/8/8 w - - 0 1"); // mate in +??
    
    let mut board = Board::default();   
    let mut board = Board::import("r7/1pp4R/2kp4/4n2P/4p3/2P5/p1P2KP1/8 w - - 0 31");
    let mut chara = Chara::init(&mut board);

    chara.w.random_fact = 3;

    println!("\n--- AKIRA HAS BEEN FULLY LOADED INTO MACHINE MEMORY ---\n");

    let mut hmc = 0;
    let scan = [false, true];
    let ab = [88, 16384];
    // soft limit lol, may overflow by about 1-50 msc5b
    let time = 2950;
    let dl = 50;
    let mut abi = 0;

    loop {
        let legals = chara.board.get_legal_moves();
        if legals.is_empty() {
            break;
        }

        if scan[hmc & 1] {
            println!("Processing...\n");
            let best_move = chara.think(ab[abi], time, dl);
            abi = 0;
            println!();
            println!("Best move: {} ({})", move_transform(best_move.mov, chara.board.turn), score_transform(best_move.score, chara.board.turn));
            println!();
        }

        let mut mov;
        loop {
            println!();
            let str = input();

            if str == "b" {
                chara.revert_move();
                hmc -= 2;
                break;
            } 
            if str == "r" {
                hmc -= 1;
                break;
            } 
            if str == "hr" {
                hmc -= 1;
                abi = 1;
                break;
            } 
            if str == "ex" {
                println!("{}", chara.board.export());
                continue;
            }

            mov = move_transform_back(&str.to_owned(), &legals, chara.board.turn);
            if let Some(i) = mov {
                chara.make_move(i);
                break;
            }
            println!("Move not found?");
        }

        hmc += 1;
    }
}

fn input() -> String {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_goes_into_input_above) => {},
        Err(_no_updates_is_fine) => {},
    }
    input.trim().to_string()
}

fn memtest() {
    let mut board = Board::default();
    let mut chara = Chara::init(&mut board);

    //hypertest
    let mut rng = rand::thread_rng();
    while chara.cache_branches.len() < CACHED_BRANCHES_LIMIT {
        chara.cache_branches.insert(rng.gen::<u64>(), EvalBr::new(rng.gen::<i32>(), rng.gen::<i16>(), rng.gen::<i16>()));
    }
    while chara.cache_leaves.len() < CACHED_LEAVES_LIMIT {
        chara.cache_leaves.insert(rng.gen::<u64>(), rng.gen::<i32>());
    }
}