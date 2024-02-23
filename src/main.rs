 #![feature(test)]

extern crate test;

mod gen;
mod frame;
mod engine;

use std::io;
use crate::frame::util::{move_transform, move_transform_back};
use crate::gen::{leaping::init_leaping_attacks, magic::init_magics, secondary::init_secondary_maps};
use crate::frame::board::Board;
use crate::engine::chara::Chara;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    init_secondary_maps();

    let mut board = Board::default();
    // let mut board = Board::import("r1bqkb1r/ppp2ppp/2n5/3p4/2Bpn3/5N2/PPP2PPP/RNBQR1K1 w kq - 0 7");
    let mut board = Board::import("8/5k2/p1PR2N1/P6p/1P5P/8/5K2/8 w - - 1 54");
    // let mut board = Board::import("2Q5/7k/R5N1/P6p/1P5P/8/5K2/8 w - - 1 57");
    let mut chara = Chara::init(&mut board);

    println!("\n--- AKIRA HAS BEEN FULLY LOADED INTO MACHINE MEMORY ---\n");

    let mut hmc = 0;
    let scan = [true, true];
    let ab = [50, 1000];
    // soft limit lol, may overflow by about 1-50 msc5b
    let time = 800;
    let mut abi = 0;

    loop {
        let legals = chara.board.get_legal_moves();
        if legals.is_empty() {
            break;
        }

        if scan[hmc & 1] {
            println!("Processing...\n");
            let best_move = chara.think(ab[abi], time, 50);
            abi = 0;
            println!("Best move: {} ({})", move_transform(best_move.mov), best_move.score);
        }

        let mut mov;
        loop {
            let str = input();

            if str == "b" {
                chara.revert_move();
                hmc -= 2;
                break;
            } else if str == "r" {
                hmc -= 1;
                break;
            } else if str == "hr" {
                hmc -= 1;
                abi = 1;
                break;
            } else if str == "ex" {
                println!("{}", chara.board.export());
                continue;
            }

            mov = move_transform_back(&str.to_owned(), &legals);
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