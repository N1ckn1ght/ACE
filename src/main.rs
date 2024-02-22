 #![feature(test)]

extern crate test;

mod gen;
mod frame;
mod engine;

use std::io;
use crate::frame::util::{move_transform, move_transform_back, visualise};
use crate::gen::{leaping::init_leaping_attacks, magic::init_magics, secondary::init_secondary_maps};
use crate::frame::board::Board;
use crate::engine::chara::Chara;

fn main() {
    init_magics(&mut 1773);
    init_leaping_attacks();
    init_secondary_maps();

    let mut board = Board::import("5k1r/ppp2pq1/6np/1Q1p1b2/3P4/2P1R2P/PP3PPK/RNr5 b - - 1 22");
    let mut chara = Chara::init(&mut board);

    println!("\n--- AKIRA HAS BEEN FULLY LOADED INTO MACHINE MEMORY ---\n");

    loop {
        println!("Processing...\n");

        let legals = chara.board.get_legal_moves();
        if legals.is_empty() {
            break;
        }

        let best_move = chara.think(50, 2500);
        println!("Best move: {} ({})", move_transform(best_move.mov), best_move.score);

        let mut mov;
        loop {
            let str = input();
            mov = move_transform_back(&str.to_owned(), &legals);
            if let Some(i) = mov {
                chara.make_move(i);
                break;
            }
            println!("Move not found?");
        }
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